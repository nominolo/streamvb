pub fn encode_simd(input: &[u32]) -> (usize, Vec<u8>) {
    let items = input.len();
    if items == 0 {
        return (0, Vec::new());
    }

    let mut output: Vec<u8> = Vec::with_capacity(max_compressed_len(items));

    // This always points to where the currently collected control byte needs
    // to be written.
    let controls: *mut u8 = output.as_mut_ptr();
    let data: *mut u8 = unsafe { controls.add(control_bytes_len(items)) };
    let input: *const u32 = input.as_ptr();

    unsafe {
        let data = encode_worker(items, input, controls, data);
        let len = data.offset_from(output.as_ptr()) as usize;
        debug_assert!(len <= output.capacity());
        output.set_len(len)
    };

    (items, output)
}

use std::arch::aarch64::{
    uint32x2_t, uint32x4_t, uint8x16_t, uint8x8_t, vclzq_u32, vdupq_n_u32, vld1_u32, vld1_u8,
    vld1q_u32, vld1q_u8, vmul_u32, vqsubq_u32, vqtbl1_u8, vqtbl1q_u8, vreinterpret_u32_u8,
    vreinterpretq_u8_u32, vshrq_n_u32, vst1_u32, vst1q_u8,
};

use crate::common::{control_bytes_len, max_compressed_len};

static GATHER_LO: [u8; 8] = [12, 8, 4, 0, 12, 8, 4, 0];

static AGGREGATORS: [u32; 2] = [
    1 | 1 << 10 | 1 << 20 | 1 << 30, // concat
    1 | 1 << 8 | 1 << 16 | 1 << 24,  // sum
];

// based on https://github.com/lemire/streamvbyte/blob/master/src/streamvbyte_arm_encode.c
unsafe fn encode_worker(
    items: usize,
    mut input: *const u32,
    mut controls: *mut u8,
    mut out: *mut u8,
) -> *mut u8 {
    let gatherlo: uint8x8_t = vld1_u8(&GATHER_LO as *const u8);
    let aggregators: uint32x2_t = vld1_u32(&AGGREGATORS as *const u32);

    let end: *const u32 = input.add(items & !3);
    while input != end {
        let data: uint32x4_t = vld1q_u32(input);
        // Ex: [11, 3322, 77665544, aa9988]

        // clz = count leading zero bits
        // lane code is 3 - (saturating sub) (clz(data)/8)
        let clzbytes: uint32x4_t = vshrq_n_u32::<3>(vclzq_u32(data));
        // Ex: [3, 2, 0, 1]
        let lanecodes: uint32x4_t = vqsubq_u32(vdupq_n_u32(3), clzbytes);
        // Ex: [0, 1, 3, 2]
        let lanebytes: uint8x16_t = vreinterpretq_u8_u32(lanecodes);
        // [00000000_01000000_03000000_02000000]
        let lobytes: uint8x8_t = vqtbl1_u8(lanebytes, gatherlo);
        // [2, 3, 1, 0, 2, 3, 1, 0]
        let mulshift: uint32x2_t = vreinterpret_u32_u8(lobytes);
        // [00010302, 00010302]
        let mut code_and_length: [u32; 2] = [0, 0];
        vst1_u32(
            &mut code_and_length as *mut u32,
            vmul_u32(mulshift, aggregators),
        );
        // [b42d0b02, 06060502]
        // b4  ==  10_11_01_00
        let code: u32 = code_and_length[0] >> 24;
        let len = 4 + (code_and_length[1] >> 24);
        let databytes: uint8x16_t = vreinterpretq_u8_u32(data);
        let shuffle: uint8x16_t =
            vld1q_u8(ENCODE_SHUFFLE_TABLE[code as usize].as_ptr() as *const u8);
        vst1q_u8(out, vqtbl1q_u8(databytes, shuffle));

        // println!("{:x?}", clzbytes);
        // println!("{:x?}", lanecodes);
        // println!("{:x?}", lanebytes);
        // println!("{:x?}", lobytes);
        // println!("{:x?}", mulshift);
        // println!("{:x?}", code_and_length);
        // println!("{:x?}", code_and_length);
        // println!("{:x?}", databytes);
        // println!("{:x?}", shuffle);

        // for i in 0..len {
        //     println!("{}: {:x?}", i, *out.add(i as usize));
        // }

        *controls = code as u8;
        controls = controls.add(1);
        input = input.add(4);
        out = out.add(len as usize);
    }
    if items & 3 > 0 {
        let mut key: u32 = 0;
        // handle the rest
        for i in 0..items & 3 {
            let word = *input;
            let symbol = encode_one(word);
            key |= symbol << (i + i);
            std::ptr::copy_nonoverlapping(input as *const u8, out, 4);
            input = input.add(1);
            out = out.add(symbol as usize + 1);
        }
        *controls = key as u8;
    }
    out
}

fn encode_one(word: u32) -> u32 {
    let t1 = (word > 0x000000ff) as u32;
    let t2 = (word > 0x0000ffff) as u32;
    let t3 = (word > 0x00ffffff) as u32;
    t1 + t2 + t3
}

#[rustfmt::skip]
static ENCODE_SHUFFLE_TABLE: [[u8; 16]; 256] = [
    [   0,    4,    8,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1111
    [   0,    1,    4,    8,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2111
    [   0,    1,    2,    4,    8,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3111
    [   0,    1,    2,    3,    4,    8,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4111
    [   0,    4,    5,    8,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1211
    [   0,    1,    4,    5,    8,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2211
    [   0,    1,    2,    4,    5,    8,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3211
    [   0,    1,    2,    3,    4,    5,    8,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4211
    [   0,    4,    5,    6,    8,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1311
    [   0,    1,    4,    5,    6,    8,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2311
    [   0,    1,    2,    4,    5,    6,    8,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3311
    [   0,    1,    2,    3,    4,    5,    6,    8,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4311
    [   0,    4,    5,    6,    7,    8,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1411
    [   0,    1,    4,    5,    6,    7,    8,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2411
    [   0,    1,    2,    4,    5,    6,    7,    8,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3411
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4411
    [   0,    4,    8,    9,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1121
    [   0,    1,    4,    8,    9,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2121
    [   0,    1,    2,    4,    8,    9,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3121
    [   0,    1,    2,    3,    4,    8,    9,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4121
    [   0,    4,    5,    8,    9,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1221
    [   0,    1,    4,    5,    8,    9,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2221
    [   0,    1,    2,    4,    5,    8,    9,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3221
    [   0,    1,    2,    3,    4,    5,    8,    9,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4221
    [   0,    4,    5,    6,    8,    9,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1321
    [   0,    1,    4,    5,    6,    8,    9,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2321
    [   0,    1,    2,    4,    5,    6,    8,    9,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3321
    [   0,    1,    2,    3,    4,    5,    6,    8,    9,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4321
    [   0,    4,    5,    6,    7,    8,    9,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1421
    [   0,    1,    4,    5,    6,    7,    8,    9,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2421
    [   0,    1,    2,    4,    5,    6,    7,    8,    9,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3421
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   12, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4421
    [   0,    4,    8,    9,   10,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1131
    [   0,    1,    4,    8,    9,   10,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2131
    [   0,    1,    2,    4,    8,    9,   10,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3131
    [   0,    1,    2,    3,    4,    8,    9,   10,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4131
    [   0,    4,    5,    8,    9,   10,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1231
    [   0,    1,    4,    5,    8,    9,   10,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2231
    [   0,    1,    2,    4,    5,    8,    9,   10,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3231
    [   0,    1,    2,    3,    4,    5,    8,    9,   10,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4231
    [   0,    4,    5,    6,    8,    9,   10,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1331
    [   0,    1,    4,    5,    6,    8,    9,   10,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2331
    [   0,    1,    2,    4,    5,    6,    8,    9,   10,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3331
    [   0,    1,    2,    3,    4,    5,    6,    8,    9,   10,   12, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4331
    [   0,    4,    5,    6,    7,    8,    9,   10,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1431
    [   0,    1,    4,    5,    6,    7,    8,    9,   10,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2431
    [   0,    1,    2,    4,    5,    6,    7,    8,    9,   10,   12, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3431
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10,   12, 0xff, 0xff, 0xff, 0xff, ],  // 4431
    [   0,    4,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1141
    [   0,    1,    4,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2141
    [   0,    1,    2,    4,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3141
    [   0,    1,    2,    3,    4,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4141
    [   0,    4,    5,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1241
    [   0,    1,    4,    5,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2241
    [   0,    1,    2,    4,    5,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3241
    [   0,    1,    2,    3,    4,    5,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4241
    [   0,    4,    5,    6,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1341
    [   0,    1,    4,    5,    6,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2341
    [   0,    1,    2,    4,    5,    6,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3341
    [   0,    1,    2,    3,    4,    5,    6,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, 0xff, ],  // 4341
    [   0,    4,    5,    6,    7,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1441
    [   0,    1,    4,    5,    6,    7,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2441
    [   0,    1,    2,    4,    5,    6,    7,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, 0xff, ],  // 3441
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, ],  // 4441
    [   0,    4,    8,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1112
    [   0,    1,    4,    8,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2112
    [   0,    1,    2,    4,    8,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3112
    [   0,    1,    2,    3,    4,    8,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4112
    [   0,    4,    5,    8,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1212
    [   0,    1,    4,    5,    8,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2212
    [   0,    1,    2,    4,    5,    8,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3212
    [   0,    1,    2,    3,    4,    5,    8,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4212
    [   0,    4,    5,    6,    8,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1312
    [   0,    1,    4,    5,    6,    8,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2312
    [   0,    1,    2,    4,    5,    6,    8,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3312
    [   0,    1,    2,    3,    4,    5,    6,    8,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4312
    [   0,    4,    5,    6,    7,    8,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1412
    [   0,    1,    4,    5,    6,    7,    8,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2412
    [   0,    1,    2,    4,    5,    6,    7,    8,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3412
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4412
    [   0,    4,    8,    9,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1122
    [   0,    1,    4,    8,    9,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2122
    [   0,    1,    2,    4,    8,    9,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3122
    [   0,    1,    2,    3,    4,    8,    9,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4122
    [   0,    4,    5,    8,    9,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1222
    [   0,    1,    4,    5,    8,    9,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2222
    [   0,    1,    2,    4,    5,    8,    9,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3222
    [   0,    1,    2,    3,    4,    5,    8,    9,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4222
    [   0,    4,    5,    6,    8,    9,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1322
    [   0,    1,    4,    5,    6,    8,    9,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2322
    [   0,    1,    2,    4,    5,    6,    8,    9,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3322
    [   0,    1,    2,    3,    4,    5,    6,    8,    9,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4322
    [   0,    4,    5,    6,    7,    8,    9,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1422
    [   0,    1,    4,    5,    6,    7,    8,    9,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2422
    [   0,    1,    2,    4,    5,    6,    7,    8,    9,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3422
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   12,   13, 0xff, 0xff, 0xff, 0xff, ],  // 4422
    [   0,    4,    8,    9,   10,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1132
    [   0,    1,    4,    8,    9,   10,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2132
    [   0,    1,    2,    4,    8,    9,   10,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3132
    [   0,    1,    2,    3,    4,    8,    9,   10,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4132
    [   0,    4,    5,    8,    9,   10,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1232
    [   0,    1,    4,    5,    8,    9,   10,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2232
    [   0,    1,    2,    4,    5,    8,    9,   10,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3232
    [   0,    1,    2,    3,    4,    5,    8,    9,   10,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4232
    [   0,    4,    5,    6,    8,    9,   10,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1332
    [   0,    1,    4,    5,    6,    8,    9,   10,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2332
    [   0,    1,    2,    4,    5,    6,    8,    9,   10,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3332
    [   0,    1,    2,    3,    4,    5,    6,    8,    9,   10,   12,   13, 0xff, 0xff, 0xff, 0xff, ],  // 4332
    [   0,    4,    5,    6,    7,    8,    9,   10,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1432
    [   0,    1,    4,    5,    6,    7,    8,    9,   10,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2432
    [   0,    1,    2,    4,    5,    6,    7,    8,    9,   10,   12,   13, 0xff, 0xff, 0xff, 0xff, ],  // 3432
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10,   12,   13, 0xff, 0xff, 0xff, ],  // 4432
    [   0,    4,    8,    9,   10,   11,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1142
    [   0,    1,    4,    8,    9,   10,   11,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2142
    [   0,    1,    2,    4,    8,    9,   10,   11,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3142
    [   0,    1,    2,    3,    4,    8,    9,   10,   11,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4142
    [   0,    4,    5,    8,    9,   10,   11,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1242
    [   0,    1,    4,    5,    8,    9,   10,   11,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2242
    [   0,    1,    2,    4,    5,    8,    9,   10,   11,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3242
    [   0,    1,    2,    3,    4,    5,    8,    9,   10,   11,   12,   13, 0xff, 0xff, 0xff, 0xff, ],  // 4242
    [   0,    4,    5,    6,    8,    9,   10,   11,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1342
    [   0,    1,    4,    5,    6,    8,    9,   10,   11,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2342
    [   0,    1,    2,    4,    5,    6,    8,    9,   10,   11,   12,   13, 0xff, 0xff, 0xff, 0xff, ],  // 3342
    [   0,    1,    2,    3,    4,    5,    6,    8,    9,   10,   11,   12,   13, 0xff, 0xff, 0xff, ],  // 4342
    [   0,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1442
    [   0,    1,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13, 0xff, 0xff, 0xff, 0xff, ],  // 2442
    [   0,    1,    2,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13, 0xff, 0xff, 0xff, ],  // 3442
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13, 0xff, 0xff, ],  // 4442
    [   0,    4,    8,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1113
    [   0,    1,    4,    8,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2113
    [   0,    1,    2,    4,    8,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3113
    [   0,    1,    2,    3,    4,    8,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4113
    [   0,    4,    5,    8,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1213
    [   0,    1,    4,    5,    8,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2213
    [   0,    1,    2,    4,    5,    8,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3213
    [   0,    1,    2,    3,    4,    5,    8,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4213
    [   0,    4,    5,    6,    8,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1313
    [   0,    1,    4,    5,    6,    8,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2313
    [   0,    1,    2,    4,    5,    6,    8,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3313
    [   0,    1,    2,    3,    4,    5,    6,    8,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4313
    [   0,    4,    5,    6,    7,    8,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1413
    [   0,    1,    4,    5,    6,    7,    8,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2413
    [   0,    1,    2,    4,    5,    6,    7,    8,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3413
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, ],  // 4413
    [   0,    4,    8,    9,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1123
    [   0,    1,    4,    8,    9,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2123
    [   0,    1,    2,    4,    8,    9,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3123
    [   0,    1,    2,    3,    4,    8,    9,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4123
    [   0,    4,    5,    8,    9,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1223
    [   0,    1,    4,    5,    8,    9,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2223
    [   0,    1,    2,    4,    5,    8,    9,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3223
    [   0,    1,    2,    3,    4,    5,    8,    9,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4223
    [   0,    4,    5,    6,    8,    9,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1323
    [   0,    1,    4,    5,    6,    8,    9,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2323
    [   0,    1,    2,    4,    5,    6,    8,    9,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3323
    [   0,    1,    2,    3,    4,    5,    6,    8,    9,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, ],  // 4323
    [   0,    4,    5,    6,    7,    8,    9,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1423
    [   0,    1,    4,    5,    6,    7,    8,    9,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2423
    [   0,    1,    2,    4,    5,    6,    7,    8,    9,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, ],  // 3423
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   12,   13,   14, 0xff, 0xff, 0xff, ],  // 4423
    [   0,    4,    8,    9,   10,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1133
    [   0,    1,    4,    8,    9,   10,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2133
    [   0,    1,    2,    4,    8,    9,   10,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3133
    [   0,    1,    2,    3,    4,    8,    9,   10,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4133
    [   0,    4,    5,    8,    9,   10,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1233
    [   0,    1,    4,    5,    8,    9,   10,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2233
    [   0,    1,    2,    4,    5,    8,    9,   10,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3233
    [   0,    1,    2,    3,    4,    5,    8,    9,   10,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, ],  // 4233
    [   0,    4,    5,    6,    8,    9,   10,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1333
    [   0,    1,    4,    5,    6,    8,    9,   10,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2333
    [   0,    1,    2,    4,    5,    6,    8,    9,   10,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, ],  // 3333
    [   0,    1,    2,    3,    4,    5,    6,    8,    9,   10,   12,   13,   14, 0xff, 0xff, 0xff, ],  // 4333
    [   0,    4,    5,    6,    7,    8,    9,   10,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1433
    [   0,    1,    4,    5,    6,    7,    8,    9,   10,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, ],  // 2433
    [   0,    1,    2,    4,    5,    6,    7,    8,    9,   10,   12,   13,   14, 0xff, 0xff, 0xff, ],  // 3433
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10,   12,   13,   14, 0xff, 0xff, ],  // 4433
    [   0,    4,    8,    9,   10,   11,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1143
    [   0,    1,    4,    8,    9,   10,   11,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2143
    [   0,    1,    2,    4,    8,    9,   10,   11,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3143
    [   0,    1,    2,    3,    4,    8,    9,   10,   11,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, ],  // 4143
    [   0,    4,    5,    8,    9,   10,   11,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1243
    [   0,    1,    4,    5,    8,    9,   10,   11,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2243
    [   0,    1,    2,    4,    5,    8,    9,   10,   11,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, ],  // 3243
    [   0,    1,    2,    3,    4,    5,    8,    9,   10,   11,   12,   13,   14, 0xff, 0xff, 0xff, ],  // 4243
    [   0,    4,    5,    6,    8,    9,   10,   11,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1343
    [   0,    1,    4,    5,    6,    8,    9,   10,   11,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, ],  // 2343
    [   0,    1,    2,    4,    5,    6,    8,    9,   10,   11,   12,   13,   14, 0xff, 0xff, 0xff, ],  // 3343
    [   0,    1,    2,    3,    4,    5,    6,    8,    9,   10,   11,   12,   13,   14, 0xff, 0xff, ],  // 4343
    [   0,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13,   14, 0xff, 0xff, 0xff, 0xff, ],  // 1443
    [   0,    1,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13,   14, 0xff, 0xff, 0xff, ],  // 2443
    [   0,    1,    2,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13,   14, 0xff, 0xff, ],  // 3443
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13,   14, 0xff, ],  // 4443
    [   0,    4,    8,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1114
    [   0,    1,    4,    8,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2114
    [   0,    1,    2,    4,    8,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3114
    [   0,    1,    2,    3,    4,    8,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4114
    [   0,    4,    5,    8,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1214
    [   0,    1,    4,    5,    8,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2214
    [   0,    1,    2,    4,    5,    8,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3214
    [   0,    1,    2,    3,    4,    5,    8,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4214
    [   0,    4,    5,    6,    8,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1314
    [   0,    1,    4,    5,    6,    8,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2314
    [   0,    1,    2,    4,    5,    6,    8,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3314
    [   0,    1,    2,    3,    4,    5,    6,    8,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, ],  // 4314
    [   0,    4,    5,    6,    7,    8,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1414
    [   0,    1,    4,    5,    6,    7,    8,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2414
    [   0,    1,    2,    4,    5,    6,    7,    8,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, ],  // 3414
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,   12,   13,   14,   15, 0xff, 0xff, 0xff, ],  // 4414
    [   0,    4,    8,    9,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1124
    [   0,    1,    4,    8,    9,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2124
    [   0,    1,    2,    4,    8,    9,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3124
    [   0,    1,    2,    3,    4,    8,    9,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 4124
    [   0,    4,    5,    8,    9,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1224
    [   0,    1,    4,    5,    8,    9,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2224
    [   0,    1,    2,    4,    5,    8,    9,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3224
    [   0,    1,    2,    3,    4,    5,    8,    9,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, ],  // 4224
    [   0,    4,    5,    6,    8,    9,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1324
    [   0,    1,    4,    5,    6,    8,    9,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2324
    [   0,    1,    2,    4,    5,    6,    8,    9,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, ],  // 3324
    [   0,    1,    2,    3,    4,    5,    6,    8,    9,   12,   13,   14,   15, 0xff, 0xff, 0xff, ],  // 4324
    [   0,    4,    5,    6,    7,    8,    9,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1424
    [   0,    1,    4,    5,    6,    7,    8,    9,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, ],  // 2424
    [   0,    1,    2,    4,    5,    6,    7,    8,    9,   12,   13,   14,   15, 0xff, 0xff, 0xff, ],  // 3424
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   12,   13,   14,   15, 0xff, 0xff, ],  // 4424
    [   0,    4,    8,    9,   10,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1134
    [   0,    1,    4,    8,    9,   10,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2134
    [   0,    1,    2,    4,    8,    9,   10,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 3134
    [   0,    1,    2,    3,    4,    8,    9,   10,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, ],  // 4134
    [   0,    4,    5,    8,    9,   10,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1234
    [   0,    1,    4,    5,    8,    9,   10,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2234
    [   0,    1,    2,    4,    5,    8,    9,   10,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, ],  // 3234
    [   0,    1,    2,    3,    4,    5,    8,    9,   10,   12,   13,   14,   15, 0xff, 0xff, 0xff, ],  // 4234
    [   0,    4,    5,    6,    8,    9,   10,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1334
    [   0,    1,    4,    5,    6,    8,    9,   10,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, ],  // 2334
    [   0,    1,    2,    4,    5,    6,    8,    9,   10,   12,   13,   14,   15, 0xff, 0xff, 0xff, ],  // 3334
    [   0,    1,    2,    3,    4,    5,    6,    8,    9,   10,   12,   13,   14,   15, 0xff, 0xff, ],  // 4334
    [   0,    4,    5,    6,    7,    8,    9,   10,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, ],  // 1434
    [   0,    1,    4,    5,    6,    7,    8,    9,   10,   12,   13,   14,   15, 0xff, 0xff, 0xff, ],  // 2434
    [   0,    1,    2,    4,    5,    6,    7,    8,    9,   10,   12,   13,   14,   15, 0xff, 0xff, ],  // 3434
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10,   12,   13,   14,   15, 0xff, ],  // 4434
    [   0,    4,    8,    9,   10,   11,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1144
    [   0,    1,    4,    8,    9,   10,   11,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 2144
    [   0,    1,    2,    4,    8,    9,   10,   11,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, ],  // 3144
    [   0,    1,    2,    3,    4,    8,    9,   10,   11,   12,   13,   14,   15, 0xff, 0xff, 0xff, ],  // 4144
    [   0,    4,    5,    8,    9,   10,   11,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, 0xff, ],  // 1244
    [   0,    1,    4,    5,    8,    9,   10,   11,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, ],  // 2244
    [   0,    1,    2,    4,    5,    8,    9,   10,   11,   12,   13,   14,   15, 0xff, 0xff, 0xff, ],  // 3244
    [   0,    1,    2,    3,    4,    5,    8,    9,   10,   11,   12,   13,   14,   15, 0xff, 0xff, ],  // 4244
    [   0,    4,    5,    6,    8,    9,   10,   11,   12,   13,   14,   15, 0xff, 0xff, 0xff, 0xff, ],  // 1344
    [   0,    1,    4,    5,    6,    8,    9,   10,   11,   12,   13,   14,   15, 0xff, 0xff, 0xff, ],  // 2344
    [   0,    1,    2,    4,    5,    6,    8,    9,   10,   11,   12,   13,   14,   15, 0xff, 0xff, ],  // 3344
    [   0,    1,    2,    3,    4,    5,    6,    8,    9,   10,   11,   12,   13,   14,   15, 0xff, ],  // 4344
    [   0,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13,   14,   15, 0xff, 0xff, 0xff, ],  // 1444
    [   0,    1,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13,   14,   15, 0xff, 0xff, ],  // 2444
    [   0,    1,    2,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13,   14,   15, 0xff, ],  // 3444
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13,   14,   15, ],  // 4444
];

#[test]
fn generate_encoding_shuffle_table() {
    println!("#[rustfmt::skip]");
    println!("static ENCODE_SHUFFLE_TABLE: [[u8; 16]; 256] = [");
    for b0 in 1..5 {
        for b1 in 1..5 {
            for b2 in 1..5 {
                for b3 in 1..5 {
                    let mut shuf = vec![];
                    let mut src_ofs = 0;
                    #[allow(clippy::needless_range_loop)]
                    for _ in 0..b3 {
                        shuf.push(src_ofs);
                        src_ofs += 1;
                    }
                    src_ofs = 4;
                    for _ in 0..b2 {
                        shuf.push(src_ofs);
                        src_ofs += 1;
                    }
                    src_ofs = 8;
                    for _ in 0..b1 {
                        shuf.push(src_ofs);
                        src_ofs += 1;
                    }
                    src_ofs = 12;
                    for _ in 0..b0 {
                        shuf.push(src_ofs);
                        src_ofs += 1;
                    }
                    while shuf.len() < 16 {
                        shuf.push(0xff);
                    }
                    print!("    [");
                    for b in shuf {
                        if b < 0x80 {
                            print!("{:4}, ", b);
                        } else {
                            print!("0xff, ");
                        }
                    }
                    println!("],  // {}{}{}{}", b3, b2, b1, b0);
                }
            }
        }
    }
    println!["];"]
}

#[cfg(test)]
mod tests {
    #[test]
    fn encode_step() {
        let values = vec![
            0x11, 0x3322, 0x77665544, 0xaa9988, 0x2010, 0x504030, 0x90000060, 0xa0, 0x70, 0x8000,
        ];
        let (len, encoded) = super::encode_simd(&values);
        println!("len={}, encoded: {:x?}", len, encoded);
    }

    use rand::Rng;

    pub fn random_any_bit(count: usize) -> Vec<u32> {
        let mut input: Vec<u32> = Vec::with_capacity(count);
        let mut rng = rand::thread_rng();
        for _ in 0..count {
            let sz = rng.gen_range(1..5);
            let b = match sz {
                1 => rng.gen::<u8>() as u32,
                2 => rng.gen::<u16>() as u32,
                3 => rng.gen_range(0u32..16777216),
                4 => rng.gen::<u32>(),
                _ => panic!("impossible"),
            };
            input.push(b);
        }
        input
    }

    #[test]
    fn encode_random() {
        for n in 0..100 {
            let count = 1000 + n;
            let input = random_any_bit(count);

            let (len, encoded) = super::encode_simd(&input);
            assert_eq!(len, input.len());

            let decoded = crate::scalar::decode::decode(len, &encoded).unwrap();
            assert_eq!(&input, &decoded);
        }
    }
}

use crate::{
    common::{control_bytes_len, max_compressed_len},
    tables::len::LENGTH_TABLE,
};

pub(crate) trait Encoder {
    #[cfg(target_feature = "sse2")]
    unsafe fn simd_encode_4x32(data: __m128i) -> __m128i;
    fn encode_1(x: u32) -> u32;
}

pub(crate) struct NoEncode;
impl Encoder for NoEncode {
    #[cfg(target_feature = "sse2")]
    #[inline]
    unsafe fn simd_encode_4x32(data: __m128i) -> __m128i {
        data
    }

    #[inline]
    fn encode_1(x: u32) -> u32 {
        x
    }
}

pub(crate) struct ZigZagEncode;
impl Encoder for ZigZagEncode {
    #[cfg(target_feature = "sse2")]
    #[inline]
    unsafe fn simd_encode_4x32(data: __m128i) -> __m128i {
        let data_shl_1 = _mm_add_epi32(data, data);
        let data_shr_31 = _mm_srai_epi32::<31>(data);
        _mm_xor_si128(data_shl_1, data_shr_31)
    }

    #[inline]
    fn encode_1(x: u32) -> u32 {
        let x: i32 = x as i32;
        (x as u32).wrapping_add(x as u32) ^ ((x >> 31) as u32)
    }
}

pub(crate) fn encode_simd<E: Encoder>(input: &[u32]) -> (usize, Vec<u8>) {
    let mut output = Vec::new();
    let items = encode_into_simd::<E>(input, &mut output);
    (items, output)
}

pub(crate) fn encode_into_simd<E: Encoder>(input: &[u32], output: &mut Vec<u8>) -> usize {
    let items = input.len();
    if items == 0 {
        return 0;
    }

    output.reserve(max_compressed_len(items));
    //let mut output: Vec<u8> = Vec::with_capacity(max_compressed_len(items));

    // This always points to where the currently collected control byte needs
    // to be written.
    let controls: *mut u8 = unsafe { output.as_mut_ptr().add(output.len()) };
    let data: *mut u8 = unsafe { controls.add(control_bytes_len(items)) };
    let input: *const u32 = input.as_ptr();

    unsafe {
        let data = encode_worker::<E>(items, input, controls, data);
        let len = data.offset_from(output.as_ptr()) as usize;
        let new_len = output.len() + len;
        debug_assert!(new_len <= output.capacity());
        output.set_len(new_len)
    };

    items
}

use std::arch::x86_64::{
    __m128i, _mm_add_epi32, _mm_adds_epu16, _mm_loadu_si128, _mm_min_epi16, _mm_min_epu8,
    _mm_movemask_epi8, _mm_packus_epi16, _mm_set1_epi16, _mm_set1_epi8, _mm_shuffle_epi8,
    _mm_srai_epi32, _mm_storeu_si128, _mm_xor_si128,
};

unsafe fn encode_worker<E: Encoder>(
    items: usize,
    mut input: *const u32,
    mut controls: *mut u8,
    mut data: *mut u8,
) -> *mut u8 {
    let mask_01: __m128i = _mm_set1_epi8(0x01);
    let mask_7f00: __m128i = _mm_set1_epi16(0x7f00);

    // Based on https://github.com/lemire/streamvbyte/blob/master/src/streamvbyte_x64_encode.c
    // That implementation in turn was contributed by aqrit

    let end: *const u32 = input.add(items & !7);
    while input != end {
        // Load 8 values / 32 bytes into r0, r1
        let r0 = E::simd_encode_4x32(_mm_loadu_si128(input as *const __m128i));
        let r1 = E::simd_encode_4x32(_mm_loadu_si128(input.add(4) as *const __m128i));
        // debug_u8x16(r0);
        // debug_u8x16(r1);
        // Ex: r0 = 11_00_00_00__22_33_00_00__44_55_66_77__88_99_aa_00
        //     r1 = 10_20_00_00__30_40_50_00__60_00_00_90__a0_00_00_00_

        // Turn all non-zero bytes into 1, the rest stay at 0
        let r2 = _mm_min_epu8(mask_01, r0);
        let r3 = _mm_min_epu8(mask_01, r1);
        // println!("_min_epu8");
        // debug_u8x16(r2);
        // debug_u8x16(r3);
        // debug_u16x8(r2);
        // debug_u16x8(r3);
        // Ex: r2 = 01_00_00_00__01_01_00_00__01_01_01_01__01_01_01_00
        //     r3 = 01_01_00_00__01_01_01_00__01_00_00_01__01_00_00_00
        // Seen as u16x8:
        //     r2 = 0001__0000___0101__0000___0101__0101___0101__0001
        //     r3 = 0101__0000___0101__0001___0001__0100___0001__0000

        // Takes [r2,r3] as u16x16 and turns 0101 => ff, the rest stays
        let r2 = _mm_packus_epi16(r2, r3);
        // println!("_packus_epi16");
        // debug_u16x8(r2);
        // debug_u8x16(r2);
        // Ex: r2 = 01_00_ff_00__ff_ff_ff_01__ff_00_ff_01__01_ff_01_00
        // Reinterpreted as u16x8 for next op
        //     r2 = 0001__00ff___ffff__01ff___00ff__01ff___ff01__0001

        // Turn any 01ff into 0101
        let r2 = _mm_min_epi16(r2, mask_01);
        // println!("_min_epi16");
        // Ex: r2 = 0001__00ff___ffff__0101___00ff__0101___ff01__0001
        // debug_u16x8(r2);
        // debug_u8x16(r2);

        // converts: 0x0101 to 0x8001, 0xff01 to 0xffff
        let r2 = _mm_adds_epu16(r2, mask_7f00);
        // Ex: r2 = 7f01__7fff___ffff__8001___7fff__8001___ffff__7f01
        // Back as bytes:
        //     r2 = 01_7f_ff_7f__ff_ff_01_80__ff_7f_01_80__ff_ff_01_7f
        // println!("_mm_adds_epu16");
        // debug_u16x8(r2);
        // debug_u8x16(r2);

        // Takes the highest bit from each byte.
        let keys = _mm_movemask_epi8(r2) as usize;
        //      keys = 0b_00_11_10_01__10_11_01_00
        // Rightmost bit corresponds to first byte.
        //
        // Or in order of input values:
        // (from r0) 00_01_11_10  (from r1) 01_10_11_00
        // println!("_movemask_epi8");
        // println!("{:#b}", keys);

        let r2 = _mm_loadu_si128(
            (ENCODING_SHUFFLE_TABLE.as_ptr() as *const u8).add((keys << 4) & 0x03F0)
                as *const __m128i,
        );
        let r3 = _mm_loadu_si128(
            (ENCODING_SHUFFLE_TABLE.as_ptr() as *const u8).add((keys >> 4) & 0x03F0)
                as *const __m128i,
        );
        // debug_u8x16(r2);
        // debug_u8x16(r3);

        let r0 = _mm_shuffle_epi8(r0, r2);
        let r1 = _mm_shuffle_epi8(r1, r3);

        _mm_storeu_si128(data as *mut __m128i, r0);
        data = data.add(LENGTH_TABLE[keys & 0xff] as usize);
        _mm_storeu_si128(data as *mut __m128i, r1);
        data = data.add(LENGTH_TABLE[keys >> 8] as usize);

        *controls = (keys & 0xff) as u8;
        *controls.add(1) = (keys >> 8) as u8;
        controls = controls.add(2);

        input = input.add(8);
    }
    let mut key: u32 = 0;
    for i in 0..items & 7 {
        let word: u32 = E::encode_1(*input);

        let t1 = (word > 0x000000ff) as u32;
        let t2 = (word > 0x0000ffff) as u32;
        let t3 = (word > 0x00ffffff) as u32;
        let symbol = t1 + t2 + t3;
        key |= symbol << (i + i);
        std::ptr::copy_nonoverlapping((&word) as *const u32 as *const u8, data, 4);
        input = input.add(1);
        data = data.add(symbol as usize + 1);
    }
    std::ptr::copy_nonoverlapping(
        &key as *const u32 as *const u8,
        controls,
        ((items & 7) + 3) >> 2,
    );

    data
}

/*
#[inline]
pub fn zigzag_encode_1(x: i32) -> u32 {
    (x as u32).wrapping_add(x as u32) ^ ((x >> 31) as u32)
}

#[cfg(target_feature = "sse2")]
pub unsafe fn zigzag_encode_4x32(data: __m128i) -> __m128i {
    let data_shl_1 = _mm_add_epi32(data, data); // SSE2
    let data_shr_31 = _mm_srai_epi32::<31>(data); // SSE2
    _mm_xor_si128(data_shl_1, data_shr_31) // SSE2
}

#[cfg(target_feature = "avx2")]
pub unsafe fn zigzag_encode_8x32(data: __m256i) -> __m256i {
    use std::arch::x86_64::{_mm256_add_epi32, _mm256_srai_epi32, _mm256_xor_si256};
    let data_shl_1 = _mm256_add_epi32(data, data);
    let data_shr_31 = _mm256_srai_epi32::<32>(data);
    _mm256_xor_si256(data_shl_1, data_shr_31)
}

pub fn zigzag_encode_into(input: &[i32], output: &mut Vec<u32>) {
    output.reserve(input.len());
    let count = input.len();
    let mut src: *const i32 = input.as_ptr();
    let mut dst: *mut u32 = output.as_mut_ptr();
    unsafe {
        for _ in 0..count {
            *dst = zigzag_encode_1(*src);
            src = src.add(1);
            dst = dst.add(1);
        }
        output.set_len(output.len() + count)
    }
}
// */

#[allow(dead_code, clippy::needless_range_loop)]
fn debug_u8x16(data: __m128i) {
    let mut bytes: [u8; 16] = [0; 16];
    unsafe { _mm_storeu_si128(bytes.as_mut_ptr() as *mut __m128i, data) }
    print!("[");
    for b in 0..16 {
        if b > 1 && (b & 3) == 0 {
            print!("_");
        }
        print!("{:02x}_", bytes[b]);
    }
    println!("]");
}

#[allow(dead_code, clippy::needless_range_loop)]
fn debug_u16x8(data: __m128i) {
    let mut bytes: [u16; 8] = [0; 8];
    unsafe { _mm_storeu_si128(bytes.as_mut_ptr() as *mut __m128i, data) }
    print!("[");
    for b in 0..8 {
        if b > 1 && (b & 1) == 0 {
            print!("_");
        }
        print!("{:04x}__", bytes[b]);
    }
    println!("]");
}

#[allow(unused)]
#[rustfmt::skip]
const ENCODING_SHUFFLE_TABLE: [[u8; 16]; 64] = [
    [0x00, 0x04, 0x08, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x04, 0x08, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x04, 0x08, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x03, 0x04, 0x08, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x04, 0x05, 0x08, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x04, 0x05, 0x08, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x04, 0x05, 0x08, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x08, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x04, 0x05, 0x06, 0x08, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x04, 0x05, 0x06, 0x08, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x04, 0x05, 0x06, 0x08, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x08, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x04, 0x05, 0x06, 0x07, 0x08, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x04, 0x05, 0x06, 0x07, 0x08, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x04, 0x05, 0x06, 0x07, 0x08, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF],
    [0x00, 0x04, 0x08, 0x09, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x04, 0x08, 0x09, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x04, 0x08, 0x09, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x03, 0x04, 0x08, 0x09, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x04, 0x05, 0x08, 0x09, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x04, 0x05, 0x08, 0x09, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x04, 0x05, 0x08, 0x09, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x08, 0x09, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF],
    [0x00, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF],
    [0x00, 0x04, 0x08, 0x09, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x04, 0x08, 0x09, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x04, 0x08, 0x09, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x03, 0x04, 0x08, 0x09, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x04, 0x05, 0x08, 0x09, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x04, 0x05, 0x08, 0x09, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x04, 0x05, 0x08, 0x09, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x08, 0x09, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF],
    [0x00, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF],
    [0x00, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF],
    [0x00, 0x04, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x04, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x04, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x03, 0x04, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF],
    [0x00, 0x04, 0x05, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x04, 0x05, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x04, 0x05, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF],
    [0x00, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF],
    [0x00, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF, 0xFF],
    [0x00, 0x01, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF, 0xFF],
    [0x00, 0x01, 0x02, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0xFF],
    [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F]
];

#[cfg(test)]
mod tests {
    use rand::Rng;

    use crate::x86_64::encode::NoEncode;

    #[test]
    fn encode_step() {
        let values = vec![
            0x11, 0x3322, 0x77665544, 0xaa9988, 0x2010, 0x504030, 0x90000060, 0xa0, 0x70, 0x8000,
        ];
        let (len, encoded) = super::encode_simd::<NoEncode>(&values);
        println!("len={}, encoded: {:x?}", len, encoded);
    }

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

            let (len, encoded) = super::encode_simd::<NoEncode>(&input);
            assert_eq!(len, input.len());

            let decoded = crate::scalar::decode::decode(len, &encoded).unwrap();
            assert_eq!(&input, &decoded);
        }
    }
}

use crate::{
    common::{control_bytes_len, StreamVbyteError},
    tables::{len::LENGTH_TABLE, shuffle::DECODE_SHUFFLE_TABLE},
};

pub fn decode_simd(len: usize, input: &[u8]) -> Result<Vec<u32>, StreamVbyteError> {
    if len == 0 {
        return Ok(Vec::new());
    }
    let end: *const u8 = input.as_ptr_range().end;
    let num_controls = control_bytes_len(len);
    if num_controls >= input.len() {
        return Err(StreamVbyteError::DecodeOutOfBounds);
    }
    let mut control_ptr: *const u8 = input.as_ptr();
    let mut data_ptr: *const u8 = unsafe { input.as_ptr().add(num_controls) };
    let mut result: Vec<u32> = Vec::with_capacity(len);
    let mut output_ptr: *mut u32 = result.as_mut_ptr();
    let mut remaining_len = len;

    // println!("output: {:?}", output_ptr);

    // The SIMD version reads data 16 bytes at once. At least the first 4 bytes
    // correspond to the current control byte (if it is a full control byte).
    // Bu the remaining 12 bytes may not be part of it. To ensure that it never
    // reads past the end of the data stream we must ensure there are always at
    // least 3 full control bytes + 1 potentially partial control byte.
    if num_controls > 4 {
        unsafe {
            let num_controls = num_controls - 4;
            let (new_data_ptr, ok) = decode_neon_worker_checked_unrolled(
                control_ptr,
                data_ptr,
                end,
                output_ptr,
                num_controls,
            );
            if !ok {
                return Err(StreamVbyteError::DecodeOutOfBounds);
            }
            data_ptr = new_data_ptr;
            control_ptr = control_ptr.add(num_controls);
            output_ptr = output_ptr.add(4 * num_controls);
            remaining_len -= 4 * num_controls;
        }
    }
    // Decode the leftovers using scalar decoder.
    unsafe {
        let (_, ok) = crate::scalar::decode::decode_inner_checked(
            control_ptr,
            data_ptr,
            end,
            output_ptr,
            remaining_len,
        );
        if !ok {
            return Err(StreamVbyteError::DecodeOutOfBounds);
        }
    }

    unsafe { result.set_len(len) };

    Ok(result)
}

unsafe fn decode_neon_worker_checked_unrolled(
    mut control_ptr: *const u8,
    mut data_ptr: *const u8,
    end_ptr: *const u8,
    mut decoded_ptr: *mut u32,
    mut num_controls: usize,
) -> (*const u8, bool) {
    // println!(
    //     "n={:?}, ctrl={:?}, data={:?}, end={:?}, out={:?}",
    //     num_controls, control_ptr, data_ptr, end_ptr, decoded_ptr,
    // );
    while num_controls >= 4 {
        let control1 = *control_ptr;
        control_ptr = control_ptr.add(1);
        let control2 = *control_ptr;
        control_ptr = control_ptr.add(1);
        let control3 = *control_ptr;
        control_ptr = control_ptr.add(1);
        let control4 = *control_ptr;
        control_ptr = control_ptr.add(1);

        if data_ptr.add(64) > end_ptr {
            break;
            // return (data_ptr, false);
        }

        num_controls -= 4;

        data_ptr = step_simd(control1, data_ptr, decoded_ptr);
        decoded_ptr = decoded_ptr.add(4_usize);
        data_ptr = step_simd(control2, data_ptr, decoded_ptr);
        decoded_ptr = decoded_ptr.add(4_usize);
        data_ptr = step_simd(control3, data_ptr, decoded_ptr);
        decoded_ptr = decoded_ptr.add(4_usize);
        data_ptr = step_simd(control4, data_ptr, decoded_ptr);
        decoded_ptr = decoded_ptr.add(4_usize);
    }
    // println!("Done big steps");
    // println!(
    //     "n={:?}, ctrl={:?}, data={:?}, end={:?}, out={:?}",
    //     num_controls, control_ptr, data_ptr, end_ptr, decoded_ptr,
    // );
    while num_controls > 0 {
        let control = *control_ptr;
        control_ptr = control_ptr.add(1);
        num_controls -= 1;

        if data_ptr.add(16) > end_ptr {
            return (data_ptr, false);
        }
        data_ptr = step_simd(control, data_ptr, decoded_ptr);
        decoded_ptr = decoded_ptr.add(4_usize);
    }

    (data_ptr, true)
}

use multiversion::target;

// #[cfg(target_feature="neon")]
#[target("aarch64+neon")]
#[inline]
unsafe fn step_simd(control: u8, data_ptr: *const u8, decoded_ptr: *mut u32) -> *const u8 {
    use std::arch::aarch64::{uint8x16_t, vld1q_u8, vqtbl1q_u8, vst1q_u8};

    // Safety: Safe if source data has 12 extra bytes allocated (we always
    // consume at least 4 bytes).
    let encoded: uint8x16_t = vld1q_u8(data_ptr);
    let entry: *const [u8; 16] = &DECODE_SHUFFLE_TABLE[control as usize] as *const _;
    // Safety: the types are compatible and we allow unaligned reads.
    let mask = vld1q_u8(entry as *const u8);
    let decoded = vqtbl1q_u8(encoded, mask);
    let bytes_consumed: u8 = LENGTH_TABLE[control as usize];
    let data_ptr = data_ptr.add(bytes_consumed as usize);
    // Safety: we allocated enough memory.
    vst1q_u8(decoded_ptr as *mut u8, decoded);
    data_ptr
}

#[test]
fn test_step_simd() {
    let control = 0b_1000_0111;
    let data: Vec<u8> = (1..16).collect();
    let mut out: Vec<u32> = vec![0; 4];
    let ofs = unsafe {
        let p = step_simd(control, data.as_ptr(), out.as_mut_ptr());
        p.offset_from(data.as_ptr())
    };
    println!("{:x?}, ofs={}", out, ofs);
    assert_eq!(out, vec![0x04030201, 0x0605, 0x07, 0x0a0908]);
    assert_eq!(ofs, 10);
    // assert!(false)
}

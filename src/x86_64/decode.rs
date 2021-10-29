use std::arch::x86_64::{__m128i, _mm_loadu_si128, _mm_shuffle_epi8, _mm_storeu_si128};

use crate::{
    common::{control_bytes_len, StreamVbyteError},
    tables::len::LENGTH_TABLE,
    tables::shuffle::DECODE_SHUFFLE_TABLE,
};

pub fn decode_simd(len: usize, input: &[u8]) -> Result<Vec<u32>, StreamVbyteError> {
    // FIXME: We currently totally trust `len`. That can cause us to read past
    // the end.
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

    // The SIMD version reads data 16 bytes at once. At least the first 4 bytes
    // correspond to the current control byte (if it is a full control byte).
    // Bu the remaining 12 bytes may not be part of it. To ensure that it never
    // reads past the end of the data stream we must ensure there are always at
    // least 3 full control bytes + 1 potentially partial control byte.
    if num_controls > 4 {
        unsafe {
            let num_controls = num_controls - 4;
            let (new_data_ptr, ok) =
                decode_ssse3_worker_checked(control_ptr, data_ptr, end, output_ptr, num_controls);
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

pub fn decode_simd1(len: usize, input: &[u8]) -> Result<Vec<u32>, StreamVbyteError> {
    // FIXME: We currently totally trust `len`. That can cause us to read past
    // the end.
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
            let (new_data_ptr, ok) = decode_ssse3_worker_checked_unrolled(
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

/// # Safety
///
/// Safe if `len` is the value that was produced by `encode`. Otherwise, it may
/// read out-of-bounds memory.
pub unsafe fn decode_simd_trusted_len(len: usize, input: &[u8]) -> Vec<u32> {
    // FIXME: We currently totally trust `len`. That can cause us to read past
    // the end.
    if len == 0 {
        return Vec::new();
    }
    let num_controls = control_bytes_len(len);
    let mut control_ptr: *const u8 = input.as_ptr();
    let mut data_ptr: *const u8 = unsafe { input.as_ptr().add(num_controls) };
    let mut result: Vec<u32> = Vec::with_capacity(len);
    let mut output_ptr: *mut u32 = result.as_mut_ptr();
    let mut remaining_len = len;

    // The SIMD version reads data 16 bytes at once. At least the first 4 bytes
    // correspond to the current control byte (if it is a full control byte).
    // Bu the remaining 12 bytes may not be part of it. To ensure that it never
    // reads past the end of the data stream we must ensure there are always at
    // least 3 full control bytes + 1 potentially partial control byte.
    if num_controls > 4 {
        unsafe {
            let num_controls = num_controls - 4;
            data_ptr = decode_ssse3_worker(control_ptr, data_ptr, output_ptr, num_controls);
            control_ptr = control_ptr.add(num_controls);
            output_ptr = output_ptr.add(4 * num_controls);
            remaining_len -= 4 * num_controls;
        }
    }
    // Decode the leftovers using scalar decoder.
    unsafe {
        crate::scalar::decode::decode_inner(control_ptr, data_ptr, output_ptr, remaining_len);
    }

    unsafe { result.set_len(len) };

    result
}

unsafe fn decode_ssse3_worker(
    mut control_ptr: *const u8,
    mut data_ptr: *const u8,
    mut decoded_ptr: *mut u32,
    mut num_controls: usize,
) -> *const u8 {
    while num_controls > 0 {
        let control = *control_ptr;
        control_ptr = control_ptr.add(1);
        num_controls -= 1;

        // Safety: Safe if source data has 12 extra bytes allocated (we always
        // consume at least 4 bytes).
        let encoded: __m128i = _mm_loadu_si128(data_ptr as *const __m128i);
        let entry: *const [u8; 16] = &DECODE_SHUFFLE_TABLE[control as usize] as *const _;
        // Safety: the types are compatible and we allow unaligned reads.
        let mask = _mm_loadu_si128(entry as *const __m128i);
        let decoded = _mm_shuffle_epi8(encoded, mask);
        let bytes_consumed: u8 = LENGTH_TABLE[control as usize];
        data_ptr = data_ptr.add(bytes_consumed as usize);
        // Safety: we allocated enough memory.
        _mm_storeu_si128(decoded_ptr as *mut __m128i, decoded);
        decoded_ptr = decoded_ptr.add(4_usize);
    }
    data_ptr
}

unsafe fn decode_ssse3_worker_checked(
    mut control_ptr: *const u8,
    mut data_ptr: *const u8,
    end_ptr: *const u8,
    mut decoded_ptr: *mut u32,
    mut num_controls: usize,
) -> (*const u8, bool) {
    while num_controls > 0 {
        let control = *control_ptr;
        control_ptr = control_ptr.add(1);
        num_controls -= 1;

        if data_ptr.add(16) > end_ptr {
            return (data_ptr, false);
        }

        // Safety: Safe if source data has 12 extra bytes allocated (we always
        // consume at least 4 bytes).
        let encoded: __m128i = _mm_loadu_si128(data_ptr as *const __m128i);
        let entry: *const [u8; 16] = &DECODE_SHUFFLE_TABLE[control as usize] as *const _;
        // Safety: the types are compatible and we allow unaligned reads.
        let mask = _mm_loadu_si128(entry as *const __m128i);
        let decoded = _mm_shuffle_epi8(encoded, mask);
        let bytes_consumed: u8 = LENGTH_TABLE[control as usize];
        data_ptr = data_ptr.add(bytes_consumed as usize);
        // Safety: we allocated enough memory.
        _mm_storeu_si128(decoded_ptr as *mut __m128i, decoded);
        decoded_ptr = decoded_ptr.add(4_usize);
    }
    (data_ptr, true)
}

unsafe fn decode_ssse3_worker_checked_unrolled(
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

unsafe fn step_simd(control: u8, data_ptr: *const u8, decoded_ptr: *mut u32) -> *const u8 {
    // Safety: Safe if source data has 12 extra bytes allocated (we always
    // consume at least 4 bytes).
    let encoded: __m128i = _mm_loadu_si128(data_ptr as *const __m128i);
    let entry: *const [u8; 16] = &DECODE_SHUFFLE_TABLE[control as usize] as *const _;
    // Safety: the types are compatible and we allow unaligned reads.
    let mask = _mm_loadu_si128(entry as *const __m128i);
    let decoded = _mm_shuffle_epi8(encoded, mask);
    let bytes_consumed: u8 = LENGTH_TABLE[control as usize];
    let data_ptr = data_ptr.add(bytes_consumed as usize);
    // Safety: we allocated enough memory.
    _mm_storeu_si128(decoded_ptr as *mut __m128i, decoded);
    data_ptr
}

#[cfg(test)]
mod tests {
    use crate::safe::encode;

    use super::*;

    #[test]
    fn basic() {
        let inputs = &[
            vec![],
            vec![42],
            vec![300],
            vec![70000],
            vec![0x12345678],
            vec![1000, 2000],
            vec![1, 2, 3],
            vec![1, 288, 3],
            vec![1, 288, 3, 94320],
            vec![1, 288, 3, 123123, 83291],
            vec![1, 288, 3, 123123, 83291, 82, 30],
            vec![1, 288, 3, 123123, 83291, 82, 16621, 30],
            (1..101).collect(),
            (1..102).collect(),
            (1..103).collect(),
            (1..104).collect(),
            (1000..1101).collect(),
            (1000..1102).collect(),
            (1000..1103).collect(),
            (1000..1104).collect(),
            (10..1101).collect(),
            (10..1102).collect(),
            (10..1103).collect(),
            (10..1104).collect(),
        ];
        for input in inputs {
            //println!("{:?}", input);
            let (len, bytes) = encode(input);
            let decoded = decode_simd1(len, &bytes).unwrap();
            assert_eq!(input, &decoded);
        }
    }

    #[test]
    fn wrong_len() {
        let inputs = &[
            vec![],
            vec![42],
            vec![300],
            vec![70000],
            vec![0x12345678],
            vec![1000, 2000],
            vec![1, 2, 3],
            vec![1, 288, 3],
            vec![1, 288, 3, 94320],
            vec![1, 288, 3, 123123, 83291],
            vec![1, 288, 3, 123123, 83291, 82, 30],
            vec![1, 288, 3, 123123, 83291, 82, 16621, 30],
            (1..101).collect(),
            (1..102).collect(),
            (1..103).collect(),
            (1..104).collect(),
            (1000..1101).collect(),
            (1000..1102).collect(),
            (1000..1103).collect(),
            (1000..1104).collect(),
            (10..1101).collect(),
            (10..1102).collect(),
            (10..1103).collect(),
            (10..1104).collect(),
        ];
        for input in inputs {
            //println!("{:?}", input);
            let (len, bytes) = encode(input);
            assert!(decode_simd1(len + 1, &bytes).is_err());
            //assert_eq!(input, &decoded);
        }
    }
}

use std::arch::x86_64::{__m128i, _mm_loadu_si128, _mm_shuffle_epi8, _mm_storeu_si128};

use crate::{common::control_bytes_len, tables::shuffle::DECODE_SHUFFLE_TABLE, LENGTH_TABLE};

pub fn decode_simd(len: usize, input: &[u8]) -> Vec<u32> {
    if len == 0 {
        return Vec::new();
    }
    let num_controls = control_bytes_len(len);
    // TODO: Handle left-over controls bits
    assert_eq!(num_controls * 4, len);

    let control_ptr: *const u8 = input.as_ptr();
    let encoded_ptr: *const u8 = unsafe { input.as_ptr().add(num_controls) };
    let mut result: Vec<u32> = Vec::with_capacity(len);
    let decoded_ptr: *mut u32 = result.as_mut_ptr();

    // FIXME: SIMD version should only run if there are at least 4 decoding bytes.
    // Otherwise, it may read past the end of the input buffer as it reads 16
    // bytes at a time.

    let _ = unsafe { decode_ssse3_worker(control_ptr, encoded_ptr, decoded_ptr, num_controls) };

    unsafe { result.set_len(len) };

    result
}

unsafe fn decode_ssse3_worker(
    mut control_ptr: *const u8,
    mut encoded_ptr: *const u8,
    mut decoded_ptr: *mut u32,
    mut num_controls: usize,
) -> *const u8 {
    while num_controls > 0 {
        let control = *control_ptr;
        control_ptr = control_ptr.add(1);
        num_controls -= 1;

        // Safety: Safe if source data has 12 extra bytes allocated (we always
        // consume at least 4 bytes).
        let encoded: __m128i = _mm_loadu_si128(encoded_ptr as *const __m128i);
        let entry: *const [u8; 16] = &DECODE_SHUFFLE_TABLE[control as usize] as *const _;
        // Safety: the types are compatible and we allow unaligned reads.
        let mask = _mm_loadu_si128(entry as *const __m128i);
        let decoded = _mm_shuffle_epi8(encoded, mask);
        let bytes_consumed: u8 = LENGTH_TABLE[control as usize];
        encoded_ptr = encoded_ptr.add(bytes_consumed as usize);
        // Safety: we allocated enough memory.
        _mm_storeu_si128(decoded_ptr as *mut __m128i, decoded);
        decoded_ptr = decoded_ptr.add(4_usize);
    }
    encoded_ptr
}

#[cfg(test)]
mod tests {
    use crate::safe::encode;

    use super::decode_simd;

    #[test]
    fn basic() {
        let inputs = &[
            // vec![],
            // vec![42],
            // vec![300],
            // vec![70000],
            // vec![0x12345678],
            // vec![1000, 2000],
            // vec![1, 2, 3],
            // vec![1, 288, 3],
            vec![1, 288, 3, 94320],
            // vec![1, 288, 3, 123123, 83291],
            // vec![1, 288, 3, 123123, 83291, 82, 30],
            vec![1, 288, 3, 123123, 83291, 82, 16621, 30],
        ];
        for input in inputs {
            //println!("{:?}", input);
            let (len, bytes) = encode(input);
            let decoded = decode_simd(len, &bytes);
            assert_eq!(input, &decoded);
        }
    }
}

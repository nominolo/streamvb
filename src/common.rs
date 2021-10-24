use std::mem;

/// Maximum length of the compressed output vector.
pub fn max_compressed_len(input_len: usize) -> usize {
    let simd_max_overread = 16 - 4;
    let max_data_bytes = input_len * mem::size_of::<u32>();
    control_bytes_len(input_len) + max_data_bytes + simd_max_overread
}

/// Exact number of control bytes in the compressed output vector.
pub fn control_bytes_len(input_len: usize) -> usize {
    // 2 bits per input, round up to next byte
    (input_len + 3) / 4
}

/// Compute the exact compressed output length in bytes. `O(n)` because it needs
/// to read the full input.
pub fn exact_compressed_len(input: &[u32]) -> usize {
    let mut len = 0;
    for value in input {
        if *value < (1 << 8) {
            len += 1;
        } else if *value < (1 << 16) {
            len += 2;
        } else if *value < (1 << 24) {
            len += 3;
        } else {
            len += 4;
        }
    }
    len
}

#[cfg(test)]
mod tests {}

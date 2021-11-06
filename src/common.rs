use std::mem;

/// Maximum length of the compressed output vector where control bytes and
/// data bytes are combined into one vector.
pub fn max_compressed_len(input_len: usize) -> usize {
    let max_data_bytes = input_len * mem::size_of::<u32>();
    control_bytes_len(input_len) + max_data_bytes
}

/// Exact number of control bytes in the compressed output vector.
pub fn control_bytes_len(input_len: usize) -> usize {
    // 4 numbers per control byte (2 bits per input), round up to next byte
    (input_len + 3) / 4
}

/// Compute the exact compressed data length in bytes. `O(n)` because it needs
/// to read the full input.
pub fn exact_compressed_len(input: &[u32]) -> usize {
    let mut len = 0;
    for value in input {
        let t1 = (*value > 0x000000ff) as u32;
        let t2 = (*value > 0x0000ffff) as u32;
        let t3 = (*value > 0x00ffffff) as u32;
        len += (t1 + t2 + t3 + 1) as usize;
    }
    len
}

#[derive(Debug)]
pub enum StreamVbyteError {
    DecodeOutOfBounds,
}

#[cfg(test)]
mod tests {}

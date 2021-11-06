use crate::common::{control_bytes_len, max_compressed_len};

pub fn encode(input: &[u32]) -> (usize, Vec<u8>) {
    let items = input.len();
    if items == 0 {
        return (0, Vec::new());
    }

    let mut output: Vec<u8> = Vec::with_capacity(max_compressed_len(items));

    // This always points to where the currently constructed control byte needs
    // to be written.
    let controls: *mut u8 = output.as_mut_ptr();
    let data: *mut u8 = unsafe { controls.add(control_bytes_len(items)) };
    let input: *const u32 = input.as_ptr();

    // Safety:
    //   - We read exactly `items` u32 values from input
    //   - We write exactly `ceil(item/4)` bytes to `controls`
    //   - We write at most `items * 4` bytes into `data`.
    unsafe {
        let data = encode_worker(items, input, controls, data);
        let len = data.offset_from(output.as_ptr()) as usize;
        debug_assert!(len <= output.capacity());
        output.set_len(len)
    };

    (items, output)
}

unsafe fn encode_worker(
    items: usize,
    mut input: *const u32,
    mut controls: *mut u8,
    mut data: *mut u8,
) -> *mut u8 {
    let mut key: u32 = 0;
    let full_controls = items / 4;

    // Encode 4 values per iteration. That yields one whole control byte.
    //
    // We always write 4 bytes (using unaligned writes) and then overwrite the
    // unnecessary bytes later. This is safe because the output must have enough
    // capacity for the worst case where all values require 4 bytes.
    for _i in 0..full_controls {
        let word1 = *input;
        let word2 = *input.add(1);
        let word3 = *input.add(2);
        let word4 = *input.add(3);

        let symbol1 = encode_one(word1);
        key |= symbol1;
        // Use copy_nonoverlapping because we're doing unaligned writes.
        std::ptr::copy_nonoverlapping(input as *const u8, data, 4);
        data = data.add(symbol1 as usize + 1);

        let symbol2 = encode_one(word2);
        key |= symbol2 << 2;
        std::ptr::copy_nonoverlapping(input.add(1) as *const u8, data, 4);
        data = data.add(symbol2 as usize + 1);

        let symbol3 = encode_one(word3);
        key |= symbol3 << 4;
        std::ptr::copy_nonoverlapping(input.add(2) as *const u8, data, 4);
        data = data.add(symbol3 as usize + 1);

        let symbol4 = encode_one(word4);
        key |= symbol4 << 6;
        std::ptr::copy_nonoverlapping(input.add(3) as *const u8, data, 4);
        data = data.add(symbol4 as usize + 1);

        input = input.add(4);

        *controls = key as u8;
        controls = controls.add(1);
        key = 0;
    }
    if items & 3 > 0 {
        // handle the rest
        for i in 0..items & 3 {
            let word = *input;
            let symbol = encode_one(word);
            key |= symbol << (i + i);
            std::ptr::copy_nonoverlapping(input as *const u8, data, 4);
            input = input.add(1);
            data = data.add(symbol as usize + 1);
        }
        *controls = key as u8;
    }
    data
}

fn encode_one(word: u32) -> u32 {
    let t1 = (word > 0x000000ff) as u32;
    let t2 = (word > 0x0000ffff) as u32;
    let t3 = (word > 0x00ffffff) as u32;
    t1 + t2 + t3
}

#[cfg(test)]
mod tests {
    use super::encode;

    #[test]
    fn short() {
        assert_eq!(encode(&[]), (0, vec![]));

        assert_eq!(encode(&[1]), (1, vec![0, 1]));
        assert_eq!(encode(&[300]), (1, vec![0x1, 44, 1]));
        assert_eq!(encode(&[70000]), (1, vec![0x2, 112, 17, 1]));
        assert_eq!(encode(&[0x12345678]), (1, vec![3, 0x78, 0x56, 0x34, 0x12]));

        assert_eq!(encode(&[1, 2]), (2, vec![0, 1, 2]));
        assert_eq!(encode(&[1, 2, 3]), (3, vec![0, 1, 2, 3]));
        assert_eq!(encode(&[1, 2, 3, 4]), (4, vec![0, 1, 2, 3, 4]));
        assert_eq!(encode(&[1, 2, 3, 4, 5]), (5, vec![0, 0, 1, 2, 3, 4, 5]));

        assert_eq!(
            super::encode(&[0, 23, 99, 301, 70211, 89902932]),
            (6, vec![64, 14, 0, 23, 99, 45, 1, 67, 18, 1, 84, 207, 91, 5])
        );
    }
}

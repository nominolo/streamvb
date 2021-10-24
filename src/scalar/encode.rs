use crate::common::{control_bytes_len, max_compressed_len};

pub fn encode(input: &[u32]) -> (usize, Vec<u8>) {
    let items = input.len();
    if items == 0 {
        return (0, Vec::new());
    }

    let mut output: Vec<u8> = Vec::with_capacity(max_compressed_len(items));

    // This always points to where the currently collected control byte needs
    // to be written.
    let mut control: *mut u8 = output.as_mut_ptr();
    let mut data: *mut u8 = unsafe { control.add(control_bytes_len(items)) };

    // We accumulate the next control byte in `key`
    let mut key: u8 = 0;
    let mut shift: u8 = 0;
    for val in input {
        if shift == 8 {
            // control byte has been filled. Write to memory, and advance the
            // pointer.
            shift = 0;
            // Safety: Called for every 4 input values. So never advances past
            // offset: control_bytes_len(items)
            unsafe {
                *control = key;
                control = control.add(1);
            }
            key = 0;
        }
        let code = unsafe { encode_single(*val, &mut data) };
        key |= code << shift;
        shift += 2;
    }
    unsafe {
        // Write back last partial key.
        *control = key;
        // Set proper array length. `data` points to first byte outside of array
        let len = data.offset_from(output.as_ptr()) as usize;
        debug_assert!(len <= output.capacity());
        output.set_len(len)
    };
    (items, output)
}

unsafe fn encode_single(val: u32, out: &mut *mut u8) -> u8 {
    let bytes: [u8; 4] = val.to_le_bytes();
    // Write all the bytes in little endian. We later overwrite them of necessary.
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), *out, 4);
    if val < (1 << 8) {
        *out = out.add(1);
        0
    } else if val < (1 << 16) {
        *out = out.add(2);
        1
    } else if val < (1 << 24) {
        *out = out.add(3);
        2
    } else {
        *out = out.add(4);
        3
    }
}

#[cfg(test)]
mod tests {
    use crate::scalar::encode::encode;

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

        //println!("{:?}", crate::safe::encode(&[0, 23, 99, 301, 70211, 89902932]));
        assert_eq!(
            encode(&[0, 23, 99, 301, 70211, 89902932]),
            (6, vec![64, 14, 0, 23, 99, 45, 1, 67, 18, 1, 84, 207, 91, 5])
        );
    }
}

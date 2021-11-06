use crate::common::{control_bytes_len, StreamVbyteError};

#[inline]
unsafe fn extract_bytes(data: *const u8, count: u8) -> u32 {
    if count == 1 {
        // 1 byte
        *data as u32
    } else if count == 2 {
        let mut result: u32 = 0;
        std::ptr::copy_nonoverlapping(data, (&mut result) as *mut u32 as *mut u8, 2);
        // 2 bytes
        result
    } else if count == 3 {
        let mut result: u32 = 0;
        std::ptr::copy_nonoverlapping(data, (&mut result) as *mut u32 as *mut u8, 3);
        result
    } else {
        let mut result: u32 = 0;
        std::ptr::copy_nonoverlapping(data, (&mut result) as *mut u32 as *mut u8, 4);
        result
    }
}

pub fn decode(len: usize, input: &[u8]) -> Result<Vec<u32>, StreamVbyteError> {
    if len == 0 {
        return Ok(Vec::new());
    }
    let end: *const u8 = input.as_ptr_range().end;
    let num_control_bytes = control_bytes_len(len);
    if num_control_bytes >= input.len() {
        return Err(StreamVbyteError::DecodeOutOfBounds);
    }
    let control: *const u8 = input.as_ptr();
    let data: *const u8 = unsafe { input.as_ptr().add(num_control_bytes) };
    let mut result: Vec<u32> = Vec::with_capacity(len);
    let out: *mut u32 = result.as_mut_ptr();
    unsafe {
        let (_out, ok) = decode_unroll_inner_checked(control, data, end, out, len);
        if ok {
            result.set_len(len);
        } else {
            return Err(StreamVbyteError::DecodeOutOfBounds);
        }
    }
    Ok(result)
}

// Returns the final output pointer and whether all values were in bounds.
// If `(_, false)` is returned, decoding ended early because the
#[inline]
pub(crate) unsafe fn decode_unroll_inner_checked(
    mut control: *const u8,
    mut data: *const u8,
    end: *const u8,
    mut out: *mut u32,
    len: usize,
) -> (*mut u32, bool) {
    // We know: control < data, Therfore, if we run out of bounds it will be
    // the data pointer.
    let mut len_remaining = len;
    while len_remaining >= 4 {
        let key = *control as u32;

        if data.add(16) >= end {
            break;
        }
        len_remaining -= 4;

        control = control.add(1);

        let key1 = key & 0x3;
        let key2 = (key >> 2) & 0x3;
        let key3 = (key >> 4) & 0x3;
        let key4 = key >> 6;

        let val: u32 = (data as *const u32).read_unaligned();
        *out = val & !((!0xff) << (8 * key1)); // mask out the extra bytes
        data = data.add(key1 as usize + 1);

        let val: u32 = (data as *const u32).read_unaligned();
        *out.add(1) = val & !((!0xff) << (8 * key2));
        data = data.add(key2 as usize + 1);

        let val: u32 = (data as *const u32).read_unaligned();
        *out.add(2) = val & !((!0xff) << (8 * key3));
        data = data.add(key3 as usize + 1);

        let val: u32 = (data as *const u32).read_unaligned();
        *out.add(3) = val & !((!0xff) << (8 * key4));
        data = data.add(key4 as usize + 1);
        out = out.add(4);
    }

    let mut key = *control;
    control = control.add(1);

    let mut shift = 0;
    for _ in 0..len_remaining {
        if shift == 8 {
            key = *control;
            control = control.add(1);
            shift = 0;
        }
        let nbytes = ((key >> shift) & 0x3) + 1;
        let next_data = data.add(nbytes as usize);
        // Out of bounds access?
        if next_data > end {
            return (out, false);
        }
        let val = extract_bytes(data, nbytes);
        data = next_data;
        *out = val;
        out = out.add(1);
        shift += 2;
    }
    (out, true)
}

#[cfg(test)]
mod tests {
    use crate::scalar::{decode, encode};

    #[test]
    fn encode_decode() {
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
        ];
        for input in inputs {
            //println!("{:?}", input);
            let (len, bytes) = encode(input);
            let decoded = decode(len, &bytes).unwrap();
            assert_eq!(input, &decoded);
        }
    }

    #[test]
    fn encode_decode_bad_length() {
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
        ];
        for input in inputs {
            //println!("{:?}", input);
            let (len, bytes) = encode(input);
            assert!(decode(len + 1, &bytes).is_err());
            // let decoded = decode(len + 1, &bytes).unwrap();
            // assert_eq!(input, &decoded);
        }
    }
}

use crate::common::control_bytes_len;

pub fn decode(len: usize, input: &[u8]) -> Vec<u32> {
    if len == 0 {
        return Vec::new();
    }
    let num_control_bytes = control_bytes_len(len);
    let mut control: *const u8 = input.as_ptr();
    let mut data: *const u8 = unsafe { input.as_ptr().add(num_control_bytes) };
    let mut result: Vec<u32> = Vec::with_capacity(len);
    let mut out: *mut u32 = result.as_mut_ptr();
    let mut key = unsafe { *control };
    unsafe { control = control.add(1) };
    let mut shift = 0;
    for _ in 0..len {
        if shift == 8 {
            key = unsafe { *control };
            unsafe { control = control.add(1) };
            shift = 0;
        }
        let (val, bytes) = unsafe { decode_single(data, (key >> shift) & 0x3) };
        unsafe {
            data = data.add(bytes);
            *out = val;
            out = out.add(1);
        };
        shift += 2;
    }
    unsafe {
        result.set_len(len);
    }
    result
}

unsafe fn decode_single(data: *const u8, control: u8) -> (u32, usize) {
    if control == 0 {
        // 1 byte
        (*data as u32, 1)
    } else if control == 1 {
        let mut result: u32 = 0;
        std::ptr::copy_nonoverlapping(data, (&mut result) as *mut u32 as *mut u8, 2);
        // 2 bytes
        (result, 2)
    } else if control == 2 {
        let mut result: u32 = 0;
        std::ptr::copy_nonoverlapping(data, (&mut result) as *mut u32 as *mut u8, 3);
        (result, 3)
    } else {
        let mut result: u32 = 0;
        std::ptr::copy_nonoverlapping(data, (&mut result) as *mut u32 as *mut u8, 4);
        (result, 4)
    }
}

#[cfg(test)]
mod tests {
    use crate::scalar::{decode::decode, encode::encode};

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
            let decoded = decode(len, &bytes);
            assert_eq!(input, &decoded);
        }
    }
}

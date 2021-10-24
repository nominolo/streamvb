use crate::common::{control_bytes_len, max_compressed_len};

pub fn encode(input: &[u32]) -> (usize, Vec<u8>) {
    let items = input.len();
    if items == 0 {
        return (0, Vec::new());
    }

    let mut output: Vec<u8> = Vec::with_capacity(max_compressed_len(items));
    output.resize(control_bytes_len(items), 0);
    let mut controls: Vec<u8> = Vec::with_capacity(control_bytes_len(items));

    let mut shift: u8 = 0;
    let mut key: u8 = 0;
    for val in input {
        if shift == 8 {
            shift = 0;
            controls.push(key);
            key = 0;
        }
        let code = encode_single(*val, &mut output);
        key |= code << shift;
        shift += 2;
    }
    controls.push(key);
    debug_assert_eq!(controls.len(), control_bytes_len(items));
    let (control, _) = output.split_at_mut(control_bytes_len(items));
    control.copy_from_slice(&controls);
    (items, output)
}

fn encode_single(val: u32, out: &mut Vec<u8>) -> u8 {
    let bytes: [u8; 4] = val.to_le_bytes();
    if val < (1 << 8) {
        out.push(bytes[0]);
        0
    } else if val < (1 << 16) {
        out.extend_from_slice(&bytes[0..2]);
        1
    } else if val < (1 << 24) {
        out.extend_from_slice(&bytes[0..3]);
        2
    } else {
        out.extend_from_slice(&bytes);
        3
    }
}

mod tests {
    use super::encode;
    #[test]
    fn single() {
        assert_eq!(encode(&[]), (0, vec![]));
        assert_eq!(encode(&[1]), (1, vec![0, 1]));
        assert_eq!(encode(&[300]), (1, vec![0x1, 44, 1]));
        assert_eq!(encode(&[70000]), (1, vec![0x2, 112, 17, 1]));
        assert_eq!(encode(&[0x12345678]), (1, vec![3, 0x78, 0x56, 0x34, 0x12]));
    }

    #[test]
    fn short() {
        assert_eq!(encode(&[1, 2]), (2, vec![0, 1, 2]));
        assert_eq!(encode(&[1, 2, 3]), (3, vec![0, 1, 2, 3]));
        assert_eq!(encode(&[1, 2, 3, 4]), (4, vec![0, 1, 2, 3, 4]));
        assert_eq!(encode(&[1, 2, 3, 4, 5]), (5, vec![0, 0, 1, 2, 3, 4, 5]));
    }
}

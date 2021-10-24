pub mod common;
pub mod scalar;

#[cfg(test)]
pub mod safe;

use std::convert::TryInto;

fn main() {
    //    let control = &[0b01000011]
    // let (control, data) = encode(&[1024, 12, 10, 1_073_741_824, 1, 2, 3, 1024]);
    // println!("{:x?}", control);
    // println!("{:x?}", data);

    // println!("Hello, world!");
}

fn encode_single(val: u32, out: &mut Vec<u8>) -> u8 {
    let bytes: [u8; 4] = val.to_le_bytes();
    if val < (1 << 8) {
        out.push(bytes[0]);
        0
    } else if val < (1 << 16) {
        out.extend_from_slice(&bytes[..2]);
        1
    } else if val < (1 << 24) {
        out.extend_from_slice(&bytes[..3]);
        2
    } else {
        out.extend_from_slice(&bytes);
        3
    }
}

pub fn decode(control: &[u8], data: &[u8]) -> Vec<u32> {
    let mut result = Vec::with_capacity(control.len() * 4);
    let mut offset = 0;
    for c in control {
        for shift in (0..4).map(|s| s * 2) {
            let (val, bytes) = decode_single(&data[offset..], (*c >> shift) & 0x3);
            offset += bytes;
            result.push(val);
        }
    }
    result
}

pub fn decode_single(data: &[u8], control: u8) -> (u32, usize) {
    if control == 0 {
        // 1 byte
        (data[0] as u32, 1)
    } else if control == 1 {
        // 2 bytes
        (u16::from_le_bytes([data[0], data[1]]) as u32, 2)
    } else if control == 2 {
        (u32::from_le_bytes([data[0], data[1], data[2], 0]), 3)
    } else {
        (u32::from_le_bytes(data[0..4].try_into().unwrap()), 4)
    }
}

pub fn debug_print_control(control: &[u8]) {
    let mut line_length = 0;
    for c in control {
        for shift in (0..4).map(|s| s * 2) {
            let ctrl = (*c >> shift) & 0x3;
            print!("{} ", ctrl + 1);
        }
        line_length += 2;
        if line_length >= 64 {
            println!();
            line_length = 0;
        }
    }
    if line_length != 0 {
        println!()
    }
}

// Example:
//
// control byte: 00|01|11|00  =>  sizes: 1|2|4|1
//
// shuffle mask: describes where each output byte should be come from. `z` means
// it should be set to zero. It is encoded as `0x80`
//
// [0, z, z, z, 1, 2, z, z, 3, 4, 5, 6, 7, z, z, z]
//
#[test]
fn build_shuffle_table() {
    println!("#[rustfmt::skip]");
    println!("static SHUFFLE_TABLE: [[u8; 16]; 256] = [");
    for b0 in 1..5 {
        for b1 in 1..5 {
            for b2 in 1..5 {
                for b3 in 1..5 {
                    let mut shuf = [0xff_u8; 16];
                    let mut src_ofs = 0;
                    for i in 0..b3 {
                        shuf[i] = src_ofs;
                        src_ofs += 1;
                    }
                    for i in 0..b2 {
                        shuf[4 + i] = src_ofs;
                        src_ofs += 1;
                    }
                    for i in 0..b1 {
                        shuf[8 + i] = src_ofs;
                        src_ofs += 1;
                    }
                    for i in 0..b0 {
                        shuf[12 + i] = src_ofs;
                        src_ofs += 1;
                    }
                    print!("    [");
                    for b in shuf {
                        if b < 0x80 {
                            print!("{:4}, ", b);
                        } else {
                            print!("0xff, ");
                        }
                    }
                    println!("],");
                }
            }
        }
    }
    println!("];")
}

#[test]
fn build_length_table() {
    println!("#[rustfmt::skip]");
    println!("static LENGTH_TABLE: [u8; 256] = [");
    for b0 in 1..5 {
        for b1 in 1..5 {
            print!("    ");
            for b2 in 1..5 {
                for b3 in 1..5 {
                    print!("{:2}, ", b0 + b1 + b2 + b3);
                }
            }
            println!();
        }
    }
    println!("];")
}

#[allow(unused)]
#[rustfmt::skip]
static LENGTH_TABLE: [u8; 256] = [
     4,  5,  6,  7,  5,  6,  7,  8,  6,  7,  8,  9,  7,  8,  9, 10, 
     5,  6,  7,  8,  6,  7,  8,  9,  7,  8,  9, 10,  8,  9, 10, 11, 
     6,  7,  8,  9,  7,  8,  9, 10,  8,  9, 10, 11,  9, 10, 11, 12, 
     7,  8,  9, 10,  8,  9, 10, 11,  9, 10, 11, 12, 10, 11, 12, 13, 
     5,  6,  7,  8,  6,  7,  8,  9,  7,  8,  9, 10,  8,  9, 10, 11, 
     6,  7,  8,  9,  7,  8,  9, 10,  8,  9, 10, 11,  9, 10, 11, 12, 
     7,  8,  9, 10,  8,  9, 10, 11,  9, 10, 11, 12, 10, 11, 12, 13, 
     8,  9, 10, 11,  9, 10, 11, 12, 10, 11, 12, 13, 11, 12, 13, 14, 
     6,  7,  8,  9,  7,  8,  9, 10,  8,  9, 10, 11,  9, 10, 11, 12, 
     7,  8,  9, 10,  8,  9, 10, 11,  9, 10, 11, 12, 10, 11, 12, 13, 
     8,  9, 10, 11,  9, 10, 11, 12, 10, 11, 12, 13, 11, 12, 13, 14, 
     9, 10, 11, 12, 10, 11, 12, 13, 11, 12, 13, 14, 12, 13, 14, 15, 
     7,  8,  9, 10,  8,  9, 10, 11,  9, 10, 11, 12, 10, 11, 12, 13, 
     8,  9, 10, 11,  9, 10, 11, 12, 10, 11, 12, 13, 11, 12, 13, 14, 
     9, 10, 11, 12, 10, 11, 12, 13, 11, 12, 13, 14, 12, 13, 14, 15, 
    10, 11, 12, 13, 11, 12, 13, 14, 12, 13, 14, 15, 13, 14, 15, 16, 
];

#[allow(unused)]
#[rustfmt::skip]
static SHUFFLE_TABLE: [[u8; 16]; 256] = [
    [   0, 0xff, 0xff, 0xff,    1, 0xff, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3, 0xff, 0xff, 0xff, ],
    [   0,    1, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3, 0xff, 0xff, 0xff,    4, 0xff, 0xff, 0xff, ],
    [   0,    1,    2, 0xff,    3, 0xff, 0xff, 0xff,    4, 0xff, 0xff, 0xff,    5, 0xff, 0xff, 0xff, ],
    [   0,    1,    2,    3,    4, 0xff, 0xff, 0xff,    5, 0xff, 0xff, 0xff,    6, 0xff, 0xff, 0xff, ],
    [   0, 0xff, 0xff, 0xff,    1,    2, 0xff, 0xff,    3, 0xff, 0xff, 0xff,    4, 0xff, 0xff, 0xff, ],
    [   0,    1, 0xff, 0xff,    2,    3, 0xff, 0xff,    4, 0xff, 0xff, 0xff,    5, 0xff, 0xff, 0xff, ],
    [   0,    1,    2, 0xff,    3,    4, 0xff, 0xff,    5, 0xff, 0xff, 0xff,    6, 0xff, 0xff, 0xff, ],
    [   0,    1,    2,    3,    4,    5, 0xff, 0xff,    6, 0xff, 0xff, 0xff,    7, 0xff, 0xff, 0xff, ],
    [   0, 0xff, 0xff, 0xff,    1,    2,    3, 0xff,    4, 0xff, 0xff, 0xff,    5, 0xff, 0xff, 0xff, ],
    [   0,    1, 0xff, 0xff,    2,    3,    4, 0xff,    5, 0xff, 0xff, 0xff,    6, 0xff, 0xff, 0xff, ],
    [   0,    1,    2, 0xff,    3,    4,    5, 0xff,    6, 0xff, 0xff, 0xff,    7, 0xff, 0xff, 0xff, ],
    [   0,    1,    2,    3,    4,    5,    6, 0xff,    7, 0xff, 0xff, 0xff,    8, 0xff, 0xff, 0xff, ],
    [   0, 0xff, 0xff, 0xff,    1,    2,    3,    4,    5, 0xff, 0xff, 0xff,    6, 0xff, 0xff, 0xff, ],
    [   0,    1, 0xff, 0xff,    2,    3,    4,    5,    6, 0xff, 0xff, 0xff,    7, 0xff, 0xff, 0xff, ],
    [   0,    1,    2, 0xff,    3,    4,    5,    6,    7, 0xff, 0xff, 0xff,    8, 0xff, 0xff, 0xff, ],
    [   0,    1,    2,    3,    4,    5,    6,    7,    8, 0xff, 0xff, 0xff,    9, 0xff, 0xff, 0xff, ],
    [   0, 0xff, 0xff, 0xff,    1, 0xff, 0xff, 0xff,    2,    3, 0xff, 0xff,    4, 0xff, 0xff, 0xff, ],
    [   0,    1, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3,    4, 0xff, 0xff,    5, 0xff, 0xff, 0xff, ],
    [   0,    1,    2, 0xff,    3, 0xff, 0xff, 0xff,    4,    5, 0xff, 0xff,    6, 0xff, 0xff, 0xff, ],
    [   0,    1,    2,    3,    4, 0xff, 0xff, 0xff,    5,    6, 0xff, 0xff,    7, 0xff, 0xff, 0xff, ],
    [   0, 0xff, 0xff, 0xff,    1,    2, 0xff, 0xff,    3,    4, 0xff, 0xff,    5, 0xff, 0xff, 0xff, ],
    [   0,    1, 0xff, 0xff,    2,    3, 0xff, 0xff,    4,    5, 0xff, 0xff,    6, 0xff, 0xff, 0xff, ],
    [   0,    1,    2, 0xff,    3,    4, 0xff, 0xff,    5,    6, 0xff, 0xff,    7, 0xff, 0xff, 0xff, ],
    [   0,    1,    2,    3,    4,    5, 0xff, 0xff,    6,    7, 0xff, 0xff,    8, 0xff, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3, 0xff,    4,    5, 0xff, 0xff,    6, 0xff, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4, 0xff,    5,    6, 0xff, 0xff,    7, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5, 0xff,    6,    7, 0xff, 0xff,    8, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6, 0xff,    7,    8, 0xff, 0xff,    9, 0xff, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3,    4,    5,    6, 0xff, 0xff,    7, 0xff, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4,    5,    6,    7, 0xff, 0xff,    8, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5,    6,    7,    8, 0xff, 0xff,    9, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9, 0xff, 0xff,   10, 0xff, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1, 0xff, 0xff, 0xff,    2,    3,    4, 0xff,    5, 0xff, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3,    4,    5, 0xff,    6, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3, 0xff, 0xff, 0xff,    4,    5,    6, 0xff,    7, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4, 0xff, 0xff, 0xff,    5,    6,    7, 0xff,    8, 0xff, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2, 0xff, 0xff,    3,    4,    5, 0xff,    6, 0xff, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3, 0xff, 0xff,    4,    5,    6, 0xff,    7, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4, 0xff, 0xff,    5,    6,    7, 0xff,    8, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5, 0xff, 0xff,    6,    7,    8, 0xff,    9, 0xff, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3, 0xff,    4,    5,    6, 0xff,    7, 0xff, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4, 0xff,    5,    6,    7, 0xff,    8, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5, 0xff,    6,    7,    8, 0xff,    9, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6, 0xff,    7,    8,    9, 0xff,   10, 0xff, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3,    4,    5,    6,    7, 0xff,    8, 0xff, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4,    5,    6,    7,    8, 0xff,    9, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5,    6,    7,    8,    9, 0xff,   10, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10, 0xff,   11, 0xff, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1, 0xff, 0xff, 0xff,    2,    3,    4,    5,    6, 0xff, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3,    4,    5,    6,    7, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3, 0xff, 0xff, 0xff,    4,    5,    6,    7,    8, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4, 0xff, 0xff, 0xff,    5,    6,    7,    8,    9, 0xff, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2, 0xff, 0xff,    3,    4,    5,    6,    7, 0xff, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3, 0xff, 0xff,    4,    5,    6,    7,    8, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4, 0xff, 0xff,    5,    6,    7,    8,    9, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5, 0xff, 0xff,    6,    7,    8,    9,   10, 0xff, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3, 0xff,    4,    5,    6,    7,    8, 0xff, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4, 0xff,    5,    6,    7,    8,    9, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5, 0xff,    6,    7,    8,    9,   10, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6, 0xff,    7,    8,    9,   10,   11, 0xff, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3,    4,    5,    6,    7,    8,    9, 0xff, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4,    5,    6,    7,    8,    9,   10, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5,    6,    7,    8,    9,   10,   11, 0xff, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10,   11,   12, 0xff, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1, 0xff, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3,    4, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3, 0xff, 0xff, 0xff,    4,    5, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3, 0xff, 0xff, 0xff,    4, 0xff, 0xff, 0xff,    5,    6, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4, 0xff, 0xff, 0xff,    5, 0xff, 0xff, 0xff,    6,    7, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2, 0xff, 0xff,    3, 0xff, 0xff, 0xff,    4,    5, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3, 0xff, 0xff,    4, 0xff, 0xff, 0xff,    5,    6, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4, 0xff, 0xff,    5, 0xff, 0xff, 0xff,    6,    7, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5, 0xff, 0xff,    6, 0xff, 0xff, 0xff,    7,    8, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3, 0xff,    4, 0xff, 0xff, 0xff,    5,    6, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4, 0xff,    5, 0xff, 0xff, 0xff,    6,    7, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5, 0xff,    6, 0xff, 0xff, 0xff,    7,    8, 0xff, 0xff, ],
    [   0,    1,    2,    3,    4,    5,    6, 0xff,    7, 0xff, 0xff, 0xff,    8,    9, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3,    4,    5, 0xff, 0xff, 0xff,    6,    7, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4,    5,    6, 0xff, 0xff, 0xff,    7,    8, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5,    6,    7, 0xff, 0xff, 0xff,    8,    9, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6,    7,    8, 0xff, 0xff, 0xff,    9,   10, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1, 0xff, 0xff, 0xff,    2,    3, 0xff, 0xff,    4,    5, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3,    4, 0xff, 0xff,    5,    6, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3, 0xff, 0xff, 0xff,    4,    5, 0xff, 0xff,    6,    7, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4, 0xff, 0xff, 0xff,    5,    6, 0xff, 0xff,    7,    8, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2, 0xff, 0xff,    3,    4, 0xff, 0xff,    5,    6, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3, 0xff, 0xff,    4,    5, 0xff, 0xff,    6,    7, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4, 0xff, 0xff,    5,    6, 0xff, 0xff,    7,    8, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5, 0xff, 0xff,    6,    7, 0xff, 0xff,    8,    9, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3, 0xff,    4,    5, 0xff, 0xff,    6,    7, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4, 0xff,    5,    6, 0xff, 0xff,    7,    8, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5, 0xff,    6,    7, 0xff, 0xff,    8,    9, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6, 0xff,    7,    8, 0xff, 0xff,    9,   10, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3,    4,    5,    6, 0xff, 0xff,    7,    8, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4,    5,    6,    7, 0xff, 0xff,    8,    9, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5,    6,    7,    8, 0xff, 0xff,    9,   10, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9, 0xff, 0xff,   10,   11, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1, 0xff, 0xff, 0xff,    2,    3,    4, 0xff,    5,    6, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3,    4,    5, 0xff,    6,    7, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3, 0xff, 0xff, 0xff,    4,    5,    6, 0xff,    7,    8, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4, 0xff, 0xff, 0xff,    5,    6,    7, 0xff,    8,    9, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2, 0xff, 0xff,    3,    4,    5, 0xff,    6,    7, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3, 0xff, 0xff,    4,    5,    6, 0xff,    7,    8, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4, 0xff, 0xff,    5,    6,    7, 0xff,    8,    9, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5, 0xff, 0xff,    6,    7,    8, 0xff,    9,   10, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3, 0xff,    4,    5,    6, 0xff,    7,    8, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4, 0xff,    5,    6,    7, 0xff,    8,    9, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5, 0xff,    6,    7,    8, 0xff,    9,   10, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6, 0xff,    7,    8,    9, 0xff,   10,   11, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3,    4,    5,    6,    7, 0xff,    8,    9, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4,    5,    6,    7,    8, 0xff,    9,   10, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5,    6,    7,    8,    9, 0xff,   10,   11, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10, 0xff,   11,   12, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1, 0xff, 0xff, 0xff,    2,    3,    4,    5,    6,    7, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3,    4,    5,    6,    7,    8, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3, 0xff, 0xff, 0xff,    4,    5,    6,    7,    8,    9, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4, 0xff, 0xff, 0xff,    5,    6,    7,    8,    9,   10, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2, 0xff, 0xff,    3,    4,    5,    6,    7,    8, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3, 0xff, 0xff,    4,    5,    6,    7,    8,    9, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4, 0xff, 0xff,    5,    6,    7,    8,    9,   10, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5, 0xff, 0xff,    6,    7,    8,    9,   10,   11, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3, 0xff,    4,    5,    6,    7,    8,    9, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4, 0xff,    5,    6,    7,    8,    9,   10, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5, 0xff,    6,    7,    8,    9,   10,   11, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6, 0xff,    7,    8,    9,   10,   11,   12, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10, 0xff, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4,    5,    6,    7,    8,    9,   10,   11, 0xff, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5,    6,    7,    8,    9,   10,   11,   12, 0xff, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13, 0xff, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1, 0xff, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3,    4,    5, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3, 0xff, 0xff, 0xff,    4,    5,    6, 0xff, ],  
    [   0,    1,    2, 0xff,    3, 0xff, 0xff, 0xff,    4, 0xff, 0xff, 0xff,    5,    6,    7, 0xff, ],  
    [   0,    1,    2,    3,    4, 0xff, 0xff, 0xff,    5, 0xff, 0xff, 0xff,    6,    7,    8, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2, 0xff, 0xff,    3, 0xff, 0xff, 0xff,    4,    5,    6, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3, 0xff, 0xff,    4, 0xff, 0xff, 0xff,    5,    6,    7, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4, 0xff, 0xff,    5, 0xff, 0xff, 0xff,    6,    7,    8, 0xff, ],  
    [   0,    1,    2,    3,    4,    5, 0xff, 0xff,    6, 0xff, 0xff, 0xff,    7,    8,    9, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3, 0xff,    4, 0xff, 0xff, 0xff,    5,    6,    7, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4, 0xff,    5, 0xff, 0xff, 0xff,    6,    7,    8, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5, 0xff,    6, 0xff, 0xff, 0xff,    7,    8,    9, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6, 0xff,    7, 0xff, 0xff, 0xff,    8,    9,   10, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3,    4,    5, 0xff, 0xff, 0xff,    6,    7,    8, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4,    5,    6, 0xff, 0xff, 0xff,    7,    8,    9, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5,    6,    7, 0xff, 0xff, 0xff,    8,    9,   10, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6,    7,    8, 0xff, 0xff, 0xff,    9,   10,   11, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1, 0xff, 0xff, 0xff,    2,    3, 0xff, 0xff,    4,    5,    6, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3,    4, 0xff, 0xff,    5,    6,    7, 0xff, ],  
    [   0,    1,    2, 0xff,    3, 0xff, 0xff, 0xff,    4,    5, 0xff, 0xff,    6,    7,    8, 0xff, ],  
    [   0,    1,    2,    3,    4, 0xff, 0xff, 0xff,    5,    6, 0xff, 0xff,    7,    8,    9, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2, 0xff, 0xff,    3,    4, 0xff, 0xff,    5,    6,    7, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3, 0xff, 0xff,    4,    5, 0xff, 0xff,    6,    7,    8, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4, 0xff, 0xff,    5,    6, 0xff, 0xff,    7,    8,    9, 0xff, ],  
    [   0,    1,    2,    3,    4,    5, 0xff, 0xff,    6,    7, 0xff, 0xff,    8,    9,   10, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3, 0xff,    4,    5, 0xff, 0xff,    6,    7,    8, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4, 0xff,    5,    6, 0xff, 0xff,    7,    8,    9, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5, 0xff,    6,    7, 0xff, 0xff,    8,    9,   10, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6, 0xff,    7,    8, 0xff, 0xff,    9,   10,   11, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3,    4,    5,    6, 0xff, 0xff,    7,    8,    9, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4,    5,    6,    7, 0xff, 0xff,    8,    9,   10, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5,    6,    7,    8, 0xff, 0xff,    9,   10,   11, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9, 0xff, 0xff,   10,   11,   12, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1, 0xff, 0xff, 0xff,    2,    3,    4, 0xff,    5,    6,    7, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3,    4,    5, 0xff,    6,    7,    8, 0xff, ],  
    [   0,    1,    2, 0xff,    3, 0xff, 0xff, 0xff,    4,    5,    6, 0xff,    7,    8,    9, 0xff, ],  
    [   0,    1,    2,    3,    4, 0xff, 0xff, 0xff,    5,    6,    7, 0xff,    8,    9,   10, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2, 0xff, 0xff,    3,    4,    5, 0xff,    6,    7,    8, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3, 0xff, 0xff,    4,    5,    6, 0xff,    7,    8,    9, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4, 0xff, 0xff,    5,    6,    7, 0xff,    8,    9,   10, 0xff, ],  
    [   0,    1,    2,    3,    4,    5, 0xff, 0xff,    6,    7,    8, 0xff,    9,   10,   11, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3, 0xff,    4,    5,    6, 0xff,    7,    8,    9, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4, 0xff,    5,    6,    7, 0xff,    8,    9,   10, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5, 0xff,    6,    7,    8, 0xff,    9,   10,   11, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6, 0xff,    7,    8,    9, 0xff,   10,   11,   12, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3,    4,    5,    6,    7, 0xff,    8,    9,   10, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4,    5,    6,    7,    8, 0xff,    9,   10,   11, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5,    6,    7,    8,    9, 0xff,   10,   11,   12, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10, 0xff,   11,   12,   13, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1, 0xff, 0xff, 0xff,    2,    3,    4,    5,    6,    7,    8, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3,    4,    5,    6,    7,    8,    9, 0xff, ],  
    [   0,    1,    2, 0xff,    3, 0xff, 0xff, 0xff,    4,    5,    6,    7,    8,    9,   10, 0xff, ],  
    [   0,    1,    2,    3,    4, 0xff, 0xff, 0xff,    5,    6,    7,    8,    9,   10,   11, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2, 0xff, 0xff,    3,    4,    5,    6,    7,    8,    9, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3, 0xff, 0xff,    4,    5,    6,    7,    8,    9,   10, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4, 0xff, 0xff,    5,    6,    7,    8,    9,   10,   11, 0xff, ],  
    [   0,    1,    2,    3,    4,    5, 0xff, 0xff,    6,    7,    8,    9,   10,   11,   12, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3, 0xff,    4,    5,    6,    7,    8,    9,   10, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4, 0xff,    5,    6,    7,    8,    9,   10,   11, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5, 0xff,    6,    7,    8,    9,   10,   11,   12, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6, 0xff,    7,    8,    9,   10,   11,   12,   13, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10,   11, 0xff, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4,    5,    6,    7,    8,    9,   10,   11,   12, 0xff, ],  
    [   0,    1,    2, 0xff,    3,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13, 0xff, ],  
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13,   14, 0xff, ],  
    [   0, 0xff, 0xff, 0xff,    1, 0xff, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3,    4,    5,    6, ],  
    [   0,    1, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3, 0xff, 0xff, 0xff,    4,    5,    6,    7, ],  
    [   0,    1,    2, 0xff,    3, 0xff, 0xff, 0xff,    4, 0xff, 0xff, 0xff,    5,    6,    7,    8, ],  
    [   0,    1,    2,    3,    4, 0xff, 0xff, 0xff,    5, 0xff, 0xff, 0xff,    6,    7,    8,    9, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2, 0xff, 0xff,    3, 0xff, 0xff, 0xff,    4,    5,    6,    7, ],  
    [   0,    1, 0xff, 0xff,    2,    3, 0xff, 0xff,    4, 0xff, 0xff, 0xff,    5,    6,    7,    8, ],  
    [   0,    1,    2, 0xff,    3,    4, 0xff, 0xff,    5, 0xff, 0xff, 0xff,    6,    7,    8,    9, ],  
    [   0,    1,    2,    3,    4,    5, 0xff, 0xff,    6, 0xff, 0xff, 0xff,    7,    8,    9,   10, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3, 0xff,    4, 0xff, 0xff, 0xff,    5,    6,    7,    8, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4, 0xff,    5, 0xff, 0xff, 0xff,    6,    7,    8,    9, ],  
    [   0,    1,    2, 0xff,    3,    4,    5, 0xff,    6, 0xff, 0xff, 0xff,    7,    8,    9,   10, ],  
    [   0,    1,    2,    3,    4,    5,    6, 0xff,    7, 0xff, 0xff, 0xff,    8,    9,   10,   11, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3,    4,    5, 0xff, 0xff, 0xff,    6,    7,    8,    9, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4,    5,    6, 0xff, 0xff, 0xff,    7,    8,    9,   10, ],  
    [   0,    1,    2, 0xff,    3,    4,    5,    6,    7, 0xff, 0xff, 0xff,    8,    9,   10,   11, ],  
    [   0,    1,    2,    3,    4,    5,    6,    7,    8, 0xff, 0xff, 0xff,    9,   10,   11,   12, ],  
    [   0, 0xff, 0xff, 0xff,    1, 0xff, 0xff, 0xff,    2,    3, 0xff, 0xff,    4,    5,    6,    7, ],  
    [   0,    1, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3,    4, 0xff, 0xff,    5,    6,    7,    8, ],  
    [   0,    1,    2, 0xff,    3, 0xff, 0xff, 0xff,    4,    5, 0xff, 0xff,    6,    7,    8,    9, ],  
    [   0,    1,    2,    3,    4, 0xff, 0xff, 0xff,    5,    6, 0xff, 0xff,    7,    8,    9,   10, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2, 0xff, 0xff,    3,    4, 0xff, 0xff,    5,    6,    7,    8, ],  
    [   0,    1, 0xff, 0xff,    2,    3, 0xff, 0xff,    4,    5, 0xff, 0xff,    6,    7,    8,    9, ],  
    [   0,    1,    2, 0xff,    3,    4, 0xff, 0xff,    5,    6, 0xff, 0xff,    7,    8,    9,   10, ],  
    [   0,    1,    2,    3,    4,    5, 0xff, 0xff,    6,    7, 0xff, 0xff,    8,    9,   10,   11, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3, 0xff,    4,    5, 0xff, 0xff,    6,    7,    8,    9, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4, 0xff,    5,    6, 0xff, 0xff,    7,    8,    9,   10, ],  
    [   0,    1,    2, 0xff,    3,    4,    5, 0xff,    6,    7, 0xff, 0xff,    8,    9,   10,   11, ],  
    [   0,    1,    2,    3,    4,    5,    6, 0xff,    7,    8, 0xff, 0xff,    9,   10,   11,   12, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3,    4,    5,    6, 0xff, 0xff,    7,    8,    9,   10, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4,    5,    6,    7, 0xff, 0xff,    8,    9,   10,   11, ],  
    [   0,    1,    2, 0xff,    3,    4,    5,    6,    7,    8, 0xff, 0xff,    9,   10,   11,   12, ],  
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9, 0xff, 0xff,   10,   11,   12,   13, ],
    [   0, 0xff, 0xff, 0xff,    1, 0xff, 0xff, 0xff,    2,    3,    4, 0xff,    5,    6,    7,    8, ],  
    [   0,    1, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3,    4,    5, 0xff,    6,    7,    8,    9, ],  
    [   0,    1,    2, 0xff,    3, 0xff, 0xff, 0xff,    4,    5,    6, 0xff,    7,    8,    9,   10, ],  
    [   0,    1,    2,    3,    4, 0xff, 0xff, 0xff,    5,    6,    7, 0xff,    8,    9,   10,   11, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2, 0xff, 0xff,    3,    4,    5, 0xff,    6,    7,    8,    9, ],  
    [   0,    1, 0xff, 0xff,    2,    3, 0xff, 0xff,    4,    5,    6, 0xff,    7,    8,    9,   10, ],  
    [   0,    1,    2, 0xff,    3,    4, 0xff, 0xff,    5,    6,    7, 0xff,    8,    9,   10,   11, ],  
    [   0,    1,    2,    3,    4,    5, 0xff, 0xff,    6,    7,    8, 0xff,    9,   10,   11,   12, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3, 0xff,    4,    5,    6, 0xff,    7,    8,    9,   10, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4, 0xff,    5,    6,    7, 0xff,    8,    9,   10,   11, ],  
    [   0,    1,    2, 0xff,    3,    4,    5, 0xff,    6,    7,    8, 0xff,    9,   10,   11,   12, ],  
    [   0,    1,    2,    3,    4,    5,    6, 0xff,    7,    8,    9, 0xff,   10,   11,   12,   13, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3,    4,    5,    6,    7, 0xff,    8,    9,   10,   11, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4,    5,    6,    7,    8, 0xff,    9,   10,   11,   12, ],  
    [   0,    1,    2, 0xff,    3,    4,    5,    6,    7,    8,    9, 0xff,   10,   11,   12,   13, ],  
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10, 0xff,   11,   12,   13,   14, ],  
    [   0, 0xff, 0xff, 0xff,    1, 0xff, 0xff, 0xff,    2,    3,    4,    5,    6,    7,    8,    9, ],  
    [   0,    1, 0xff, 0xff,    2, 0xff, 0xff, 0xff,    3,    4,    5,    6,    7,    8,    9,   10, ],  
    [   0,    1,    2, 0xff,    3, 0xff, 0xff, 0xff,    4,    5,    6,    7,    8,    9,   10,   11, ],  
    [   0,    1,    2,    3,    4, 0xff, 0xff, 0xff,    5,    6,    7,    8,    9,   10,   11,   12, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2, 0xff, 0xff,    3,    4,    5,    6,    7,    8,    9,   10, ],  
    [   0,    1, 0xff, 0xff,    2,    3, 0xff, 0xff,    4,    5,    6,    7,    8,    9,   10,   11, ],  
    [   0,    1,    2, 0xff,    3,    4, 0xff, 0xff,    5,    6,    7,    8,    9,   10,   11,   12, ],  
    [   0,    1,    2,    3,    4,    5, 0xff, 0xff,    6,    7,    8,    9,   10,   11,   12,   13, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3, 0xff,    4,    5,    6,    7,    8,    9,   10,   11, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4, 0xff,    5,    6,    7,    8,    9,   10,   11,   12, ],  
    [   0,    1,    2, 0xff,    3,    4,    5, 0xff,    6,    7,    8,    9,   10,   11,   12,   13, ],  
    [   0,    1,    2,    3,    4,    5,    6, 0xff,    7,    8,    9,   10,   11,   12,   13,   14, ],  
    [   0, 0xff, 0xff, 0xff,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10,   11,   12, ],  
    [   0,    1, 0xff, 0xff,    2,    3,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13, ],  
    [   0,    1,    2, 0xff,    3,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13,   14, ],  
    [   0,    1,    2,    3,    4,    5,    6,    7,    8,    9,   10,   11,   12,   13,   14,   15, ],  
];

#[cfg(target_arch = "x86-64")]
use std::arch::x86_64::{__m128i, _mm_loadu_si128, _mm_shuffle_epi8, _mm_storeu_si128};

#[cfg(target_arch = "x86-64")]
pub fn decode_ssse3(control: &[u8], data: &[u8]) -> Vec<u32> {
    let mut result = Vec::with_capacity(control.len() * 4);

    let mut in_ptr: *const u8 = data.as_ptr();
    //println!("in_ptr: {:?}", in_ptr);
    let mut out_ptr: *mut u32 = result.as_mut_ptr();

    for c in control {
        // Safe if source data has 12 extra bytes allocated (we always consume
        // at least 4 bytes).
        let encoded: __m128i = unsafe { _mm_loadu_si128(in_ptr as *const __m128i) };
        let shuf: *const [u8; 16] = &SHUFFLE_TABLE[*c as usize] as *const _;
        // Safe, because the types are compatible and we allow unaligned reads.
        let mask = unsafe { _mm_loadu_si128(shuf as *const __m128i) };
        //println!("{:x?} ({:#b})", mask, *c);
        let res = unsafe { _mm_shuffle_epi8(encoded, mask) };
        //println!("{:x?} ({:x?})", res, encoded);
        let bytes_consumed = LENGTH_TABLE[*c as usize];
        unsafe { in_ptr = in_ptr.add(bytes_consumed as usize) };
        //println!("consumed: {}, {:?}", bytes_consumed, in_ptr);
        unsafe { _mm_storeu_si128(out_ptr as *mut __m128i, res) };
        unsafe { out_ptr = out_ptr.add(4usize) };
    }

    unsafe { result.set_len(control.len() * 4) };
    result
}

#[cfg(test)]
mod tests {
    // use crate::{debug_print_control, decode, decode_ssse3};

    // use super::encode;
    // #[test]
    // fn encode_simple() {
    //     let input = [1024, 12, 10, 1_073_741_824, 1, 2, 3, 1024];
    //     let (control, data) = encode(&input);
    //     println!("{:x?}", control);
    //     debug_print_control(&control);
    //     println!("{:x?}", data);

    //     let values = decode(&control, &data);
    //     println!("{:?}", values);
    //     assert_eq!(values, input);
    // }

    // #[test]
    // fn decode_simd() {
    //     let input = [1024, 12, 10, 1_073_741_824, 1, 2, 3, 1024];
    //     let (control, data) = encode(&input);
    //     println!("{:x?}", data);
    //     let r = decode_ssse3(&control, &data);
    //     println!("{:?}", r);
    // }
}

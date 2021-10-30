use streamvb::{decode, encode};

fn main() {
    let numbers: Vec<u32> = (1..100).collect();
    let (len, bytes) = encode(&numbers);
    let _n = decode(len, &bytes);
    println!("xx");
}

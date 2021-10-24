use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::Rng;
use streamvbyte2::scalar::encode::encode;

#[inline]
fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

pub fn random_8bit(count: usize) -> Vec<u32> {
    let mut input: Vec<u32> = Vec::with_capacity(count);
    let mut rng = rand::thread_rng();
    for _ in 0..count {
        let b: u8 = rng.gen();
        input.push(b as u32);
    }
    input
}

pub fn random_16bit(count: usize) -> Vec<u32> {
    let mut input: Vec<u32> = Vec::with_capacity(count);
    let mut rng = rand::thread_rng();
    for _ in 0..count {
        let b: u16 = rng.gen();
        input.push(b as u32);
    }
    input
}

pub fn random_32bit(count: usize) -> Vec<u32> {
    let mut input: Vec<u32> = Vec::with_capacity(count);
    let mut rng = rand::thread_rng();
    for _ in 0..count {
        let b: u32 = rng.gen();
        input.push(b);
    }
    input
}

pub fn random_any_bit(count: usize) -> Vec<u32> {
    let mut input: Vec<u32> = Vec::with_capacity(count);
    let mut rng = rand::thread_rng();
    for _ in 0..count {
        let sz = rng.gen_range(1..5);
        let b = match sz {
            1 => rng.gen::<u8>() as u32,
            2 => rng.gen::<u16>() as u32,
            3 => rng.gen_range(0u32..16777216),
            4 => rng.gen::<u32>(),
            _ => panic!("impossible")
        };
        input.push(b);
    }
    input
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let input_8bit = random_8bit(8192);
    let input_16bit: Vec<u32> = random_16bit(8192);
    let input_32bit: Vec<u32> = random_32bit(8192);
    let input_any: Vec<u32> = random_any_bit(8192);
    let mut output: Vec<u32> = vec![0; 8192];

    c.bench_function("memcpy/8k*4B", |b| {
        b.iter(|| output.copy_from_slice(&input_8bit))
    });
    c.bench_function("scalar_encode/8bit/8k*4B", |b| {
        b.iter(|| {
            let _ = encode(&input_8bit);
        })
    });
    c.bench_function("scalar_encode/16bit/8k*4B", |b| {
        b.iter(|| {
            let _ = encode(&input_16bit);
        })
    });
    c.bench_function("scalar_encode/32bit/8k*4B", |b| {
        b.iter(|| {
            let _ = encode(&input_32bit);
        })
    });
    c.bench_function("scalar_encode/any-bit/8k*4B", |b| {
        b.iter(|| {
            let _ = encode(&input_any);
        })
    });

    //c.bench_function("fib 20", |b| b.iter(|| fibonacci(20)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

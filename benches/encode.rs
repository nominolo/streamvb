use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::Rng;
use streamvbyte2::scalar::{decode::decode, encode::encode};

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
            _ => panic!("impossible"),
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

    let (sz, encoded) = encode(&input_8bit);
    //println!("{}x4B => {}B", sz, encoded.len());
    c.bench_function("scalar_decode/8bit/8k*4B", |b| {
        b.iter(|| {
            let _ = decode(sz, &encoded);
        })
    });

    let (sz, encoded) = encode(&input_16bit);
    c.bench_function("scalar_decode/16bit/8k*4B", |b| {
        b.iter(|| {
            let _ = decode(sz, &encoded);
        })
    });

    let (sz, encoded) = encode(&input_32bit);
    c.bench_function("scalar_decode/32bit/8k*4B", |b| {
        b.iter(|| {
            let _ = decode(sz, &encoded);
        })
    });

    // TODO: We should probably hard-code a few data sets. The performance seems
    // to be highly variable. Probably, due to interactions with the branch
    // predictor.
    let (sz, encoded) = encode(&input_any);
    //println!("{}x4B => {}B", sz, encoded.len());
    c.bench_function("scalar_decode/any-bit/8k*4B", |b| {
        b.iter(|| {
            let _ = decode(sz, &encoded);
        })
    });

    let (sz, encoded) = encode(&input_8bit);
    //println!("{}x4B => {}B", sz, encoded.len());
    c.bench_function("simd_decode/8bit/8k*4B", |b| {
        b.iter(|| {
            let _ = streamvbyte2::x86_64::decode::decode_simd(sz, &encoded);
        })
    });

    let (sz, encoded) = encode(&input_16bit);
    //println!("{}x4B => {}B", sz, encoded.len());
    c.bench_function("simd_decode/16bit/8k*4B", |b| {
        b.iter(|| {
            let _ = streamvbyte2::x86_64::decode::decode_simd(sz, &encoded);
        })
    });

    let (sz, encoded) = encode(&input_16bit);
    //println!("{}x4B => {}B", sz, encoded.len());
    c.bench_function("simd_decode_unroll/16bit/8k*4B", |b| {
        b.iter(|| {
            let _ = streamvbyte2::x86_64::decode::decode_simd1(sz, &encoded);
        })
    });

    let (sz, encoded) = encode(&input_16bit);
    //println!("{}x4B => {}B", sz, encoded.len());
    c.bench_function("simd_decode_tr/16bit/8k*4B", |b| {
        b.iter(|| {
            let _ = unsafe { streamvbyte2::x86_64::decode::decode_simd_trusted_len(sz, &encoded) };
        })
    });

    let (sz, encoded) = encode(&input_32bit);
    //println!("{}x4B => {}B", sz, encoded.len());
    c.bench_function("simd_decode/32bit/8k*4B", |b| {
        b.iter(|| {
            let _ = streamvbyte2::x86_64::decode::decode_simd(sz, &encoded);
        })
    });

    let (sz, encoded) = encode(&input_any);
    //println!("{}x4B => {}B", sz, encoded.len());
    c.bench_function("simd_decode/any-bit/8k*4B", |b| {
        b.iter(|| {
            let _ = streamvbyte2::x86_64::decode::decode_simd(sz, &encoded);
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

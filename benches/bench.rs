use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use rand::Rng;
use streamvb::scalar::{decode, encode};

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

pub fn bench_memcpy(c: &mut Criterion) {
    let mut group = c.benchmark_group("memcpy");
    for power in 10..15 {
        let n = 1 << power;
        let input: Vec<u32> = random_any_bit(n);
        let mut output: Vec<u32> = vec![0; n];
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(format!("n={}k", n / 1024), &input, |b, input| {
            b.iter(|| output.copy_from_slice(input))
        });
    }
    group.finish();
}

pub fn bench_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_scalar");
    for power in 10..15 {
        let n = 1 << power;

        for (bitname, input) in [
            ("8bit", random_8bit(n)),
            ("16bit", random_16bit(n)),
            ("any-bit", random_any_bit(n)),
        ] {
            group.throughput(Throughput::Elements(n as u64));
            group.bench_with_input(
                format!("{}/n={}k", bitname, n / 1024),
                &input,
                |b, input| {
                    b.iter(|| {
                        let (_len, _bytes) = encode(input);
                    })
                },
            );
        }
    }
    group.finish();
}

#[allow(unused)]
pub fn bench_encode_simd(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_simd");
    for power in 10..15 {
        let n = 1 << power;

        #[cfg(any(
            all(target_arch = "aarch64", feature = "aarch64-simd"),
            all(
                any(target_arch = "x86", target_arch = "x86_64"),
                target_feature = "ssse3"
            )
        ))]
        for (bitname, input) in [("8bit", random_8bit(n)), ("any-bit", random_any_bit(n))] {
            let mut output: Vec<u8> = Vec::with_capacity(streamvb::max_compressed_len(input.len()));
            group.throughput(Throughput::Elements(n as u64));
            group.bench_with_input(
                format!("{}/n={}k", bitname, n / 1024),
                &input,
                |b, input| {
                    b.iter(|| {
                        let _len = streamvb::simd::encode_into(input, &mut output);
                        output.clear();
                    })
                },
            );
        }
    }
    group.finish();
}

#[allow(unused)]
pub fn bench_zigzag_encode_simd(c: &mut Criterion) {
    let mut group = c.benchmark_group("zigzag_encode_simd");
    for power in 10..15 {
        let n = 1 << power;

        #[cfg(any(
            all(target_arch = "aarch64", feature = "aarch64-simd"),
            all(
                any(target_arch = "x86", target_arch = "x86_64"),
                target_feature = "ssse3"
            )
        ))]
        for (bitname, input) in [("8bit", random_8bit(n)), ("any-bit", random_any_bit(n))] {
            let mut output: Vec<u8> = Vec::with_capacity(streamvb::max_compressed_len(input.len()));
            group.throughput(Throughput::Elements(n as u64));
            group.bench_with_input(
                format!("{}/n={}k", bitname, n / 1024),
                &input,
                |b, input| {
                    b.iter(|| {
                        let _len = streamvb::simd::zigzag_encode_into(input, &mut output);
                        output.clear();
                    })
                },
            );
        }
    }
    group.finish();
}

pub fn bench_decode_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_scalar");
    for power in 10..15 {
        let n = 1 << power;

        for (bitname, input) in [("8bit", random_8bit(n)), ("any-bit", random_any_bit(n))] {
            group.throughput(Throughput::Elements(n as u64));
            let (len, encoded) = encode(&input);
            group.bench_with_input(
                format!("{}/n={}k", bitname, n / 1024),
                &encoded,
                |b, encoded| {
                    b.iter(|| {
                        let _ = decode(len, encoded);
                    })
                },
            );
        }
    }
    group.finish();
}

#[allow(unused_variables)]
pub fn bench_decode_simd(c: &mut Criterion) {
    #[cfg(any(
        all(target_arch = "aarch64", feature = "aarch64-simd"),
        all(
            any(target_arch = "x86", target_arch = "x86_64"),
            target_feature = "ssse3"
        )
    ))]
    {
        let mut group = c.benchmark_group("decode_simd");
        for power in 10..15 {
            let n = 1 << power;

            for (bitname, input) in [("8bit", random_8bit(n)), ("any-bit", random_any_bit(n))] {
                group.throughput(Throughput::Elements(n as u64));
                let (len, encoded) = encode(&input);
                let mut output: Vec<u32> = Vec::with_capacity(len);
                group.bench_with_input(
                    format!("{}/n={}k", bitname, n / 1024),
                    &encoded,
                    |b, encoded| {
                        b.iter(|| {
                            let _ = streamvb::simd::decode_into(len, encoded, &mut output).unwrap();
                            output.clear();
                        })
                    },
                );
            }
        }
        group.finish();
    }
}

#[allow(unused_variables)]
pub fn bench_zigzag_decode_simd(c: &mut Criterion) {
    #[cfg(any(
        all(target_arch = "aarch64", feature = "aarch64-simd"),
        all(
            any(target_arch = "x86", target_arch = "x86_64"),
            target_feature = "ssse3"
        )
    ))]
    {
        let mut group = c.benchmark_group("zigzag_decode_simd");
        for power in 10..15 {
            let n = 1 << power;

            for (bitname, input) in [("8bit", random_8bit(n)), ("any-bit", random_any_bit(n))] {
                group.throughput(Throughput::Elements(n as u64));
                let (len, encoded) = encode(&input);
                let mut output: Vec<u32> = Vec::with_capacity(len);
                group.bench_with_input(
                    format!("{}/n={}k", bitname, n / 1024),
                    &encoded,
                    |b, encoded| {
                        b.iter(|| {
                            let _ = streamvb::simd::zigzag_decode_into(len, encoded, &mut output)
                                .unwrap();
                            output.clear();
                        })
                    },
                );
            }
        }
        group.finish();
    }
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let input_8bit = random_8bit(8192);
    let input_16bit: Vec<u32> = random_16bit(8192);
    let input_32bit: Vec<u32> = random_32bit(8192);
    let input_any: Vec<u32> = random_any_bit(8192);
    let mut output: Vec<u32> = vec![0; 8192];

    let mut group = c.benchmark_group("memcpy");
    for power in 10..15 {
        let n = 1 << power;
        let input: Vec<u32> = random_any_bit(n);
        let mut output: Vec<u32> = vec![0; n];
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(format!("n={}k", n / 1024), &input, |b, input| {
            b.iter(|| output.copy_from_slice(input))
        });
    }
    group.finish();

    //    let mut enc_group = c.benchmark_group("encode");

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
}

criterion_group!(
    benches,
    bench_memcpy,
    bench_encode,
    bench_encode_simd,
    bench_zigzag_encode_simd,
    bench_decode_scalar,
    bench_decode_simd,
    bench_zigzag_decode_simd,
);
criterion_main!(benches);

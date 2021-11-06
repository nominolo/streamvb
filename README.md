Unreleased. Work-in-progress.


# Benchmarks

```sh
RUSTFLAGS="-C target-cpu=native" cargo bench
```

Encoding and decoding of 4096 32bit numbers (16KiB). Both encoding and decoding
is a bit faster if all input values fit into 8 bits. We include memcpy as a
reference.

| Benchmark      | Platform                    | Elements/second |
|----------------|-----------------------------|----------------:|
| memcpy         | AMD Ryzen 9 3900X           |           16.8G |
| scalar_encode  | AMD Ryzen 9 3900X           |          ~0.96G |
| scalar_decode  | AMD Ryzen 9 3900X           |          ~1.44G |
| simd_encode    | AMD Ryzen 9 3900X           |     3.6G - 4.1G |
| simd_decode    | AMD Ryzen 9 3900X           |     5.8G - 6.0G |
| memcpy         | Apple M1 (Macbook Air 2020) |    9.5G - 15.8G |
| scalar_encode  | Apple M1 (Macbook Air 2020) |          ~1.37G |
| scalar_decode  | Apple M1 (Macbook Air 2020) |          ~1.38G |
| simd_encode    | Apple M1 (Macbook Air 2020) |     4.8G - 4.9G |
| simd_decode    | Apple M1 (Macbook Air 2020) |     6.5G - 6.7G |


Note: Decoder performs bounds checks. The original C version can read out of
bounds memory if you give it the wrong length parameter.

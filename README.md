Unreleased. Work-in-progress.


# Benchmarks

```sh
RUSTFLAGS="-C target-cpu=native" cargo bench
```

Encoding and decoding of 4096 32bit numbers (16KiB). Both encoding and decoding
is a bit faster if all input values fit into 8 bits. We include memcpy as a
reference.

| Benchmark            | Platform                    | Elements/second |
|----------------------|-----------------------------|----------------:|
| memcpy/n=4Ki (16KiB) | AMD Ryzen 9 3900X           |           16.8G |
| scalar_decode/n=4Ki  | AMD Ryzen 9 3900X           |          ~0.98G |
| simd_encode/n=4Ki    | AMD Ryzen 9 3900X           |     3.6G - 4.1G |
| simd_decode/n=4Ki    | AMD Ryzen 9 3900X           |     5.8G - 6.0G |
| memcpy/n=4Ki (16KiB) | Apple M1 (Macbook Air 2020) |            9.5G |
| scalar_encode/n=4Ki  | Apple M1 (Macbook Air 2020) |           1.39G |
| simd_decode/n=4Ki    | Apple M1 (Macbook Air 2020) |     6.5G - 6.7G |


Note: Decoder performs bounds checks. The original C version can read out of
bounds memory if you give it the wrong length parameter.

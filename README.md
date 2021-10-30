Unreleased. Work-in-progress.


# Benchmarks

```sh
RUSTFLAGS="-C target-cpu=native" cargo bench
```

Encoding and decoding of 4096 32bit numbers (16KiB). Both encoding and decoding
is a bit faster if all input values fit into 8 bits.

| Benchmark            | Platform          | Elements/second |
|----------------------|-------------------|----------------:|
| memcpy/n=4Ki (16KiB) | AMD Ryzen 9 3900X |           16.8G |
| simd_encode/n=4Ki    | AMD Ryzen 9 3900X |     3.6G - 4.1G |
| simd_decode/n=4Ki    | AMD Ryzen 9 3900X |     5.8G - 6.0G |

Note: Decoder performs bounds checks. The original C version can read out of
bounds memory if you give it the wrong length parameter.

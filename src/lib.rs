// SIMD on AArch64 requires nightly as of Rust 1.58
#![cfg_attr(feature = "aarch64-simd", feature(stdsimd))]
#![cfg_attr(feature = "aarch64-simd", feature(aarch64_target_feature))]

pub mod common;
pub mod scalar;
pub(crate) mod tables;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod x86_64;

#[cfg(all(target_arch = "aarch64", feature = "aarch64-simd"))]
pub mod aarch64;

pub mod simd;

#[cfg(test)]
pub mod safe;

pub use crate::common::StreamVbyteError;

pub fn encode(values: &[u32]) -> (usize, Vec<u8>) {
    crate::scalar::encode::encode(values)
}

pub fn decode(len: usize, input: &[u8]) -> Result<Vec<u32>, StreamVbyteError> {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        // println!("Using x86-64 simd");
        return crate::x86_64::decode::decode_simd1(len, input);
    }

    #[cfg(all(target_arch = "aarch64", feature = "aarch64-simd"))]
    {
        // println!("Using aarch64 simd");
        return crate::aarch64::decode::decode_simd(len, input);
    }

    #[cfg(not(any(
        all(target_arch = "aarch64", feature = "aarch64-simd"),
        any(target_arch = "x86", target_arch = "x86_64")
    )))]
    {
        // println!("Using scalar");
        crate::scalar::decode::decode(len, input)
    }
}

//! Version of the various encoding/decoding functions that are guaranteed to
//! use SIMD. Use these versions if you want guaranteed best performance and
//! get a compiler error otherwise.
//!
//! Note that the encoding functions from the main module will still use SIMD
//! if available.
use crate::common::StreamVbyteError;

#[allow(clippy::needless_return)]
pub fn decode(len: usize, input: &[u8]) -> Result<Vec<u32>, StreamVbyteError> {
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "ssse3"
    ))]
    {
        // println!("Using x86-64 simd");
        return crate::x86_64::decode::decode_simd::<crate::x86_64::decode::NoDecode>(len, input);
    }

    #[cfg(all(target_arch = "aarch64", feature = "aarch64-simd"))]
    {
        // println!("Using aarch64 simd");
        return crate::aarch64::decode::decode_simd(len, input);
    }
}

#[allow(clippy::needless_return)]
pub fn decode_into(
    len: usize,
    input: &[u8],
    output: &mut Vec<u32>,
) -> Result<(), StreamVbyteError> {
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "ssse3"
    ))]
    {
        // println!("Using x86-64 simd");
        return crate::x86_64::decode::decode_into_simd::<crate::x86_64::decode::NoDecode>(
            len, input, output,
        );
    }

    #[cfg(all(target_arch = "aarch64", feature = "aarch64-simd"))]
    {
        // println!("Using aarch64 simd");
        return crate::aarch64::decode::decode_simd(len, input);
    }
}

#[allow(clippy::needless_return)]
pub fn zigzag_decode_into(
    len: usize,
    input: &[u8],
    output: &mut Vec<u32>,
) -> Result<(), StreamVbyteError> {
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "ssse3"
    ))]
    {
        // println!("Using x86-64 simd");
        return crate::x86_64::decode::decode_into_simd::<crate::x86_64::decode::ZigZagDecode>(
            len, input, output,
        );
    }

    #[cfg(all(target_arch = "aarch64", feature = "aarch64-simd"))]
    {
        // println!("Using aarch64 simd");
        return crate::aarch64::decode::decode_simd(len, input);
    }
}

#[allow(clippy::needless_return)]
pub fn encode(input: &[u32]) -> (usize, Vec<u8>) {
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "ssse3"
    ))]
    {
        // println!("Using x86-64 simd");
        return crate::x86_64::encode::encode_simd::<crate::x86_64::encode::NoEncode>(input);
    }

    #[cfg(all(target_arch = "aarch64", feature = "aarch64-simd"))]
    {
        // println!("Using aarch64 simd");
        return crate::aarch64::encode::encode_simd(input);
    }
}

#[allow(clippy::needless_return)]
pub fn encode_into(input: &[u32], output: &mut Vec<u8>) -> usize {
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "ssse3"
    ))]
    {
        // println!("Using x86-64 simd");
        return crate::x86_64::encode::encode_into_simd::<crate::x86_64::encode::NoEncode>(
            input, output,
        );
    }

    #[cfg(all(target_arch = "aarch64", feature = "aarch64-simd"))]
    {
        // println!("Using aarch64 simd");
        return crate::aarch64::encode::encode_simd(input);
    }
}

#[allow(clippy::needless_return)]
pub fn zigzag_encode_into(input: &[u32], output: &mut Vec<u8>) -> usize {
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "ssse3"
    ))]
    {
        // println!("Using x86-64 simd");
        return crate::x86_64::encode::encode_into_simd::<crate::x86_64::encode::ZigZagEncode>(
            input, output,
        );
    }

    #[cfg(all(target_arch = "aarch64", feature = "aarch64-simd"))]
    {
        // println!("Using aarch64 simd");
        return crate::aarch64::encode::encode_simd(input);
    }
}

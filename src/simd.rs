//! Version of the various encoding/decoding functions that are guaranteed to
//! use SIMD. Use these versions if you want guaranteed best performance and
//! get a compiler error otherwise.
use multiversion::{multiversion, target};

use crate::common::StreamVbyteError;

#[allow(clippy::needless_return)]
pub fn decode(len: usize, input: &[u8]) -> Result<Vec<u32>, StreamVbyteError> {
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "ssse3"
    ))]
    {
        // println!("Using x86-64 simd");
        return crate::x86_64::decode::decode_simd1(len, input);
    }

    #[cfg(all(target_arch = "aarch64", feature = "aarch64-simd"))]
    {
        // println!("Using aarch64 simd");
        return crate::aarch64::decode::decode_simd(len, input);
    }
}

// SIMD on AArch64 requires nightly as of Rust 1.58
#![cfg_attr(feature = "aarch64-simd", feature(stdsimd))]
#![cfg_attr(feature = "aarch64-simd", feature(aarch64_target_feature))]
// #![feature(stdsimd)]
// #![feature(aarch64_target_feature)]

pub(crate) mod common;
pub mod scalar;
pub(crate) mod tables;

#[cfg(all(
    any(target_arch = "x86", target_arch = "x86_64"),
    target_feature = "ssse3"
))]
pub(crate) mod x86_64;

#[cfg(all(target_arch = "aarch64", feature = "aarch64-simd"))]
pub mod aarch64;

#[cfg(any(
    all(target_arch = "aarch64", feature = "aarch64-simd"),
    all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "ssse3"
    )
))]
pub mod simd;

#[cfg(test)]
pub mod safe;

pub use crate::common::{
    control_bytes_len, exact_compressed_len, max_compressed_len, StreamVbyteError,
};
//pub use crate::common::control_bytes_len

/// Encode a slice of `u32` values into a single byte vector.
///
/// Returns the size of the input vector and the encoded bytes. These two values
/// must be given to [decode] for correct decoding.
///
/// If the input values were all very small, the returned vector will have a lot
/// of leftover capacity. You can call
/// [shrink_to_fit][std::vec::Vec::shrink_to_fit] to try and return it to the
/// allocator.
///
/// ```
/// let (len, bytes) = streamvb::encode(&[0x11, 0x5544, 0x230021, 0xdeadbeef, 0x2142]);
/// assert_eq!(len, 5);
/// # println!("hex(bytes): {:x?}", bytes);
/// # #[rustfmt::skip]
/// assert_eq!(bytes, vec![
///     0b11_10_01_00, 0b00_00_00_01,
///     0x11,
///     0x44, 0x55,
///     0x21, 0x00, 0x23,
///     0xef, 0xbe, 0xad, 0xde,
///     0x42, 0x21
/// ]);
/// ```
#[allow(clippy::needless_return)]
pub fn encode(values: &[u32]) -> (usize, Vec<u8>) {
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "ssse3"
    ))]
    {
        // println!("Using x86-64 simd");
        return crate::x86_64::encode::encode_simd(values);
    }

    #[cfg(all(target_arch = "aarch64", feature = "aarch64-simd"))]
    {
        // println!("Using aarch64 simd");
        return crate::aarch64::encode::encode_simd(values);
    }
    #[cfg(not(any(
        all(target_arch = "aarch64", feature = "aarch64-simd"),
        all(
            any(target_arch = "x86", target_arch = "x86_64"),
            target_feature = "ssse3"
        )
    )))]
    {
        crate::scalar::encode::encode(values)
    }
}

/// Decode bytes encoded using [encode] into the original `u32` values.
///
/// Returns an error if the decoding process tried to read bytes outside of the
/// input slice.
///
/// The `len` value must match the length of the input to the [encode] call used
/// to produce the encoded bytes. If `len` is smaller, the decoded result will
/// likely be incorrect. If it is too long, you will likely get an error.
///
/// If successful, the resulting vector will contain exactly `len` elements.
///
/// ```
/// let values = vec![0x11, 0x5544, 0x230021, 0xdeadbeef, 0x2142];
/// let (len, bytes) = streamvb::encode(&values);
/// let decoded_values = streamvb::decode(len, &bytes).unwrap();
/// assert_eq!(values, decoded_values);
/// ```
#[allow(clippy::needless_return)]
pub fn decode(len: usize, input: &[u8]) -> Result<Vec<u32>, StreamVbyteError> {
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64"),
        target_feature = "ssse3"
    ))]
    {
        // println!("Using x86-64 simd");
        return crate::x86_64::decode::decode_simd(len, input);
    }

    #[cfg(all(target_arch = "aarch64", feature = "aarch64-simd"))]
    {
        // println!("Using aarch64 simd");
        return crate::aarch64::decode::decode_simd(len, input);
    }

    #[cfg(not(any(
        all(target_arch = "aarch64", feature = "aarch64-simd"),
        all(
            any(target_arch = "x86", target_arch = "x86_64"),
            target_feature = "ssse3"
        )
    )))]
    {
        // println!("Using scalar");
        crate::scalar::decode::decode(len, input)
    }
}

use multiversion::{multiversion, target};

use crate::common::StreamVbyteError;

/*
#[multiversion]
#[clone(target = "[x86|x86_64]+ssse3")]
#[clone(target = "[aarch64]+neon")]
#[allow(unreachable_code)]
pub fn decode(len: usize, input: &[u8]) -> Result<Vec<u32>, StreamVbyteError> {
    #[target_cfg(target = "[x86,x86_64]+ssse3")]
    return crate::x86_64::decode::decode_simd1(len, input);

    #[target_cfg(target = "[aarch64]+neon")]
    return crate::aarch64::decode::decode_simd(len, input);

    #[target_cfg(not(any(target = "[x86,x86_64]+ssse3", target = "[aarch64]+neon")))]
    crate::scalar::decode::decode(len, input)
}
*/

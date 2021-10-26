pub mod common;
pub mod scalar;
pub(crate) mod tables;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod x86_64;

#[cfg(test)]
pub mod safe;

//! Reviewed lifecycle recipes; execution belongs to the authorized host.
pub mod npm;
#[cfg(test)]
mod npm_tests;

pub mod native_primary;
#[cfg(test)]
mod native_primary_tests;

pub mod native_grok_mcode;
#[cfg(test)]
mod native_grok_mcode_tests;

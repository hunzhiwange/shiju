//! Source-independent skill marketplace providers.

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod install;
pub(crate) mod provider;
pub(crate) mod skillhub;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "install_tests.rs"]
mod install_tests;

#[cfg(test)]
#[path = "skillhub_tests.rs"]
mod skillhub_tests;

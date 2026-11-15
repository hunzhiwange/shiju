//! Scoped authorization flows and execution-host callback primitives.

pub mod flow;
#[cfg(not(target_arch = "wasm32"))]
pub mod browser_callback;
#[cfg(target_os = "macos")]
pub mod keychain;
#[cfg(all(test, target_os = "macos"))]
mod keychain_tests;

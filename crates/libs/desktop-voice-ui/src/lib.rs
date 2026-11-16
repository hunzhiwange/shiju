//! Voice interaction state and dictation execution, independent of the desktop host.
#[cfg(not(target_arch = "wasm32"))]
pub mod message;
#[cfg(not(target_arch = "wasm32"))]
pub mod components;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_module_33_tests;

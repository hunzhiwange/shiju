//! Voice interaction state and dictation execution, independent of the desktop host.
#[cfg(not(target_arch = "wasm32"))]
pub mod message;
#[cfg(not(target_arch = "wasm32"))]
pub mod components;

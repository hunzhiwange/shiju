//! Domain rules for application routing availability.

#[cfg(not(target_arch = "wasm32"))]
pub mod retry;

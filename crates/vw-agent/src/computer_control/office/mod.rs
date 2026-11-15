//! Office document identity and lifecycle. Registration does not grant control permission.
pub mod sessions;

#[cfg(test)]
mod sessions_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod transport;

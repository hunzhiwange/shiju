//! Production circuit health and bounded retry rules.

pub mod circuit_breaker;
pub mod health;

#[cfg(not(target_arch = "wasm32"))]
pub mod retry;

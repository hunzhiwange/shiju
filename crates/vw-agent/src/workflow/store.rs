//! Canonical workflow implementation.
#[cfg(any(test, target_arch = "wasm32"))]
pub use workflow_core::store::*;

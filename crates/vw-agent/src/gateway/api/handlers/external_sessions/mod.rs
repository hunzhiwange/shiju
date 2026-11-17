//! Compatibility exports for the external-session route owner.
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub use gateway_external_sessions::router;
#[cfg(any(test, target_arch = "wasm32"))]
pub use gateway_external_sessions::{router, usage_scan};

//! Compatibility imports for the single gateway raw commit registry.
#[cfg(any(test, target_arch = "wasm32"))]
pub(super) use gateway_session::raw_commit::{acknowledge, claim, register};

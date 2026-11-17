//! Compatibility export for skill auditing.

#[cfg(any(test, target_arch = "wasm32"))]
pub use agent_skills::audit::*;

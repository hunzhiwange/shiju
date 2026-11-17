//! Compatibility export for the skill domain.
#[cfg(any(test, target_arch = "wasm32"))]
pub use agent_skills::shiju::*;

//! ACP 访问策略、供应商能力判定与会话偏好。

pub mod access_policy;
pub mod codex_compat;
pub mod session_mode_preference;

pub use access_policy::{AccessPolicy, AccessPolicyReason, FullAccessSupport};

#[cfg(test)]
#[path = "codex_compat_tests.rs"]
mod codex_compat_tests;

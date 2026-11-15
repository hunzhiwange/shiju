//! Supplier login preparation for executor-owned provider transactions.

#[cfg(target_os = "macos")]
pub mod codex_login;
#[cfg(target_os = "macos")]
mod codex_login_material;
#[cfg(target_os = "macos")]
mod codex_login_policy;
#[cfg(all(test, target_os = "macos"))]
mod codex_login_tests;

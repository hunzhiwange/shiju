//! Native storage and transaction dependencies of account authorization.
pub use app_provider_validation::{validate_identity, validation};
pub use app_config_paths as paths;
pub use app_provider_core::basic;
pub(crate) use app_provider_projection::projection;
pub mod catalog;
pub mod live;
pub mod operations;
#[cfg(test)]
mod operations_tests;
pub mod claude;
#[cfg(test)]
mod claude_tests;
#[cfg(target_os = "macos")]
pub mod claude_login;
#[cfg(all(test, target_os = "macos"))]
mod claude_login_tests;
#[cfg(target_os = "macos")]
pub mod dsh;
#[cfg(all(test, target_os = "macos"))]
mod dsh_tests;
#[cfg(target_os = "macos")]
pub mod kimi;
#[cfg(all(test, target_os = "macos"))]
mod kimi_tests;
#[cfg(target_os = "macos")]
pub mod mode_state;
#[cfg(all(test, target_os = "macos"))]
mod mode_state_tests;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod codex_login_material {
    pub(crate) use app_auth_codex::identity::parse_codex_jwt_claims;
}

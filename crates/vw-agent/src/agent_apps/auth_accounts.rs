//! Compatibility exports for the application account service owner.
pub use agent_app_services::auth_accounts::*;

#[cfg(all(test, target_os = "macos"))]
#[path = "auth_accounts_tests.rs"]
mod tests;

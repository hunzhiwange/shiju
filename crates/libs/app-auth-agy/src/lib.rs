//! Scoped AGY authentication and native keychain accounts.

#[cfg(not(target_arch = "wasm32"))]
pub use app_auth_core::flow;
#[cfg(target_os = "macos")]
use app_auth_core::{browser_callback, keychain, logout, response};

#[cfg(not(target_arch = "wasm32"))]
pub mod agy_account;
#[cfg(target_os = "macos")]
pub mod agy_login;
#[cfg(target_os = "macos")]
pub mod agy_refresh;

#[cfg(all(test, target_os = "macos"))]
mod agy_account_tests;
#[cfg(all(test, target_os = "macos"))]
mod agy_login_tests;
#[cfg(all(test, target_os = "macos"))]
mod agy_refresh_tests;
#[cfg(all(test, target_os = "macos"))]
mod accounts_tests;
#[cfg(all(test, target_os = "macos"))]
mod logout_tests;
#[cfg(all(test, target_os = "macos"))]
mod test_support;

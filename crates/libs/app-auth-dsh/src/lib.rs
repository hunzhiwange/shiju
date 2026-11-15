//! DeepSeek Harness execution-host browser authorization.

#![cfg(target_os = "macos")]

pub mod dsh_login;
pub use app_auth_core::flow;
#[cfg(test)]
mod dsh_login_tests;
#[cfg(test)]
mod test_support;

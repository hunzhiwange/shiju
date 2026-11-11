//! 本机语音能力；不持有服务凭据。
pub mod permissions;

#[cfg(target_os = "macos")]
mod permissions_macos;

#[cfg(test)]
mod permissions_tests;

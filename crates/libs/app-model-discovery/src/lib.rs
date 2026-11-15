//! Authorized native runtime model discovery.
#![cfg(target_os = "macos")]
pub mod discovery_agy;
pub mod discovery_opencode;
mod discovery_process;
#[cfg(test)]
mod discovery_agy_tests;
#[cfg(test)]
mod discovery_opencode_tests;

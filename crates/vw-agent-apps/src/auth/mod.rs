//! Host-owned account authorization flows and credentials.
pub mod credential_file;

#[cfg(all(test, unix))]
mod credential_file_tests;

pub mod credential_lease;

#[cfg(all(test, unix))]
mod credential_lease_tests;

pub mod flow;

#[cfg(all(test, unix))]
mod flow_tests;

#[cfg(target_os = "macos")]
pub mod claude_keychain;
#[cfg(target_os = "macos")]
pub(crate) mod claude_login_files;

#[cfg(target_os = "macos")]
mod keychain;
#[cfg(all(test, target_os = "macos"))]
mod keychain_tests;

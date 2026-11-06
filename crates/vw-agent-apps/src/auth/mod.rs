//! Official account credentials. Separate owner from application/provider projections.
pub mod credential_file;

pub mod credential_lease;

#[cfg(all(test, unix))]
mod credential_lease_tests;

#[cfg(target_os = "macos")]
pub mod claude_keychain;
#[cfg(target_os = "macos")]
pub(crate) mod claude_login_files;

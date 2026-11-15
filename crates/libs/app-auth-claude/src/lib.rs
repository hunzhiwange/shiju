//! Claude keychain, protected login files and credential generation rules.

#[cfg(target_os = "macos")]
pub mod claude_keychain;
#[cfg(target_os = "macos")]
pub mod claude_login_files;
#[cfg(target_os = "macos")]
pub mod claude_rotation;

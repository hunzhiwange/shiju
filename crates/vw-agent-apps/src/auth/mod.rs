//! Host-owned account authorization flows and credentials.

pub mod accounts;
#[cfg(all(test, target_os = "macos"))]
mod accounts_tests;
#[cfg(not(target_arch = "wasm32"))]
mod browser_callback;

#[cfg(not(target_arch = "wasm32"))]
pub mod codex_browser;

#[cfg(all(test, unix, not(target_arch = "wasm32")))]
mod codex_browser_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod codex_device;

#[cfg(all(test, unix, not(target_arch = "wasm32")))]
mod codex_device_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod codex_refresh;

#[cfg(all(test, unix, not(target_arch = "wasm32")))]
mod codex_refresh_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod copilot;

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) mod copilot_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod copilot_token;

#[cfg(all(test, unix, not(target_arch = "wasm32")))]
mod copilot_token_tests;

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
pub mod claude_login;
#[cfg(target_os = "macos")]
pub(crate) mod claude_login_files;
#[cfg(all(test, target_os = "macos"))]
mod claude_login_tests;
#[cfg(all(test, target_os = "macos"))]
mod claude_recover_tests;
#[cfg(target_os = "macos")]
pub mod claude_rotation;
#[cfg(all(test, target_os = "macos"))]
mod claude_rotation_tests;

#[cfg(target_os = "macos")]
mod keychain;
#[cfg(all(test, target_os = "macos"))]
mod keychain_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod xai;

#[cfg(all(test, unix, not(target_arch = "wasm32")))]
mod xai_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod xai_refresh;

#[cfg(all(test, unix, not(target_arch = "wasm32")))]
mod xai_refresh_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod kimi;

#[cfg(all(test, unix, not(target_arch = "wasm32")))]
mod kimi_tests;

#[cfg(target_os = "macos")]
pub mod kimi_account;

#[cfg(all(test, target_os = "macos"))]
pub(crate) mod kimi_account_tests;

#[cfg(all(test, target_os = "macos"))]
mod claude_snapshot_tests;

#[cfg(target_os = "macos")]
pub mod agy_login;
#[cfg(all(test, target_os = "macos"))]
mod agy_login_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod agy_account;
#[cfg(all(test, target_os = "macos"))]
mod agy_account_tests;

#[cfg(target_os = "macos")]
pub mod agy_refresh;
#[cfg(all(test, target_os = "macos"))]
mod agy_refresh_tests;

#[cfg(target_os = "macos")]
pub mod dsh_login;
#[cfg(all(test, target_os = "macos"))]
mod dsh_login_tests;

#[cfg(target_os = "macos")]
pub mod dsh_account;
#[cfg(all(test, target_os = "macos"))]
mod dsh_account_tests;

#[cfg(target_os = "macos")]
pub mod logout;
#[cfg(all(test, target_os = "macos"))]
mod logout_tests;

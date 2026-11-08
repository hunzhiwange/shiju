//! Host-owned account authorization flows and credentials.
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
mod copilot_tests;

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
mod kimi_account_tests;

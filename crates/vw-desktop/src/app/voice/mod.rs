//! 本机语音能力；不持有服务凭据。
pub mod permissions;

#[cfg(target_os = "macos")]
mod permissions_macos;

#[cfg(test)]
mod permissions_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod capture;
#[cfg(not(target_arch = "wasm32"))]
mod capture_native;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod capture_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod playback;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod playback_tests;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod conversation;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod preferences;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod preview;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod preview_tests;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod recordings;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod recordings_playback;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod hotkeys;

#[cfg(target_os = "macos")]
mod hotkeys_macos;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod insertion;
#[cfg(target_os = "macos")]
mod insertion_macos;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod screen_context;
#[cfg(target_os = "macos")]
mod screen_context_macos;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod acceptance_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) mod acceptance_fixture;

#[cfg(not(target_arch = "wasm32"))]
pub mod local;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod local_tests;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod local_conversation;

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

//! 本机语音能力；不持有服务凭据。
pub mod permissions;

#[cfg(target_os = "macos")]
mod permissions_macos;

#[cfg(not(target_arch = "wasm32"))]
pub mod capture;
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../../../../libs/audio-capture/src/capture_native.rs"]
mod capture_native;

#[cfg(not(target_arch = "wasm32"))]
pub mod playback;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod conversation;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod preferences;
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) mod preview;
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../../../../libs/desktop-voice/src/preview.rs"]
pub(crate) mod preview;


#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod recordings;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod recordings_playback;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod hotkeys;


#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod insertion;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod screen_context;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod acceptance_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) mod acceptance_fixture;

#[cfg(not(target_arch = "wasm32"))]
pub mod local;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod local_conversation;

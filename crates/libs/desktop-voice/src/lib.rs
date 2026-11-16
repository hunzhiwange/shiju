//! Native voice interaction, shortcuts and insertion safety.
#[cfg(not(target_arch = "wasm32"))]
pub mod conversation;
#[cfg(not(target_arch = "wasm32"))]
pub mod hotkeys;
#[cfg(not(target_arch = "wasm32"))]
pub mod insertion;
#[cfg(not(target_arch = "wasm32"))]
pub mod screen_context;
#[cfg(not(target_arch = "wasm32"))]
mod capture;
#[cfg(not(target_arch = "wasm32"))]
mod playback;
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../../audio-capture/src/capture_native.rs"]
mod capture_native;
#[cfg(target_os = "macos")]
mod hotkeys_macos;
#[cfg(target_os = "macos")]
mod insertion_macos;
pub use voice_permissions as permissions;
#[cfg(all(feature = "acceptance-fixture", not(target_arch = "wasm32")))]
pub mod acceptance_fixture;

#[cfg(not(target_arch = "wasm32"))]
pub mod local;
#[cfg(not(target_arch = "wasm32"))]
pub mod preferences;
#[cfg(not(target_arch = "wasm32"))]
pub mod preview;
#[cfg(not(target_arch = "wasm32"))]
pub mod local_conversation;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod local_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod preview_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod recordings_playback;
#[cfg(target_os = "macos")]
mod screen_context_macos;

#[cfg(all(test, feature = "acceptance-fixture", not(target_arch = "wasm32")))]
mod acceptance_tests;

#[cfg(test)]
mod desktop_split_18_tests;

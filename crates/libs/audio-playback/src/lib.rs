//! Bounded native PCM playback backend.
#[cfg(not(target_arch = "wasm32"))]
mod playback;

#[cfg(not(target_arch = "wasm32"))]
pub use playback::{Playback, PlaybackError};

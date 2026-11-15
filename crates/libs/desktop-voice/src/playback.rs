//! Desktop facade for the native playback backend.
#[cfg(not(test))]
pub use audio_playback::{Playback, PlaybackError};

// Desktop worker tests need the private in-memory callbacks. Compile the same
// backend source in their test target without exposing fixtures in the library API.
#[cfg(test)]
#[path = "../../audio-playback/src/playback.rs"]
mod test_backend;
#[cfg(test)]
pub use test_backend::{Playback, PlaybackError};
#[cfg(test)]
pub(super) use test_backend::{fixture, fixture_stream};

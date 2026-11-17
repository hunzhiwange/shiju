//! Desktop facade for the native capture backend.
#[cfg(test)]
use super::capture;
#[cfg(not(test))]
pub use audio_capture::{
    Capture, CaptureError, FRAME_SAMPLES, InputDevice, LiveCapture, LiveFrames, MAX_SECONDS,
    Recording, SAMPLE_RATE, VolumeSummary, input_devices,
};

// Desktop worker tests share the backend source to retain private device fixtures.
// Production uses the library; no fixture API is exported across crate boundaries.
#[cfg(test)]
#[path = "../../../../libs/audio-capture/src/capture.rs"]
mod test_backend;
#[cfg(test)]
pub use test_backend::{
    Capture, CaptureError, FRAME_SAMPLES, InputDevice, LiveCapture, LiveFrames, MAX_SECONDS,
    Recording, SAMPLE_RATE, VolumeSummary, input_devices,
};
#[cfg(test)]
pub(super) use test_backend::{CaptureBackend, CaptureSink, PcmConverter, device_lease};

#[cfg(test)]
#[path = "../../../../libs/audio-capture/src/capture_native.rs"]
mod capture_native;

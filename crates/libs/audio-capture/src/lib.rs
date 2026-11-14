//! Bounded native microphone capture with a shared device lease.
#[cfg(not(target_arch = "wasm32"))]
mod capture;
#[cfg(not(target_arch = "wasm32"))]
mod capture_native;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod capture_tests;

#[cfg(not(target_arch = "wasm32"))]
pub use capture::{
    Capture, CaptureError, FRAME_SAMPLES, InputDevice, LiveCapture, LiveFrames, MAX_SECONDS,
    Recording, SAMPLE_RATE, VolumeSummary, input_devices,
};

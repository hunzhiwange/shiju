//! ShiJu v1 voice DTOs. Parsing checks shape and bounds; owners, sequence ordering and
//! generation transitions must additionally be checked against the live connection state.

mod identity;
mod payload;
mod session;
mod event;
mod error;
mod validation;

pub use identity::{MAX_SAFE_INTEGER, VoiceCounter, VoiceId};
pub use payload::{AudioEncoding, AudioFormat, AudioPayload, MAX_AUDIO_BYTES, MAX_AUDIO_SAMPLES, MAX_SCREEN_BYTES, MAX_SCREEN_EDGE, ScreenPayload};
pub use session::{MAX_RECORDING_MS, MAX_SESSION_MS, MAX_WAV_BYTES, TranscriptionRequest, TranscriptionResponse, TurnMode, VoiceName, VoiceSessionStart, VoiceSessionStarted};
pub use event::{CloseOutcome, CloseReason, GenerationReason, HandoffStatus, MAX_FRAME_BYTES, MAX_METADATA_BYTES, ResponseStatus, TranscriptRole, VoiceEvent, VoiceFrame};
pub use error::{VoiceError, VoiceErrorCode, VoiceErrorResponse};

#[cfg(test)]
mod voice_tests;

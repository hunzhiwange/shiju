//! Compatibility exports for the voice protocol.
pub use vw_api_voice::{
    AudioEncoding, AudioFormat, AudioPayload, CloseOutcome, CloseReason, GenerationReason,
    HandoffStatus, MAX_AUDIO_BYTES, MAX_AUDIO_SAMPLES, MAX_FRAME_BYTES, MAX_METADATA_BYTES,
    MAX_RECORDING_MS, MAX_SAFE_INTEGER, MAX_SCREEN_BYTES, MAX_SCREEN_EDGE, MAX_SESSION_MS,
    MAX_WAV_BYTES, ResponseStatus, ScreenPayload, TranscriptRole, TranscriptionRequest,
    TranscriptionResponse, TurnMode, VoiceCounter, VoiceError, VoiceErrorCode, VoiceErrorResponse,
    VoiceEvent, VoiceFrame, VoiceId, VoiceName, VoiceSessionStart, VoiceSessionStarted,
};

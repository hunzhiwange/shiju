use serde::{Deserialize, Serialize};
use crate::{AudioFormat, VoiceCounter, VoiceId};
use crate::validation::text_64k;

pub const MAX_WAV_BYTES: usize = 24_000_000;
pub const MAX_RECORDING_MS: u64 = 180_000;
pub const MAX_SESSION_MS: u64 = 1_800_000;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptionRequest {
    pub request_id: VoiceId,
    pub chat_session_id: VoiceId,
    pub generation: VoiceCounter,
    /// Existing gateway provider configuration reference, never a service key or URL.
    pub provider_id: VoiceId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptionResponse {
    pub request_id: VoiceId,
    pub chat_session_id: VoiceId,
    pub generation: VoiceCounter,
    /// Final complete text; safe to fill only the matching active draft generation.
    #[serde(deserialize_with = "text_64k")]
    pub text: String,
    /// May be empty or contain codes outside the configured recognition language subset.
    pub detected_languages: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnMode {
    Manual,
    ServerVad,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceSessionStart {
    pub request_id: VoiceId,
    pub chat_session_id: VoiceId,
    pub generation: VoiceCounter,
    pub provider_id: VoiceId,
    pub turn_mode: TurnMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceName {
    Alloy,
    Ash,
    Ballad,
    Coral,
    Echo,
    Sage,
    Shimmer,
    Verse,
    Marin,
    Cedar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceSessionStarted {
    pub request_id: VoiceId,
    pub chat_session_id: VoiceId,
    pub generation: VoiceCounter,
    pub voice_session_id: VoiceId,
    /// Same-origin authenticated route; the client must also match this path to the session ID.
    pub stream_path: String,
    pub format: AudioFormat,
    pub voice: VoiceName,
    #[serde(deserialize_with = "session_duration")]
    pub expires_in_ms: u64,
}

fn session_duration<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    let ms = u64::deserialize(d)?;
    if ms == 0 || ms > MAX_SESSION_MS {
        return Err(serde::de::Error::custom("invalid voice session duration"));
    }
    Ok(ms)
}

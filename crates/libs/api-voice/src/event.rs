use serde::{Deserialize, Serialize};
use crate::{AudioFormat, AudioPayload, ScreenPayload, VoiceCounter, VoiceError, VoiceId, VoiceName, MAX_SESSION_MS};
use crate::payload::validate_audio;
use crate::validation::{text_16k, text_64k};

pub const MAX_FRAME_BYTES: usize = 64 * 1024;
pub const MAX_METADATA_BYTES: usize = 8 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptRole {
    User,
    Assistant,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Completed,
    Cancelled,
    Failed,
    Incomplete,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffStatus {
    Accepted,
    AwaitingApproval,
    Running,
    Completed,
    Failed,
    Cancelled,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationReason {
    Interrupt,
    Mute,
    Vad,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseOutcome {
    Ended,
    Failed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseReason {
    UserStop,
    Timeout,
    Cancelled,
    ProtocolError,
    Backpressure,
    ProviderUnavailable,
}

/// Adjacently tagged payloads prevent subtitle deltas from masquerading as final text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", deny_unknown_fields)]
pub enum VoiceEvent {
    #[serde(rename = "screen.share")]
    ScreenShare(ScreenPayload),
    #[serde(rename = "audio.append")]
    AudioAppend(AudioPayload),
    #[serde(rename = "input.commit")]
    InputCommit {},
    #[serde(rename = "input.mute")]
    InputMute { muted: bool },
    #[serde(rename = "response.interrupt")]
    ResponseInterrupt {
        response_id: VoiceId,
        item_id: Option<VoiceId>,
        content_index: Option<u32>,
        audio_end_ms: Option<u64>,
    },
    #[serde(rename = "playback.progress")]
    PlaybackProgress {
        response_id: VoiceId,
        item_id: VoiceId,
        content_index: u32,
        audio_end_ms: u64,
    },
    #[serde(rename = "session.stop")]
    SessionStop {},
    #[serde(rename = "session.ready")]
    SessionReady { format: AudioFormat, voice: VoiceName },
    #[serde(rename = "generation.changed")]
    GenerationChanged {
        reason: GenerationReason,
        response_id: Option<VoiceId>,
        item_id: Option<VoiceId>,
        content_index: Option<u32>,
    },
    #[serde(rename = "audio.delta")]
    AudioDelta {
        response_id: VoiceId,
        item_id: VoiceId,
        content_index: u32,
        audio: String,
        samples_per_channel: usize,
    },
    #[serde(rename = "audio.done")]
    AudioDone { response_id: VoiceId, item_id: VoiceId, content_index: u32 },
    #[serde(rename = "transcript.delta")]
    TranscriptDelta {
        role: TranscriptRole,
        item_id: VoiceId,
        content_index: u32,
        #[serde(deserialize_with = "text_64k")]
        delta: String,
    },
    #[serde(rename = "transcript.final")]
    TranscriptFinal {
        role: TranscriptRole,
        item_id: VoiceId,
        content_index: u32,
        #[serde(deserialize_with = "text_64k")]
        text: String,
    },
    #[serde(rename = "response.started")]
    ResponseStarted { response_id: VoiceId },
    #[serde(rename = "response.done")]
    ResponseDone { response_id: VoiceId, status: ResponseStatus },
    #[serde(rename = "handoff.status")]
    HandoffStatus {
        call_id: VoiceId,
        status: HandoffStatus,
        #[serde(deserialize_with = "text_16k")]
        text: String,
    },
    #[serde(rename = "error")]
    Error(VoiceError),
    #[serde(rename = "session.closed")]
    SessionClosed { outcome: CloseOutcome, reason: CloseReason },
}
impl VoiceEvent {
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::ScreenShare(screen) => screen.bytes().map(|_| ()),
            Self::AudioAppend(audio) => validate_audio(&audio.audio, audio.samples_per_channel),
            Self::AudioDelta { audio, samples_per_channel, .. } => {
                validate_audio(audio, *samples_per_channel)
            }
            Self::ResponseInterrupt { item_id, content_index, audio_end_ms, .. } => {
                if item_id.is_some() != content_index.is_some()
                    || item_id.is_some() != audio_end_ms.is_some()
                    || audio_end_ms.is_some_and(|ms| ms > MAX_SESSION_MS)
                {
                    return Err("invalid voice interruption position");
                }
                Ok(())
            }
            Self::PlaybackProgress { audio_end_ms, .. } if *audio_end_ms > MAX_SESSION_MS => {
                Err("invalid voice playback position")
            }
            Self::GenerationChanged { item_id, content_index, .. }
                if item_id.is_some() != content_index.is_some() =>
            {
                Err("invalid voice generation association")
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VoiceFrame {
    pub voice_session_id: VoiceId,
    pub generation: VoiceCounter,
    pub seq: VoiceCounter,
    #[serde(flatten)]
    pub event: VoiceEvent,
}
impl VoiceFrame {
    /// Enforce complete decoded WebSocket message size BEFORE JSON allocates payloads.
    /// WebSocket transport must independently cap frames/messages and disable compression.
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > MAX_FRAME_BYTES {
            return Err("voice message exceeds size limit");
        }
        serde_json::from_slice(bytes).map_err(|_| "invalid voice frame")
    }
}
impl<'de> Deserialize<'de> for VoiceFrame {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireFrame {
            voice_session_id: VoiceId,
            generation: VoiceCounter,
            seq: VoiceCounter,
            #[serde(rename = "type")]
            kind: String,
            payload: serde_json::Value,
        }
        let wire = WireFrame::deserialize(d)?;
        let event: VoiceEvent =
            serde_json::from_value(serde_json::json!({"type": wire.kind, "payload": wire.payload}))
                .map_err(serde::de::Error::custom)?;
        event.validate().map_err(serde::de::Error::custom)?;
        Ok(Self {
            voice_session_id: wire.voice_session_id,
            generation: wire.generation,
            seq: wire.seq,
            event,
        })
    }
}

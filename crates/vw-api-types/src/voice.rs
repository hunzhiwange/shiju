//! ShiJu v1 voice DTOs. Parsing checks shape and bounds; owners, sequence ordering and
//! generation transitions must additionally be checked against the live connection state.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

pub const MAX_AUDIO_SAMPLES: usize = 2400;
pub const MAX_AUDIO_BYTES: usize = 4800;
pub const MAX_FRAME_BYTES: usize = 64 * 1024;
pub const MAX_METADATA_BYTES: usize = 8 * 1024;
pub const MAX_WAV_BYTES: usize = 24_000_000;
pub const MAX_RECORDING_MS: u64 = 180_000;
pub const MAX_SESSION_MS: u64 = 1_800_000;
pub const MAX_SAFE_INTEGER: u64 = (1 << 53) - 1;

/// Opaque ASCII identifier (1–128 printable bytes). Never a URL, credential or device name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct VoiceId(String);
impl TryFrom<String> for VoiceId {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() || value.len() > 128 || !value.bytes().all(|b| b.is_ascii_graphic()) {
            return Err("invalid voice identifier");
        }
        Ok(Self(value))
    }
}
impl From<VoiceId> for String {
    fn from(id: VoiceId) -> Self {
        id.0
    }
}
impl AsRef<str> for VoiceId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Both generation and per-direction sequence start at one and fit exactly in JSON clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct VoiceCounter(u64);
impl VoiceCounter {
    pub const FIRST: Self = Self(1);
    pub fn get(self) -> u64 {
        self.0
    }
    pub fn checked_next(self) -> Result<Self, &'static str> {
        Self::try_from(self.0 + 1)
    }
}
impl TryFrom<u64> for VoiceCounter {
    type Error = &'static str;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if !(1..=MAX_SAFE_INTEGER).contains(&value) {
            return Err("invalid voice counter");
        }
        Ok(Self(value))
    }
}
impl From<VoiceCounter> for u64 {
    fn from(value: VoiceCounter) -> Self {
        value.0
    }
}

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

/// The sole negotiated media format in v1: raw little-endian PCM16, mono, 24 kHz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioFormat {
    pub encoding: AudioEncoding,
    #[serde(deserialize_with = "sample_rate_24k")]
    pub sample_rate: u32,
    #[serde(deserialize_with = "mono_channels")]
    pub channels: u8,
}
impl Default for AudioFormat {
    fn default() -> Self {
        Self { encoding: AudioEncoding::PcmS16le, sample_rate: 24000, channels: 1 }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioEncoding {
    PcmS16le,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "UncheckedAudio")]
pub struct AudioPayload {
    pub audio: String,
    pub samples_per_channel: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UncheckedAudio {
    audio: String,
    samples_per_channel: usize,
}
impl TryFrom<UncheckedAudio> for AudioPayload {
    type Error = &'static str;
    fn try_from(raw: UncheckedAudio) -> Result<Self, Self::Error> {
        validate_audio(&raw.audio, raw.samples_per_channel)?;
        Ok(Self { audio: raw.audio, samples_per_channel: raw.samples_per_channel })
    }
}
fn validate_audio(audio: &str, samples: usize) -> Result<(), &'static str> {
    if samples == 0 || samples > MAX_AUDIO_SAMPLES || audio.len() > MAX_AUDIO_BYTES / 3 * 4 {
        return Err("voice audio frame exceeds sample or byte limit");
    }
    let bytes = STANDARD.decode(audio).map_err(|_| "invalid voice audio base64")?;
    if bytes.len() != samples * 2 {
        return Err("voice audio sample count mismatch");
    }
    Ok(())
}

/// One in-memory PNG, with no URLs, OCR instructions or permission changes.
pub const MAX_SCREEN_BYTES: usize = 40 * 1024;
pub const MAX_SCREEN_EDGE: u32 = 1024;
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScreenPayload {
    pub png: String,
}
impl std::fmt::Debug for ScreenPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScreenPayload([redacted])")
    }
}
impl ScreenPayload {
    pub fn bytes(&self) -> Result<Vec<u8>, &'static str> {
        if self.png.len() > MAX_SCREEN_BYTES.div_ceil(3) * 4 {
            return Err("screen frame exceeds byte limit");
        }
        let bytes = STANDARD.decode(&self.png).map_err(|_| "invalid screen base64")?;
        if bytes.len() < 33
            || bytes.len() > MAX_SCREEN_BYTES
            || &bytes[..8] != b"\x89PNG\r\n\x1a\n"
            || &bytes[8..16] != b"\0\0\0\rIHDR"
        {
            return Err("invalid screen PNG");
        }
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        if width == 0 || height == 0 || width > MAX_SCREEN_EDGE || height > MAX_SCREEN_EDGE {
            return Err("screen dimensions exceed limit");
        }
        Ok(bytes)
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceErrorCode {
    InvalidRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    UnsupportedMedia,
    PayloadTooLarge,
    UnsupportedCapability,
    ProviderAuth,
    RateLimited,
    ProviderUnavailable,
    Timeout,
    Cancelled,
    ProtocolError,
    Backpressure,
}
impl VoiceErrorCode {
    pub fn http_status(self) -> u16 {
        match self {
            Self::InvalidRequest => 400,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::Conflict | Self::Cancelled => 409,
            Self::UnsupportedMedia => 415,
            Self::PayloadTooLarge => 413,
            Self::UnsupportedCapability => 422,
            Self::RateLimited | Self::Backpressure => 429,
            Self::Timeout => 504,
            Self::ProviderAuth | Self::ProviderUnavailable | Self::ProtocolError => 502,
        }
    }
    pub fn retryable(self) -> bool {
        matches!(self, Self::RateLimited | Self::ProviderUnavailable | Self::Timeout)
    }
    pub fn message(self) -> &'static str {
        match self {
            Self::InvalidRequest => "语音请求无效。",
            Self::Unauthorized => "语音请求需要身份认证。",
            Self::Forbidden => "无权访问语音会话。",
            Self::NotFound => "语音会话不存在。",
            Self::Conflict => "语音请求冲突。",
            Self::UnsupportedMedia => "不支持该音频格式。",
            Self::PayloadTooLarge => "语音数据超过限制。",
            Self::UnsupportedCapability => "不支持该语音能力。",
            Self::ProviderAuth => "语音服务认证失败，请检查网关服务配置。",
            Self::RateLimited => "语音请求过于频繁。",
            Self::ProviderUnavailable => "语音服务暂不可用。",
            Self::Timeout => "语音请求超时。",
            Self::Cancelled => "语音请求已取消。",
            Self::ProtocolError => "语音协议错误。",
            Self::Backpressure => "语音队列已满，会话已停止。",
        }
    }
}
/// Construct from a local code; upstream error text must never be used as message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceError {
    pub code: VoiceErrorCode,
    pub message: String,
    pub retryable: bool,
    pub request_id: VoiceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_session_id: Option<VoiceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation: Option<VoiceCounter>,
}
impl VoiceError {
    pub fn new(code: VoiceErrorCode, request_id: VoiceId) -> Self {
        Self {
            code,
            message: code.message().into(),
            retryable: code.retryable(),
            request_id,
            voice_session_id: None,
            generation: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceErrorResponse {
    pub error: VoiceError,
}

fn bounded_text<'de, D: serde::Deserializer<'de>>(d: D, limit: usize) -> Result<String, D::Error> {
    let text = String::deserialize(d)?;
    if text.len() > limit {
        return Err(serde::de::Error::custom("voice text exceeds size limit"));
    }
    Ok(text)
}
fn text_64k<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    bounded_text(d, 64 * 1024)
}
fn text_16k<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    bounded_text(d, 16 * 1024)
}
fn sample_rate_24k<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let rate = u32::deserialize(d)?;
    if rate != 24000 {
        return Err(serde::de::Error::custom("unsupported voice sample rate"));
    }
    Ok(rate)
}
fn mono_channels<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u8, D::Error> {
    let channels = u8::deserialize(d)?;
    if channels != 1 {
        return Err(serde::de::Error::custom("unsupported voice channel count"));
    }
    Ok(channels)
}
fn session_duration<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    let ms = u64::deserialize(d)?;
    if ms == 0 || ms > MAX_SESSION_MS {
        return Err(serde::de::Error::custom("invalid voice session duration"));
    }
    Ok(ms)
}

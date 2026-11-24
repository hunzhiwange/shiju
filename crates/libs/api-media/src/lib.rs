//! Shared wire contract for session-scoped media jobs.

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
pub use uuid::Uuid as MediaJobId;

pub const MEDIA_MODELS_PATH: &str = "/api/media/models";
pub const MEDIA_JOBS_PATH: &str = "/api/media/jobs";
pub const MEDIA_JOB_ROUTE: &str = "/api/media/jobs/{id}";
pub const MEDIA_ASSET_ROUTE: &str = "/api/media/jobs/{id}/assets/{asset_id}";

pub const MAX_MEDIA_UPLOAD_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_MEDIA_ASSET_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_MEDIA_JSON_BYTES: usize = 1024 * 1024;
/// Apply to the HTTP request body before deserializing the JSON.
pub const MAX_MEDIA_SUBMIT_BYTES: usize = MAX_MEDIA_UPLOAD_BYTES.div_ceil(3) * 4 + 256 * 1024;
pub const MAX_MEDIA_ASSETS: usize = 16;
const MAX_MEDIA_INPUTS: usize = 8;
const MAX_MEDIA_TEXT_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaOperation {
    ImageGenerate,
    ImageEdit,
    VideoGenerate,
    SpeechSynthesize,
    AudioTranscribe,
    ImageUnderstand,
    VideoUnderstand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaModel {
    pub deployment_id: String,
    pub model_id: String,
    pub display_name: String,
    pub operations: Vec<MediaOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaModels {
    pub models: Vec<MediaModel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaSessionQuery {
    pub session_id: String,
}

impl MediaSessionQuery {
    pub fn validate(&self) -> Result<(), MediaErrorCode> {
        if valid_reference(&self.session_id) { Ok(()) } else { Err(MediaErrorCode::InvalidRequest) }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum MediaInput {
    Asset { asset_id: String },
    Upload { mime: String, data_base64: String },
}

impl MediaInput {
    pub fn upload(mime: String, bytes: &[u8]) -> Result<Self, MediaErrorCode> {
        if bytes.len() > MAX_MEDIA_UPLOAD_BYTES {
            return Err(MediaErrorCode::PayloadTooLarge);
        }
        if bytes.is_empty() || !valid_media_mime(&mime) {
            return Err(MediaErrorCode::UnsupportedMedia);
        }
        Ok(Self::Upload { mime, data_base64: STANDARD.encode(bytes) })
    }

    /// Asset references must be resolved by the gateway within the authenticated session.
    pub fn decode_upload(&self) -> Result<Option<Vec<u8>>, MediaErrorCode> {
        let Self::Upload { mime, data_base64 } = self else {
            return Ok(None);
        };
        if !valid_media_mime(mime) {
            return Err(MediaErrorCode::UnsupportedMedia);
        }
        if data_base64.len() > MAX_MEDIA_UPLOAD_BYTES.div_ceil(3) * 4 {
            return Err(MediaErrorCode::PayloadTooLarge);
        }
        let bytes = STANDARD.decode(data_base64).map_err(|_| MediaErrorCode::InvalidRequest)?;
        if bytes.len() > MAX_MEDIA_UPLOAD_BYTES {
            return Err(MediaErrorCode::PayloadTooLarge);
        }
        if bytes.is_empty() {
            return Err(MediaErrorCode::InvalidRequest);
        }
        Ok(Some(bytes))
    }
}

/// Generate a fresh request ID for each deliberate submission. It is also the job ID,
/// allowing a client to reconcile a lost POST response via GET in the same session.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaSubmit {
    pub request_id: MediaJobId,
    pub session_id: String,
    pub operation: MediaOperation,
    pub deployment_id: String,
    pub model_id: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub inputs: Vec<MediaInput>,
}

impl MediaSubmit {
    pub fn validate(&self) -> Result<(), MediaErrorCode> {
        if self.request_id.is_nil()
            || !valid_reference(&self.session_id)
            || !valid_reference(&self.deployment_id)
            || !valid_reference(&self.model_id)
        {
            return Err(MediaErrorCode::InvalidRequest);
        }
        if self.inputs.len() > MAX_MEDIA_INPUTS
            || self.text.as_ref().is_some_and(|text| text.len() > MAX_MEDIA_TEXT_BYTES)
        {
            return Err(MediaErrorCode::PayloadTooLarge);
        }

        // Bound the aggregate encoded size before allocating decoded uploads.
        let mut encoded_len = 0;
        for input in &self.inputs {
            match input {
                MediaInput::Asset { asset_id } if !valid_asset_id(asset_id) => {
                    return Err(MediaErrorCode::InvalidRequest);
                }
                MediaInput::Upload { data_base64, .. } => {
                    encoded_len += data_base64.len();
                    if encoded_len > MAX_MEDIA_UPLOAD_BYTES.div_ceil(3) * 4 + MAX_MEDIA_INPUTS * 4 {
                        return Err(MediaErrorCode::PayloadTooLarge);
                    }
                }
                _ => {}
            }
        }
        let mut upload_bytes = 0;
        for input in &self.inputs {
            upload_bytes += input.decode_upload()?.map_or(0, |bytes| bytes.len());
            if upload_bytes > MAX_MEDIA_UPLOAD_BYTES {
                return Err(MediaErrorCode::PayloadTooLarge);
            }
        }
        Ok(())
    }
}

/// Path identifiers are opaque ASCII tokens. Provider IDs and session IDs are JSON/query values.
pub fn valid_asset_id(id: &str) -> bool {
    (1..=128).contains(&id.len())
        && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn valid_reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn valid_media_mime(mime: &str) -> bool {
    let Some((kind, subtype)) = mime.split_once('/') else { return false };
    matches!(kind, "image" | "audio" | "video")
        && (1..=63).contains(&subtype.len())
        && subtype.bytes().next().is_some_and(|byte| byte.is_ascii_alphanumeric())
        && subtype.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'.' | b'+')
        })
        && !matches!(mime, "image/svg+xml")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum MediaJobState {
    Submitted,
    Running {
        progress_percent: Option<u8>,
    },
    Succeeded {
        result: MediaResult,
    },
    Failed {
        code: MediaErrorCode,
    },
    Canceled,
    /// An upstream submission may have succeeded. Only reconcile this job; never start it again.
    SubmissionUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaJob {
    pub id: MediaJobId,
    pub session_id: String,
    pub operation: MediaOperation,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    #[serde(flatten)]
    pub state: MediaJobState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaResult {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub assets: Vec<MediaAsset>,
}

impl MediaResult {
    pub fn validate(&self) -> Result<(), MediaErrorCode> {
        if self.assets.len() > MAX_MEDIA_ASSETS
            || self.text.as_ref().is_some_and(|text| text.len() > MAX_MEDIA_TEXT_BYTES)
        {
            return Err(MediaErrorCode::PayloadTooLarge);
        }
        if self.assets.is_empty() && self.text.as_ref().is_none_or(|text| text.trim().is_empty()) {
            return Err(MediaErrorCode::InvalidRequest);
        }
        for (index, asset) in self.assets.iter().enumerate() {
            asset.validate()?;
            if self.assets[..index].iter().any(|other| other.id == asset.id) {
                return Err(MediaErrorCode::InvalidRequest);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "storage", rename_all = "snake_case")]
pub enum MediaAssetSource {
    Image { image_id: String },
    Stream,
}

/// Read bytes at the job asset endpoint with the same authenticated session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaAsset {
    pub id: String,
    pub mime: String,
    pub byte_len: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
    #[serde(flatten)]
    pub source: MediaAssetSource,
}

impl MediaAsset {
    pub fn validate(&self) -> Result<(), MediaErrorCode> {
        if !valid_asset_id(&self.id)
            || self.byte_len == 0
            || self.width == Some(0)
            || self.height == Some(0)
            || self.duration_ms == Some(0)
        {
            return Err(MediaErrorCode::InvalidRequest);
        }
        if self.byte_len > MAX_MEDIA_ASSET_BYTES {
            return Err(MediaErrorCode::PayloadTooLarge);
        }
        if !valid_media_mime(&self.mime) {
            return Err(MediaErrorCode::UnsupportedMedia);
        }
        match &self.source {
            MediaAssetSource::Image { image_id }
                if self.mime.starts_with("image/")
                    && self.duration_ms.is_none()
                    && image_id.len() == 64
                    && image_id
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')) =>
            {
                Ok(())
            }
            MediaAssetSource::Stream
                if self.mime.starts_with("video/") || self.mime.starts_with("audio/") =>
            {
                Ok(())
            }
            _ => Err(MediaErrorCode::UnsupportedMedia),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaErrorCode {
    InvalidRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    PayloadTooLarge,
    UnsupportedMedia,
    Backpressure,
    UpstreamFailed,
    Unavailable,
}

impl MediaErrorCode {
    pub const fn http_status(self) -> u16 {
        match self {
            Self::InvalidRequest => 400,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::Conflict => 409,
            Self::PayloadTooLarge => 413,
            Self::UnsupportedMedia => 415,
            Self::Backpressure => 429,
            Self::UpstreamFailed => 502,
            Self::Unavailable => 503,
        }
    }

    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidRequest => "Invalid media request.",
            Self::Unauthorized => "Gateway authentication required.",
            Self::Forbidden => "Media operation is not allowed.",
            Self::NotFound => "Media resource not found.",
            Self::Conflict => "Media job conflicts with an existing request or state.",
            Self::PayloadTooLarge => "Media size limit exceeded.",
            Self::UnsupportedMedia => "Unsupported media type.",
            Self::Backpressure => "Too many active media jobs.",
            Self::UpstreamFailed => "Media provider operation failed.",
            Self::Unavailable => "Media service unavailable.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaErrorDetail {
    pub code: MediaErrorCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaErrorResponse {
    pub error: MediaErrorDetail,
}

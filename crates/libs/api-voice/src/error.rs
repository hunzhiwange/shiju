use serde::{Deserialize, Serialize};
use crate::{VoiceCounter, VoiceId};

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

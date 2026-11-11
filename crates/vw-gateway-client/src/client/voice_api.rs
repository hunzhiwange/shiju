//! Native completed-recording API; service credentials never enter the desktop.
use super::GatewayClient;
use crate::http::apply_request_context;
use futures_util::StreamExt;
use vw_api_types::voice::{
    MAX_METADATA_BYTES, MAX_WAV_BYTES, TranscriptionRequest, TranscriptionResponse, VoiceErrorCode,
    VoiceErrorResponse,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceApiError {
    Service(VoiceErrorCode),
    Transport,
    InvalidResponse,
}
impl std::fmt::Display for VoiceApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Service(code) => code.message(),
            Self::Transport => "无法连接语音网关。",
            Self::InvalidResponse => "语音网关响应无效。",
        })
    }
}
impl std::error::Error for VoiceApiError {}

// An abandoned upload/request explicitly cancels at the gateway. Hyper cannot
// reliably signal a peer disconnect to an Axum handler after the body was read.
struct CancelOnDrop {
    request: Option<reqwest::RequestBuilder>,
}
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if let (Some(request), Ok(runtime)) =
            (self.request.take(), tokio::runtime::Handle::try_current())
        {
            runtime.spawn(async move {
                let _ = request.send().await;
            });
        }
    }
}
impl GatewayClient {
    /// Transcribe WAV bytes and correlate the final result to the original draft.
    /// Dropping this future requests cancellation; callers must still reject stale generations.
    pub async fn voice_transcribe(
        &self,
        metadata: &TranscriptionRequest,
        audio: Vec<u8>,
    ) -> Result<TranscriptionResponse, VoiceApiError> {
        let metadata_bytes = serde_json::to_vec(metadata)
            .map_err(|_| VoiceApiError::Service(VoiceErrorCode::InvalidRequest))?;
        if metadata_bytes.len() > MAX_METADATA_BYTES || audio.len() > MAX_WAV_BYTES {
            return Err(VoiceApiError::Service(VoiceErrorCode::PayloadTooLarge));
        }
        if audio.is_empty() {
            return Err(VoiceApiError::Service(VoiceErrorCode::InvalidRequest));
        }
        // The shared endpoint currently supports HTTP only. Remote voice must
        // wait for TLS support in that transport instead of exposing recordings.
        if !self.endpoint.is_loopback() {
            return Err(VoiceApiError::Service(VoiceErrorCode::UnsupportedCapability));
        }
        let base = format!("{}/api/voice/transcriptions", self.endpoint.base_url());
        let mut cancel = CancelOnDrop {
            request: Some(apply_request_context(
                self.dynamic_host_client.delete(format!(
                    "{}/{}",
                    base,
                    encode_id(metadata.request_id.as_ref())
                )),
                &self.endpoint,
            )),
        };
        let file = reqwest::multipart::Part::bytes(audio)
            .file_name("dictation.wav")
            .mime_str("audio/wav")
            .map_err(|_| VoiceApiError::Service(VoiceErrorCode::UnsupportedMedia))?;
        let form = reqwest::multipart::Form::new()
            .part(
                "metadata",
                reqwest::multipart::Part::bytes(metadata_bytes)
                    .mime_str("application/json")
                    .map_err(|_| VoiceApiError::Service(VoiceErrorCode::InvalidRequest))?,
            )
            .part("file", file);
        let response = apply_request_context(
            self.dynamic_host_client.post(base).multipart(form),
            &self.endpoint,
        )
        .send()
        .await
        .map_err(|_| VoiceApiError::Transport)?;
        let status = response.status();
        let mut stream = response.bytes_stream();
        let mut bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| VoiceApiError::Transport)?;
            if bytes.len() + chunk.len() > 128 * 1024 {
                return Err(VoiceApiError::InvalidResponse);
            }
            bytes.extend_from_slice(&chunk);
        }
        cancel.request = None;
        if !status.is_success() {
            let error: VoiceErrorResponse =
                serde_json::from_slice(&bytes).map_err(|_| VoiceApiError::InvalidResponse)?;
            if error.error.request_id != metadata.request_id
                && error.error.request_id.as_ref() != "invalid-request"
            {
                return Err(VoiceApiError::InvalidResponse);
            }
            if error.error.code.http_status() != status.as_u16() {
                return Err(VoiceApiError::InvalidResponse);
            }
            return Err(VoiceApiError::Service(error.error.code));
        }
        let result: TranscriptionResponse =
            serde_json::from_slice(&bytes).map_err(|_| VoiceApiError::InvalidResponse)?;
        if result.request_id != metadata.request_id
            || result.chat_session_id != metadata.chat_session_id
            || result.generation != metadata.generation
        {
            return Err(VoiceApiError::InvalidResponse);
        }
        Ok(result)
    }
    /// Idempotently cancel only a request owned by this authenticated endpoint.
    pub async fn voice_cancel_transcription(
        &self,
        request_id: &vw_api_types::voice::VoiceId,
    ) -> Result<(), VoiceApiError> {
        // The shared endpoint currently supports HTTP only. Remote voice must
        // wait for TLS support in that transport instead of exposing recordings.
        if !self.endpoint.is_loopback() {
            return Err(VoiceApiError::Service(VoiceErrorCode::UnsupportedCapability));
        }
        let response = apply_request_context(
            self.dynamic_host_client.delete(format!(
                "{}/api/voice/transcriptions/{}",
                self.endpoint.base_url(),
                encode_id(request_id.as_ref())
            )),
            &self.endpoint,
        )
        .send()
        .await
        .map_err(|_| VoiceApiError::Transport)?;
        if response.status() == reqwest::StatusCode::NO_CONTENT {
            Ok(())
        } else {
            Err(VoiceApiError::InvalidResponse)
        }
    }
}
fn encode_id(id: &str) -> String {
    id.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
#[cfg(test)]
#[path = "voice_api_tests.rs"]
mod tests;

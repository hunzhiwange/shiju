//! Completed-recording transcription; never uses the generic chat capability trait.
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use vw_api_types::voice::VoiceErrorCode;
use vw_config_types::voice::VoiceConfig;

pub struct TranscriptionProvider {
    client: reqwest::Client,
    endpoint: reqwest::Url,
    authorization: reqwest::header::HeaderValue,
    timeout: Duration,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Transcript {
    pub text: String,
    pub detected_languages: Vec<String>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum TranscriptionError {
    Service(VoiceErrorCode),
    EmptyTranscript,
}
impl std::fmt::Display for TranscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Service(code) => f.write_str(code.message()),
            Self::EmptyTranscript => f.write_str("未识别到语音，请重新录音。"),
        }
    }
}
impl std::error::Error for TranscriptionError {}
impl TranscriptionProvider {
    pub(super) fn connect(
        base: &str,
        key: &str,
        builder: reqwest::ClientBuilder,
        timeout: Duration,
    ) -> Result<Self, TranscriptionError> {
        let endpoint: reqwest::Url = format!("{}/audio/transcriptions", base.trim_end_matches('/'))
            .parse()
            .map_err(|_| service(VoiceErrorCode::InvalidRequest))?;
        // A configured upstream is privileged configuration, never supplied by a recording.
        // HTTP is limited to loopback development servers; public credentials require TLS.
        if !(endpoint.scheme() == "https"
            || (endpoint.scheme() == "http"
                && matches!(endpoint.host_str(), Some("127.0.0.1" | "[::1]" | "localhost"))))
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err(service(VoiceErrorCode::InvalidRequest));
        }
        let mut authorization =
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", key.trim()))
                .map_err(|_| service(VoiceErrorCode::ProviderAuth))?;
        authorization.set_sensitive(true);
        let client = builder
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .build()
            .map_err(|_| service(VoiceErrorCode::ProviderUnavailable))?;
        Ok(Self { client, endpoint, authorization, timeout })
    }

    /// One billable completion attempt. Cancellation drops both upload and response body.
    pub async fn transcribe(
        &self,
        audio: Vec<u8>,
        media_type: &str,
        config: &VoiceConfig,
        cancel: &CancellationToken,
    ) -> Result<Transcript, TranscriptionError> {
        if cancel.is_cancelled() {
            return Err(service(VoiceErrorCode::Cancelled));
        }
        config.validate().map_err(|_| service(VoiceErrorCode::InvalidRequest))?;
        validate_wav(&audio, media_type)?;
        let model = match config.transcription_model {
            vw_config_types::voice::TranscriptionModel::GptTranscribe => "gpt-transcribe",
        };
        let file = reqwest::multipart::Part::bytes(audio)
            .file_name("dictation.wav")
            .mime_str("audio/wav")
            .map_err(|_| service(VoiceErrorCode::UnsupportedMedia))?;
        let mut form = reqwest::multipart::Form::new()
            .part("file", file)
            .text("model", model)
            .text("response_format", "json")
            .text("stream", "false");
        if let Some(language) = config.language.service_code() {
            form = form.text("languages[]", language);
        }
        for word in &config.dictionary {
            form = form.text("keywords[]", word.clone());
        }
        if !config.prompt.is_empty() {
            form = form.text("prompt", config.prompt.clone());
        }
        let work = async {
            let mut response = self
                .client
                .post(self.endpoint.clone())
                .header(reqwest::header::AUTHORIZATION, self.authorization.clone())
                .multipart(form)
                .send()
                .await
                .map_err(transport_error)?;
            let status = response.status();
            if !status.is_success() {
                // Never read or attach upstream error bodies, URLs or credential headers.
                return Err(service(match status.as_u16() {
                    401 | 403 => VoiceErrorCode::ProviderAuth,
                    429 => VoiceErrorCode::RateLimited,
                    _ => VoiceErrorCode::ProviderUnavailable,
                }));
            }
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
                if body.len() + chunk.len() > 256 * 1024 {
                    return Err(service(VoiceErrorCode::ProtocolError));
                }
                body.extend_from_slice(&chunk);
            }
            let result: WireTranscript = serde_json::from_slice(&body)
                .map_err(|_| service(VoiceErrorCode::ProtocolError))?;
            if result.text.len() > 64 * 1024
                || result.languages.len() > 100
                || result.languages.iter().any(|l| l.code.len() > 128)
            {
                return Err(service(VoiceErrorCode::ProtocolError));
            }
            if result.text.trim().is_empty() {
                return Err(TranscriptionError::EmptyTranscript);
            }
            Ok(Transcript {
                text: result.text,
                detected_languages: result.languages.into_iter().map(|l| l.code).collect(),
            })
        };
        tokio::select! {
            biased;
            () = cancel.cancelled() => Err(service(VoiceErrorCode::Cancelled)),
            result = tokio::time::timeout(self.timeout, work) => result.unwrap_or_else(|_| Err(service(VoiceErrorCode::Timeout))),
        }
    }
}
#[derive(serde::Deserialize)]
struct WireTranscript {
    text: String,
    #[serde(default)]
    languages: Vec<WireLanguage>,
}
#[derive(serde::Deserialize)]
struct WireLanguage {
    code: String,
}
fn service(code: VoiceErrorCode) -> TranscriptionError {
    TranscriptionError::Service(code)
}
fn transport_error(error: reqwest::Error) -> TranscriptionError {
    service(if error.is_timeout() {
        VoiceErrorCode::Timeout
    } else {
        VoiceErrorCode::ProviderUnavailable
    })
}

/// Parse RIFF chunks rather than trusting content type, filename or client duration.
fn validate_wav(audio: &[u8], media_type: &str) -> Result<(), TranscriptionError> {
    let invalid = || service(VoiceErrorCode::UnsupportedMedia);
    if audio.len() > 24_000_000 {
        return Err(service(VoiceErrorCode::PayloadTooLarge));
    }
    if audio.is_empty() {
        return Err(service(VoiceErrorCode::InvalidRequest));
    }
    if media_type != "audio/wav"
        || audio.len() < 12
        || &audio[..4] != b"RIFF"
        || &audio[8..12] != b"WAVE"
    {
        return Err(invalid());
    }
    let u32_at = |offset| {
        u32::from_le_bytes(audio[offset..offset + 4].try_into().expect("checked RIFF bounds"))
            as usize
    };
    if u32_at(4) + 8 != audio.len() {
        return Err(invalid());
    }
    let mut offset = 12;
    let mut fmt = false;
    let mut data = None;
    while offset < audio.len() {
        if audio.len() - offset < 8 {
            return Err(invalid());
        }
        let len = u32_at(offset + 4);
        let start = offset + 8;
        let end = start.checked_add(len).filter(|end| *end <= audio.len()).ok_or_else(invalid)?;
        match &audio[offset..offset + 4] {
            b"fmt " => {
                if fmt
                    || len != 16
                    || &audio[start..end] != b"\x01\0\x01\0\xc0\x5d\0\0\x80\xbb\0\0\x02\0\x10\0"
                {
                    return Err(invalid());
                }
                fmt = true;
            }
            b"data" => {
                if data.is_some() || !fmt || len % 2 != 0 {
                    return Err(invalid());
                }
                data = Some(len);
            }
            _ => {}
        }
        offset = end + (len % 2);
        if offset > audio.len() {
            return Err(invalid());
        }
    }
    match data {
        Some(0) | None => Err(service(VoiceErrorCode::InvalidRequest)),
        Some(bytes) if bytes > 180 * 24000 * 2 => Err(service(VoiceErrorCode::PayloadTooLarge)),
        Some(_) if fmt => Ok(()),
        _ => Err(invalid()),
    }
}
#[cfg(test)]
impl TranscriptionProvider {
    pub(super) fn fixture(endpoint: &str) -> Self {
        Self::connect(
            endpoint.strip_suffix("/audio/transcriptions").unwrap(),
            "fixture-secret",
            reqwest::Client::builder().no_proxy(),
            Duration::from_secs(2),
        )
        .unwrap()
    }
}

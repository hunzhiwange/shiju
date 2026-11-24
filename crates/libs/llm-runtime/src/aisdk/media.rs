//! Media execution against an already authorized provider and wire protocol.

use std::time::Duration;

use aisdk::{
    core::{
        AudioFile, AudioFormat, AudioModel, AudioModelError, DynamicModel, ImageModel,
        ImageModelOptions, LanguageModel, LanguageModelRequest, LanguageModelStreamChunkType,
        SpeechAudio, SpeechOptions, TranscriptionFormat, TranscriptionOptions, UserImage,
        UserMessage, VideoJobStatus, VideoModel, VideoModelError, VideoModelOptions,
        VideoReferenceImage, language_model::LanguageModelStreamChunk, messages::UserVideo,
    },
    providers::{
        OpenAI, OpenAICompatible,
        openai::{audio_model::OpenAIAudioProtocol, video_model::OpenAIVideoCapabilities},
    },
};
use base64::{Engine, engine::general_purpose::STANDARD};
use futures_util::StreamExt;
use reqwest::Url;
use tokio_util::sync::CancellationToken;

const MAX_TEXT: usize = 32 * 1024;
const MAX_IMAGE: usize = 32 * 1024 * 1024;
const MAX_AUDIO: usize = 32 * 1024 * 1024;
const MAX_VIDEO: usize = 128 * 1024 * 1024;
const VIDEO_DEADLINE: Duration = Duration::from_secs(300);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaProtocol {
    Image,
    ImageEdit,
    GeminiChatImage,
    DashScopeImage,
    Video,
    Speech,
    CosyVoice,
    Transcription,
    ChatVision,
}

impl MediaProtocol {
    pub fn from_endpoint_type(value: &str) -> Option<Self> {
        Some(match value {
            "image-generation" => Self::Image,
            "image-edit" => Self::ImageEdit,
            "gemini-chat-image" => Self::GeminiChatImage,
            "dashscope-image" => Self::DashScopeImage,
            "openai-video" => Self::Video,
            "audio-speech" => Self::Speech,
            "bailian-cosyvoice" => Self::CosyVoice,
            "audio-transcription" => Self::Transcription,
            "openai" | "chat-completions" => Self::ChatVision,
            _ => return None,
        })
    }

    pub fn supports_native_video(self, provider: &str, model: &str) -> bool {
        self == Self::ChatVision
            && matches!(provider, "alibaba" | "alibaba-cn" | "alibabacn")
            && matches!(
                model,
                "qwen3.8-omni-flash"
                    | "qwen3-vl-plus"
                    | "qwen3-vl-30b-a3b"
                    | "qwen3.5-flash"
                    | "qwen3.5-plus"
                    | "qwen3.6-flash"
                    | "qwen3.6-plus"
                    | "qwen3.7-flash"
                    | "qwen3.7-plus"
                    | "qwen3.8-flash"
                    | "qwen3.8-max"
                    | "qwen4-max"
            )
    }
}

pub struct MediaFile {
    pub mime: String,
    pub bytes: Vec<u8>,
}

pub struct MediaOutput {
    pub text: Option<String>,
    pub files: Vec<MediaFile>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaFailure {
    InvalidInput,
    Unsupported,
    TooLarge,
    Upstream,
    SubmissionUnknown,
    Canceled,
}

#[derive(Clone)]
pub struct MediaEndpoint {
    protocol: MediaProtocol,
    provider: String,
    model: String,
    base: Url,
    path: String,
    bearer: String,
    video: OpenAIVideoCapabilities,
    voice: Option<String>,
    result_origin: Option<Url>,
}

pub fn credential(provider: &model_types::Info) -> Option<String> {
    let auth = agent_auth::get(&provider.id);
    // OAuth sessions for chat products do not authorize the public media API.
    let auth = auth.as_ref().filter(|auth| matches!(auth, agent_auth::Info::Api(_)));
    let (bearer, _) = super::request::deployment_bearer(provider, auth)?;
    reqwest::header::HeaderValue::from_str(&format!("Bearer {bearer}")).ok()?;
    (bearer != agent_auth::OAUTH_DUMMY_KEY).then_some(bearer)
}

pub fn deployment_url(value: &str) -> Option<Url> {
    let url = Url::parse(value).ok()?;
    let local = matches!(url.host_str(), Some("127.0.0.1" | "[::1]" | "localhost"));
    ((url.scheme() == "https" || (url.scheme() == "http" && local))
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none())
    .then_some(url)
}

impl MediaEndpoint {
    pub fn new(
        protocol: MediaProtocol,
        provider: &model_types::Info,
        model: &model_types::Model,
        base: Url,
        bearer: String,
    ) -> Option<Self> {
        let mut base = base;
        if matches!(protocol, MediaProtocol::DashScopeImage | MediaProtocol::CosyVoice)
            && let Some(prefix) =
                base.path().trim_end_matches('/').strip_suffix("/compatible-mode/v1")
        {
            let prefix = prefix.to_owned();
            base.set_path(&prefix);
        }
        let path = match protocol {
            MediaProtocol::Image => "images/generations",
            MediaProtocol::ImageEdit => "images/edits",
            MediaProtocol::GeminiChatImage | MediaProtocol::ChatVision => "chat/completions",
            MediaProtocol::DashScopeImage => "api/v1/services/aigc/image-generation/generation",
            MediaProtocol::Video => "videos",
            // The SDK resolves the audio paths from the selected base and protocol.
            MediaProtocol::Speech | MediaProtocol::CosyVoice | MediaProtocol::Transcription => "",
        };
        let flag = |key| model.options.get(key).and_then(serde_json::Value::as_bool) == Some(true);
        let voice = model
            .options
            .get("media_voice")
            .and_then(serde_json::Value::as_str)
            .filter(|voice| !voice.trim().is_empty())
            .map(str::to_owned);
        let result_origin = model
            .options
            .get("media_result_origin")
            .and_then(serde_json::Value::as_str)
            .and_then(deployment_url)
            .filter(|url| url.scheme() == "https" && url.path() == "/");
        if protocol == MediaProtocol::CosyVoice && voice.is_none() {
            return None;
        }
        let video = OpenAIVideoCapabilities {
            image_reference: flag("media_video_image_reference")
                || provider.id == "openai"
                || (model.capabilities.input.image && model.capabilities.output.video),
            cancel: flag("media_video_cancel"),
            content: true,
        };
        Some(Self {
            protocol,
            provider: provider.id.clone(),
            model: model.api.id.clone(),
            base,
            path: path.into(),
            bearer,
            video,
            voice,
            result_origin,
        })
    }

    pub fn supports_image_to_video(&self) -> bool {
        self.video.image_reference
    }

    pub async fn execute(
        &self,
        text: Option<String>,
        mut files: Vec<MediaFile>,
        cancel: &CancellationToken,
        progress: impl FnMut(u8) + Send,
    ) -> Result<MediaOutput, MediaFailure> {
        if cancel.is_cancelled() {
            return Err(MediaFailure::Canceled);
        }
        if self.protocol == MediaProtocol::Video {
            return self.video(text.unwrap_or_default(), files.pop(), cancel, progress).await;
        }
        let work = async {
            match self.protocol {
                MediaProtocol::Image
                | MediaProtocol::ImageEdit
                | MediaProtocol::GeminiChatImage
                | MediaProtocol::DashScopeImage => self.image(text, files).await,
                MediaProtocol::Speech | MediaProtocol::CosyVoice => {
                    self.speech(text.unwrap_or_default()).await
                }
                MediaProtocol::Transcription => self.transcribe(text, files.pop()).await,
                MediaProtocol::ChatVision => self.understand(text, files).await,
                MediaProtocol::Video => unreachable!(),
            }
        };
        tokio::select! {
            biased;
            () = cancel.cancelled() => Err(MediaFailure::Canceled),
            result = tokio::time::timeout(VIDEO_DEADLINE, work) => {
                result.unwrap_or(Err(MediaFailure::SubmissionUnknown))
            }
        }
    }

    fn openai(&self) -> Result<OpenAI<DynamicModel>, MediaFailure> {
        OpenAI::<DynamicModel>::builder()
            .model_name(self.model.clone())
            .api_key(self.bearer.clone())
            .base_url(self.base.as_str())
            .path(self.path.clone())
            .build()
            .map_err(|_| MediaFailure::Unsupported)
    }

    async fn image(
        &self,
        text: Option<String>,
        files: Vec<MediaFile>,
    ) -> Result<MediaOutput, MediaFailure> {
        let options = ImageModelOptions {
            prompt: text,
            files: files.into_iter().map(user_image).collect(),
            mask: None,
            n: Some(1),
            size: None,
            aspect_ratio: None,
            seed: None,
            headers: None,
            body: None,
        };
        let images = self.openai()?.generate_image(options).await.map_err(sdk_error)?.into_images();
        let [encoded] = images.as_slice() else {
            return Err(MediaFailure::Upstream);
        };
        let encoded = if let Some(inline) = encoded.strip_prefix("data:image/") {
            inline.split_once(";base64,").ok_or(MediaFailure::Upstream)?.1
        } else {
            encoded.as_str()
        };
        if encoded.len() > MAX_IMAGE.div_ceil(3) * 4 {
            return Err(MediaFailure::TooLarge);
        }
        let bytes = STANDARD.decode(encoded).map_err(|_| MediaFailure::Upstream)?;
        if bytes.is_empty() || bytes.len() > MAX_IMAGE {
            return Err(MediaFailure::Upstream);
        }
        // Image MIME is detected from bytes before publishing to the session registry.
        Ok(file_output("image/", bytes))
    }

    async fn understand(
        &self,
        text: Option<String>,
        files: Vec<MediaFile>,
    ) -> Result<MediaOutput, MediaFailure> {
        let model = OpenAICompatible::<DynamicModel>::builder()
            .provider_name(self.provider.clone())
            .model_name(self.model.clone())
            .base_url(self.base.as_str())
            .api_key(self.bearer.clone())
            .path(self.path.clone())
            .build()
            .map_err(|_| MediaFailure::Unsupported)?;
        let mut user =
            UserMessage::new(text.unwrap_or_else(|| "Describe the supplied media.".into()));
        for file in files {
            user = if file.mime.starts_with("video/") {
                if !self.protocol.supports_native_video(&self.provider, &self.model) {
                    return Err(MediaFailure::Unsupported);
                }
                user.with_video(UserVideo::from_bytes(file.mime, file.bytes))
            } else {
                user.with_image(user_image(file))
            };
        }
        let mut request = LanguageModelRequest::builder()
            .model(model)
            .messages(vec![aisdk::core::messages::Message::User(user)])
            .build();
        request.max_output_tokens = Some(2048);
        let options = std::ops::Deref::deref(&request).clone();
        // Use the direct SDK stream. The buffered SDK path retries POST requests.
        let mut stream = request.model.stream_text(options).await.map_err(sdk_error)?;
        let mut text = String::new();
        while let Some(event) = stream.next().await {
            for part in event.map_err(sdk_error)? {
                match part {
                    LanguageModelStreamChunk::Delta(LanguageModelStreamChunkType::TextDelta(
                        delta,
                    )) => {
                        if delta.len() > MAX_TEXT - text.len() {
                            return Err(MediaFailure::TooLarge);
                        }
                        text.push_str(&delta);
                    }
                    LanguageModelStreamChunk::Done(_) => return self.text_output(text),
                    LanguageModelStreamChunk::Delta(
                        LanguageModelStreamChunkType::Failed(_)
                        | LanguageModelStreamChunkType::Incomplete(_)
                        | LanguageModelStreamChunkType::NotSupported(_)
                        | LanguageModelStreamChunkType::ToolCallStart(_),
                    ) => return Err(MediaFailure::Upstream),
                    _ => {}
                }
            }
        }
        Err(MediaFailure::SubmissionUnknown)
    }

    async fn speech(&self, text: String) -> Result<MediaOutput, MediaFailure> {
        let protocol = if self.protocol == MediaProtocol::CosyVoice {
            OpenAIAudioProtocol::BailianCosyVoice
        } else {
            OpenAIAudioProtocol::OpenAICompatible { speech: true, file_transcription: false }
        };
        let response = self
            .openai()?
            .audio_model(protocol)
            .map_err(audio_error)?
            .synthesize_speech(SpeechOptions {
                text,
                voice: self.voice.clone().unwrap_or_else(|| "alloy".into()),
                format: AudioFormat::Mp3,
                speed: None,
                max_bytes: MAX_AUDIO,
            })
            .await
            .map_err(audio_error)?;
        let bytes = match response.audio {
            SpeechAudio::Bytes(bytes) => bytes,
            SpeechAudio::Url(url) => self.download(url, "audio/", MAX_AUDIO).await?.bytes,
        };
        Ok(file_output(&response.media_type, bytes))
    }

    async fn transcribe(
        &self,
        prompt: Option<String>,
        file: Option<MediaFile>,
    ) -> Result<MediaOutput, MediaFailure> {
        let MediaFile { mime, bytes } = file.ok_or(MediaFailure::InvalidInput)?;
        let extension = match mime.as_str() {
            "audio/wav" | "audio/x-wav" | "audio/wave" => "wav",
            "audio/mpeg" | "audio/mp3" => "mp3",
            "audio/mp4" | "audio/x-m4a" => "m4a",
            "audio/webm" => "webm",
            "audio/ogg" => "ogg",
            "audio/flac" => "flac",
            _ => return Err(MediaFailure::Unsupported),
        };
        let protocol =
            OpenAIAudioProtocol::OpenAICompatible { speech: false, file_transcription: true };
        let result = self
            .openai()?
            .audio_model(protocol)
            .map_err(audio_error)?
            .transcribe_audio(TranscriptionOptions {
                file: AudioFile {
                    file_name: format!("media.{extension}"),
                    media_type: mime,
                    bytes,
                },
                language: None,
                prompt,
                format: TranscriptionFormat::Json,
                word_timestamps: false,
            })
            .await
            .map_err(audio_error)?;
        self.text_output(result.text)
    }

    async fn video(
        &self,
        prompt: String,
        file: Option<MediaFile>,
        cancel: &CancellationToken,
        mut progress: impl FnMut(u8) + Send,
    ) -> Result<MediaOutput, MediaFailure> {
        let adapter = self.openai()?.video_model(self.video).map_err(video_error)?;
        let deadline = tokio::time::sleep(VIDEO_DEADLINE);
        tokio::pin!(deadline);
        let mut job = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(MediaFailure::Canceled),
            result = adapter.submit_video(VideoModelOptions { prompt, seconds: None, size: None,
                input_reference: file.map(|file| VideoReferenceImage {
                    media_type: file.mime, bytes: file.bytes,
                }),
            }) => result.map_err(video_error)?,
        };
        let id = job.id.clone();
        let work = async {
            let mut reported = 0;
            loop {
                match job.status {
                    VideoJobStatus::Succeeded => {
                        progress(99.max(reported));
                        if let Some(url) = job.result_url.take()
                            && self.allows_result_url(&url)
                        {
                            let result = self.download(url, "video/", MAX_VIDEO).await?;
                            return Ok(file_output(&result.mime, result.bytes));
                        }
                        let content = adapter
                            .read_video_content(&id, MAX_VIDEO)
                            .await
                            .map_err(video_error)?;
                        return Ok(file_output(&content.media_type, content.bytes));
                    }
                    VideoJobStatus::Failed
                    | VideoJobStatus::Cancelled
                    | VideoJobStatus::TimedOut => {
                        return Err(MediaFailure::Upstream);
                    }
                    _ => {}
                }
                if let Some(value) = job.progress.filter(|value| value.is_finite()) {
                    let percent = value.clamp(0.0, 100.0) as u8;
                    if percent > reported {
                        progress(percent);
                        reported = percent;
                    }
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
                job = adapter.video_status(&id).await.map_err(video_error)?;
            }
        };
        let failure = tokio::select! {
            biased;
            () = cancel.cancelled() => MediaFailure::Canceled,
            () = &mut deadline => MediaFailure::Upstream,
            result = work => return result,
        };
        if self.video.cancel {
            let _ = tokio::time::timeout(Duration::from_secs(5), adapter.cancel_video(&id)).await;
        }
        Err(failure)
    }

    fn allows_result_url(&self, url: &Url) -> bool {
        (url.origin() == self.base.origin()
            || self.result_origin.as_ref().is_some_and(|origin| url.origin() == origin.origin()))
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none()
            && !url.as_str().contains(&self.bearer)
            && !url.query_pairs().any(|(_, value)| value.contains(&self.bearer))
    }

    async fn download(
        &self,
        url: Url,
        kind: &str,
        limit: usize,
    ) -> Result<MediaFile, MediaFailure> {
        if !self.allows_result_url(&url) {
            return Err(MediaFailure::Unsupported);
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| MediaFailure::Upstream)?;
        let mut response = client.get(url).send().await.map_err(|_| MediaFailure::Upstream)?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(MediaFailure::Upstream);
        }
        let mime = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| value.starts_with(kind))
            .ok_or(MediaFailure::Upstream)?;
        if response.content_length().is_some_and(|length| length > limit as u64) {
            return Err(MediaFailure::TooLarge);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| MediaFailure::Upstream)? {
            if chunk.len() > limit - bytes.len() {
                return Err(MediaFailure::TooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(MediaFile { mime, bytes })
    }

    fn text_output(&self, text: String) -> Result<MediaOutput, MediaFailure> {
        if text.trim().is_empty() || text.len() > MAX_TEXT || text.contains(&self.bearer) {
            return Err(MediaFailure::Upstream);
        }
        Ok(MediaOutput { text: Some(text), files: Vec::new() })
    }
}

fn user_image(file: MediaFile) -> UserImage {
    let encoded = STANDARD.encode(file.bytes);
    UserImage::new(format!("data:{};base64,{encoded}", file.mime)).media_type(file.mime)
}

fn file_output(mime: &str, bytes: Vec<u8>) -> MediaOutput {
    MediaOutput { text: None, files: vec![MediaFile { mime: mime.into(), bytes }] }
}

fn sdk_error(error: aisdk::Error) -> MediaFailure {
    match error {
        aisdk::Error::InvalidInput(_) => MediaFailure::InvalidInput,
        aisdk::Error::ApiError { status_code: None, .. } => MediaFailure::SubmissionUnknown,
        _ => MediaFailure::Upstream,
    }
}

fn audio_error(error: AudioModelError) -> MediaFailure {
    match error {
        AudioModelError::InvalidInput(_) => MediaFailure::InvalidInput,
        AudioModelError::Unsupported(_) => MediaFailure::Unsupported,
        AudioModelError::ContentTooLarge { .. } => MediaFailure::TooLarge,
        AudioModelError::Transport | AudioModelError::Timeout => MediaFailure::SubmissionUnknown,
        _ => MediaFailure::Upstream,
    }
}

fn video_error(error: VideoModelError) -> MediaFailure {
    match error {
        VideoModelError::SubmissionUnknown => MediaFailure::SubmissionUnknown,
        VideoModelError::Unsupported(_) => MediaFailure::Unsupported,
        VideoModelError::InvalidInput(_) => MediaFailure::InvalidInput,
        VideoModelError::ContentTooLarge { .. } => MediaFailure::TooLarge,
        _ => MediaFailure::Upstream,
    }
}

#[cfg(test)]
#[path = "media_tests.rs"]
mod tests;

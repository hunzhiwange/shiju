//! Completed dictation routing. Local never constructs a gateway client.
use super::{Capture, Device};
use crate::app::voice::{
    local::{self, Cancellation, Capabilities, LocalSpeechError},
    permissions::{Permission, PermissionStatus, SystemPermissions, VoicePermissions},
    preferences::{Preferences, VoiceEngine},
};
use std::{
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};
use vw_gateway_client::vw_api_types::voice::{TranscriptionRequest, VoiceCounter, VoiceId};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

#[derive(Clone)]
pub(super) enum Route {
    Local { language: Option<String> },
    Cloud { endpoint: GatewayEndpoint, metadata: TranscriptionRequest },
}
#[derive(Clone)]
pub struct Prepared {
    pub(super) device: Device,
    pub(super) route: Route,
}
impl Route {
    fn capture_limit(&self) -> Duration {
        Duration::from_secs(match self {
            Self::Local { .. } => local::MAX_RECOGNITION_SECONDS as u64,
            Self::Cloud { .. } => crate::app::voice::capture::MAX_SECONDS,
        })
    }
}
pub(super) async fn select_route(
    preferences: &Preferences,
    endpoint: GatewayEndpoint,
    session: Option<String>,
    generation: u64,
) -> Result<Route, String> {
    preferences.validate()?;
    if preferences.engine == VoiceEngine::Local {
        return Ok(Route::Local { language: preferences.local_language.clone() });
    }
    let session = session.ok_or("云端听写需要已建立的聊天会话；本机听写可用于新草稿。")?;
    if !endpoint.is_loopback() {
        return Err("当前云端语音传输仅支持本机网关。".into());
    }
    let client = GatewayClient::new(endpoint.clone()).map_err(|_| "无法连接语音网关。")?;
    let config: vw_config_types::voice::VoiceConfig =
        client.voice_config().await.map_err(|_| "无法读取云端语音设置。")?;
    if !config.dictation_enabled {
        return Err("请先在网关设置中启用云端听写。".into());
    }
    Ok(Route::Cloud {
        endpoint,
        metadata: TranscriptionRequest {
            request_id: VoiceId::try_from(format!("{:032x}", rand::random::<u128>()))
                .map_err(str::to_owned)?,
            chat_session_id: VoiceId::try_from(session).map_err(str::to_owned)?,
            provider_id: VoiceId::try_from(config.provider_id.ok_or("请先配置云端语音服务。")?)
                .map_err(str::to_owned)?,
            generation: VoiceCounter::try_from(generation).map_err(str::to_owned)?,
        },
    })
}
pub(super) fn local_error(error: LocalSpeechError) -> String {
    match error {
        LocalSpeechError::Permission(PermissionStatus::Restricted) =>
            "系统策略限制了语音识别，请检查屏幕使用时间或设备管理设置。".into(),
        LocalSpeechError::Permission(_) =>
            "语音识别尚未授权。请在系统弹窗或系统设置→隐私与安全性→语音识别中允许 ShiJu，返回后重试。".into(),
        LocalSpeechError::OfflineUnavailable | LocalSpeechError::UnsupportedLanguage =>
            "该语言没有可用的本机识别资源。请在 macOS 键盘→听写中检查语言资源，或选择支持的本机语言；不会自动上传云端。".into(),
        LocalSpeechError::AppMetadata =>
            "请使用包含语音识别用途说明的新版 ShiJu.app，重新打开应用后授权。".into(),
        LocalSpeechError::UnsupportedPlatform => "本机语音识别目前仅支持 macOS，请显式选择可用的语音引擎。".into(),
        LocalSpeechError::InvalidAudio | LocalSpeechError::Limit =>
            "本机听写最多60秒，请缩短录音后重试；旧录音不会自动上传云端。".into(),
        LocalSpeechError::Cancelled => "已取消本机听写。".into(),
        LocalSpeechError::Timeout => "本机识别超时，任务已停止，请缩短录音后重试。".into(),
        LocalSpeechError::EmptyResult => "没有识别到文字，请重新录音。".into(),
        LocalSpeechError::Storage => "无法创建私有临时音频，请检查磁盘空间及临时目录权限。".into(),
        _ => "本机语音服务不可用，请检查系统听写语言资源后重试；不会自动使用云端。".into(),
    }
}
fn gated_capture<T>(
    capabilities: &Capabilities,
    open: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    if !capabilities.supports_on_device {
        return Err(local_error(LocalSpeechError::OfflineUnavailable));
    }
    if capabilities.permission != PermissionStatus::Granted {
        return Err(local_error(LocalSpeechError::Permission(capabilities.permission)));
    }
    if !capabilities.available {
        return Err(local_error(LocalSpeechError::Unavailable));
    }
    open()
}
fn authorize_speech(language: Option<&str>, cancel: &Cancellation) -> Result<Capabilities, String> {
    if cancel.is_cancelled() {
        return Err(local_error(LocalSpeechError::Cancelled));
    }
    let mut capabilities = local::capabilities(language).map_err(local_error)?;
    if cancel.is_cancelled() {
        return Err(local_error(LocalSpeechError::Cancelled));
    }
    if !capabilities.supports_on_device {
        return Err(local_error(LocalSpeechError::OfflineUnavailable));
    }
    if capabilities.permission != PermissionStatus::Granted {
        VoicePermissions::new(SystemPermissions)
            .request_from_user_action(Permission::SpeechRecognition)
            .map_err(|_| "无法申请语音识别授权。请使用新版 ShiJu.app，并在系统设置→隐私与安全性→语音识别中检查权限。".to_owned())?;
        capabilities = local::capabilities(language).map_err(local_error)?;
    }
    gated_capture(&capabilities, || Ok(()))?;
    if cancel.is_cancelled() {
        return Err(local_error(LocalSpeechError::Cancelled));
    }
    Ok(capabilities)
}
pub(super) async fn prepare(
    preferences: Preferences,
    endpoint: GatewayEndpoint,
    session: Option<String>,
    generation: u64,
    cancellation: tokio::sync::watch::Receiver<bool>,
    native_cancel: Cancellation,
) -> Result<Prepared, String> {
    let mut prepare_guard = CancelOnDrop::new(native_cancel.clone());
    if native_cancel.is_cancelled() {
        return Err(local_error(LocalSpeechError::Cancelled));
    }
    let route = select_route(&preferences, endpoint, session, generation).await?;
    let limit = route.capture_limit();
    let language = match &route {
        Route::Local { language } => Some(language.clone()),
        Route::Cloud { .. } => None,
    };
    let device = tokio::task::spawn_blocking(move || {
        if native_cancel.is_cancelled()
            || *cancellation.borrow()
            || cancellation.has_changed().is_err()
        {
            return Err("听写已取消。".into());
        }
        let permission = VoicePermissions::new(SystemPermissions);
        let capabilities = language
            .as_ref()
            .map(|locale| authorize_speech(locale.as_deref(), &native_cancel))
            .transpose()?;
        let open = || {
            if native_cancel.is_cancelled()
                || *cancellation.borrow()
                || cancellation.has_changed().is_err()
            {
                return Err("听写已取消。".into());
            }
            permission.request_from_user_action(Permission::Microphone).map_err(|_| {
                "无法申请麦克风权限，请在系统设置→隐私与安全性→麦克风中允许 ShiJu。".to_owned()
            })?;
            if permission.refresh(Permission::Microphone) != PermissionStatus::Granted {
                return Err("麦克风尚未授权，请允许 ShiJu 访问后重新点击听写。".into());
            }
            if language.is_some()
                && permission.refresh(Permission::SpeechRecognition) != PermissionStatus::Granted
            {
                return Err(local_error(LocalSpeechError::Permission(
                    permission.refresh(Permission::SpeechRecognition),
                )));
            }
            if native_cancel.is_cancelled()
                || *cancellation.borrow()
                || cancellation.has_changed().is_err()
            {
                return Err("听写已取消。".into());
            }
            let capture = Capture::start_with_limit(preferences.microphone, limit)
                .map_err(|_| "无法开始录音，请检查麦克风、设备选择及系统权限。".to_owned())?;
            if native_cancel.is_cancelled()
                || *cancellation.borrow()
                || cancellation.has_changed().is_err()
            {
                drop(capture);
                return Err("听写已取消。".into());
            }
            Ok(Arc::new(Mutex::new(Some(capture))))
        };
        match capabilities {
            Some(ref c) => gated_capture(c, open),
            None => open(),
        }
    })
    .await
    .map_err(|_| "无法开始录音。")??;
    prepare_guard.disarm();
    Ok(Prepared { device, route })
}
struct CancelOnDrop {
    token: Cancellation,
    armed: bool,
}
impl CancelOnDrop {
    fn new(token: Cancellation) -> Self {
        Self { token, armed: true }
    }
    fn disarm(&mut self) {
        self.armed = false;
    }
}
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if self.armed {
            self.token.cancel();
        }
    }
}

// Only the OS recognition boundary is replaceable; cancellation and final-result
// checks always execute here, including after a late native success.
async fn recognize_local_with<F>(
    pcm: Vec<i16>,
    language: Option<String>,
    cancel: Cancellation,
    recognize: F,
) -> Result<String, String>
where
    F: FnOnce(&[i16], Option<&str>, &Cancellation) -> local::Result<local::Transcript>
        + Send
        + 'static,
{
    let mut guard = CancelOnDrop::new(cancel.clone());
    if cancel.is_cancelled() {
        return Err(local_error(LocalSpeechError::Cancelled));
    }
    let worker_cancel = cancel.clone();
    let transcript =
        tokio::task::spawn_blocking(move || recognize(&pcm, language.as_deref(), &worker_cancel))
            .await
            .map_err(|_| "本机识别任务异常结束，请重试。")?;
    if cancel.is_cancelled() {
        return Err(local_error(LocalSpeechError::Cancelled));
    }
    guard.disarm();
    transcript.map(|t| t.text).map_err(local_error)
}
async fn recognize_local(
    pcm: Vec<i16>,
    language: Option<String>,
    cancel: Cancellation,
) -> Result<String, String> {
    tokio::time::timeout(
        Duration::from_secs(75),
        recognize_local_with(pcm, language, cancel, local::transcribe_pcm),
    )
    .await
    .map_err(|_| local_error(LocalSpeechError::Timeout))?
}
pub(super) async fn transcribe(
    device: Device,
    route: Route,
    cancel: Cancellation,
) -> Result<(String, Vec<u8>), String> {
    let recording = tokio::task::spawn_blocking(move || {
        device
            .lock()
            .map_err(|_| "录音状态不可用。")?
            .take()
            .ok_or("录音已取消。")?
            .finish()
            .map_err(|_| "录音无效或没有检测到声音。")
    })
    .await
    .map_err(|_| "无法完成录音。")??;
    let wav = recording.wav().map_err(|_| "录音格式无效。")?;
    let text = match route {
        Route::Local { language } => recognize_local(recording.pcm, language, cancel).await?,
        Route::Cloud { endpoint, metadata } => {
            cloud_transcribe(endpoint, metadata, wav.clone()).await?
        }
    };
    Ok((text, wav))
}
async fn cloud_transcribe(
    endpoint: GatewayEndpoint,
    metadata: TranscriptionRequest,
    wav: Vec<u8>,
) -> Result<String, String> {
    let client = GatewayClient::new(endpoint).map_err(|_| "无法连接语音网关。")?;
    tokio::time::timeout(Duration::from_secs(90), client.voice_transcribe(&metadata, wav))
        .await
        .map_err(|_| "云端听写超时。")?
        .map(|r| r.text)
        .map_err(|e| e.to_string())
}
fn decode_local(wav: &[u8]) -> Result<Vec<i16>, String> {
    let pcm = crate::app::voice::recordings::pcm(wav)?;
    if pcm.len() / 2 > local::SAMPLE_RATE as usize * local::MAX_RECOGNITION_SECONDS {
        return Err(local_error(LocalSpeechError::Limit));
    }
    Ok(pcm.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect())
}
pub(crate) async fn retry(
    preferences: Preferences,
    endpoint: GatewayEndpoint,
    session: Option<String>,
    generation: u64,
    wav: Vec<u8>,
    cancel: Cancellation,
) -> Result<String, String> {
    retry_with_local(
        preferences,
        endpoint,
        session,
        generation,
        wav,
        cancel,
        |pcm, language, cancel| async move {
            let locale = language.clone();
            let token = cancel.clone();
            tokio::task::spawn_blocking(move || authorize_speech(locale.as_deref(), &token))
                .await
                .map_err(|_| "无法检查本机语音权限。")??;
            recognize_local(pcm, language, cancel).await
        },
    )
    .await
}
async fn retry_with_local<F, Fut>(
    preferences: Preferences,
    endpoint: GatewayEndpoint,
    session: Option<String>,
    generation: u64,
    wav: Vec<u8>,
    cancel: Cancellation,
    local: F,
) -> Result<String, String>
where
    F: FnOnce(Vec<i16>, Option<String>, Cancellation) -> Fut,
    Fut: Future<Output = Result<String, String>>,
{
    let mut guard = CancelOnDrop::new(cancel.clone());
    if cancel.is_cancelled() {
        return Err(local_error(LocalSpeechError::Cancelled));
    }
    let route = select_route(&preferences, endpoint, session, generation).await?;
    let text = match route {
        Route::Local { language } => local(decode_local(&wav)?, language, cancel.clone()).await?,
        Route::Cloud { endpoint, metadata } => cloud_transcribe(endpoint, metadata, wav).await?,
    };
    if cancel.is_cancelled() {
        return Err(local_error(LocalSpeechError::Cancelled));
    }
    guard.disarm();
    Ok(text)
}
#[cfg(test)]
#[path = "voice_dictation_tests.rs"]
mod tests;

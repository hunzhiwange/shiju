//! Playback-only worker. It never opens capture or reads chat/session state.
use super::playback::{Playback, PlaybackError};
use iced::futures::StreamExt;
use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::Duration,
};
use tokio::sync::watch;
use vw_config_types::voice::VoiceName;
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

pub(crate) struct Preview {
    stop: watch::Sender<bool>,
    local_cancel: Option<super::local::Cancellation>,
    worker: Option<JoinHandle<()>>,
    result: Arc<Mutex<Option<Result<(), String>>>>,
}
impl Preview {
    pub(crate) fn start_local(voice: Option<String>) -> Result<Self, String> {
        Self::start_local_worker(voice, |voice, cancel| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| "无法启动本机声音试听。".to_owned())?
                .block_on(super::local_conversation::speak(
                    "你好，这是识矩的本机声音试听。".into(),
                    voice,
                    cancel,
                ))
        })
    }
    pub(super) fn start_local_worker(
        voice: Option<String>,
        speak: impl FnOnce(Option<String>, super::local::Cancellation) -> Result<(), String>
        + Send
        + 'static,
    ) -> Result<Self, String> {
        let cancel = super::local::Cancellation::default();
        let token = cancel.clone();
        let (stop, _) = watch::channel(false);
        let result = Arc::new(Mutex::new(None));
        let output = result.clone();
        let worker = std::thread::Builder::new()
            .name("local-voice-preview".into())
            .spawn(move || {
                let outcome = speak(voice, token.clone());
                *output.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(if token.is_cancelled() { Ok(()) } else { outcome });
            })
            .map_err(|_| "无法启动本机声音试听。".to_owned())?;
        Ok(Self { stop, local_cancel: Some(cancel), worker: Some(worker), result })
    }

    pub(crate) fn start(endpoint: GatewayEndpoint, voice: VoiceName) -> Result<Self, String> {
        Self::start_output(endpoint, voice, Playback::start)
    }
    pub(super) fn start_output<S: 'static>(
        endpoint: GatewayEndpoint,
        voice: VoiceName,
        open: impl FnOnce() -> Result<Playback<S>, PlaybackError> + Send + 'static,
    ) -> Result<Self, String> {
        let (stop, mut cancel) = watch::channel(false);
        let result = Arc::new(Mutex::new(None));
        let output = result.clone();
        let worker = std::thread::Builder::new().name("voice-preview".into()).spawn(move || {
            let outcome = tokio::runtime::Builder::new_current_thread().enable_all().build()
                .map_err(|_| "无法启动音色试听。".to_owned()).and_then(|rt| rt.block_on(async move {
                    tokio::select! {
                        biased;
                        _ = cancel.changed() => Ok(()),
                        result = tokio::time::timeout(Duration::from_secs(25), play(endpoint, voice, open)) => result.unwrap_or_else(|_| Err("音色试听超时。".into())),
                    }
                }));
            *output.lock().unwrap_or_else(|e| e.into_inner()) = Some(outcome);
        }).map_err(|_| "无法启动音色试听。")?;
        Ok(Self { stop, local_cancel: None, worker: Some(worker), result })
    }
    pub(crate) fn result(&self) -> Option<Result<(), String>> {
        self.result.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}
impl Drop for Preview {
    fn drop(&mut self) {
        if let Some(cancel) = &self.local_cancel {
            cancel.cancel();
            // Do not join a macOS synthesis worker from the UI/main run loop.
            // Its token stops synthesis and the existing bounded playback worker.
            self.worker.take();
            return;
        }
        let _ = self.stop.send(true);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
async fn play<S>(
    endpoint: GatewayEndpoint,
    voice: VoiceName,
    open: impl FnOnce() -> Result<Playback<S>, PlaybackError>,
) -> Result<(), String> {
    let client = GatewayClient::new(endpoint).map_err(|_| "无法连接语音网关。")?;
    let mut stream = client.voice_preview(&voice).await.map_err(|_| "无法生成音色试听。")?;
    let mut playback = open().map_err(|_| "扬声器不可用。")?;
    let mut submitted = 0u64;
    let mut total = 0usize;
    let mut pending = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| "音色试听连接中断。")?;
        total += chunk.len();
        if total > 24000 * 2 * 10 {
            return Err("音色试听过长。".into());
        }
        pending.extend(chunk);
        // Pace against the actual output callback; never overflow the two-second ring.
        while pending.len() >= 960 {
            if submitted.saturating_sub(u64::from(playback.played_ms()) * 24) > 24000 {
                tokio::time::sleep(Duration::from_millis(20)).await;
            } else {
                let rest = pending.split_off(960);
                playback.push_pcm(&pending).map_err(|_| "音色试听播放失败。")?;
                submitted += (pending.len() / 2) as u64;
                pending = rest;
            }
            if playback.error().is_some() {
                return Err("扬声器已断开。".into());
            }
        }
    }
    if total == 0 || !pending.len().is_multiple_of(2) {
        return Err("音色试听音频无效。".into());
    }
    if !pending.is_empty() {
        while playback.pending() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        playback.push_pcm(&pending).map_err(|_| "音色试听播放失败。")?;
    }
    while playback.pending() {
        if playback.error().is_some() {
            return Err("扬声器已断开。".into());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Ok(())
}

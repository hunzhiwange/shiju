//! Local synthesis and bounded playback; model routing remains in ordinary chat.
use super::{
    local::{self, Cancellation},
    playback::Playback,
};
use std::time::{Duration, Instant};

pub(crate) fn speech_text(raw: &str) -> String {
    if raw.len() > 65_536 {
        return String::new();
    }
    crate::app::components::chat_panel::message_view::assistant_speech_text(raw)
}
pub(crate) fn spoken_prefix(text: &str) -> String {
    if text.chars().count() <= local::MAX_TEXT_CHARS {
        return text.to_owned();
    }
    let suffix = "。其余内容请查看聊天。";
    let mut prefix: String =
        text.chars().take(local::MAX_TEXT_CHARS - suffix.chars().count()).collect();
    prefix.push_str(suffix);
    prefix
}
struct CancelOnDrop(Cancellation);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
/// The worker owns the non-Send audio stream. Dropping this future cancels synthesis too.
pub(crate) async fn speak(
    text: String,
    voice: Option<String>,
    cancel: Cancellation,
) -> Result<(), String> {
    let _guard = CancelOnDrop(cancel.clone());
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::Builder::new()
        .name("local-voice-answer".into())
        .spawn(move || {
            let result = (|| {
                let audio = local::synthesize(&text, voice.as_deref(), &cancel).map_err(
                    |error| match error {
                        local::LocalSpeechError::VoiceUnavailable => {
                            "所选系统声音不可用，请在语音设置中重新选择。".to_owned()
                        }
                        local::LocalSpeechError::Cancelled => "朗读已取消。".to_owned(),
                        _ => "本机朗读失败，请检查 macOS 系统声音及语言资源后重试。".to_owned(),
                    },
                )?;
                play(&audio.pcm, &cancel, Playback::start)
            })();
            let _ = tx.send(result);
        })
        .map_err(|_| "无法启动本机朗读，请重试。".to_owned())?;
    rx.await.map_err(|_| "本机朗读已停止。".to_owned())?
}
fn play<S>(
    pcm: &[i16],
    cancel: &Cancellation,
    open: impl FnOnce() -> Result<Playback<S>, super::playback::PlaybackError>,
) -> Result<(), String> {
    if pcm.is_empty() || pcm.len() > local::SAMPLE_RATE as usize * local::MAX_SYNTHESIS_SECONDS {
        return Err("本机朗读音频为空或超过两分钟，请查看聊天文字。".into());
    }
    if cancel.is_cancelled() {
        return Err("朗读已取消。".into());
    }
    let mut playback = open().map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(pcm.len() as u64 / 24_000 + 5);
    let mut offset = 0;
    loop {
        if cancel.is_cancelled() {
            playback.stop();
            return Err("朗读已取消。".into());
        }
        if let Some(error) = playback.error() {
            return Err(error.to_string());
        }
        if Instant::now() >= deadline {
            return Err("扬声器播放超时，请检查输出设备。".into());
        }
        // At most one second ahead, in 100 ms packets, below the two-second queue cap.
        if offset < pcm.len() && (offset as u64).saturating_sub(playback.played_ms() * 24) < 24_000
        {
            let end = (offset + 2400).min(pcm.len());
            let bytes: Vec<u8> = pcm[offset..end].iter().flat_map(|v| v.to_le_bytes()).collect();
            playback.push_pcm(&bytes).map_err(|e| e.to_string())?;
            offset = end;
        } else if offset == pcm.len() && !playback.pending() {
            return Ok(());
        } else {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
#[cfg(test)]
#[path = "local_conversation_tests.rs"]
mod tests;

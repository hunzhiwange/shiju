//! One owner for capture, bounded transport and playback. No credentials or media are logged.
use super::{
    capture::{Capture, LiveCapture},
    playback::Playback,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::sync::watch;
use vw_gateway_client::vw_api_types::voice::*;
use vw_gateway_client::{GatewayClient, GatewayEndpoint, VoiceStream};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    Connecting,
    Listening,
    Speaking,
    Muted,
    Ending,
    Error,
}
#[derive(Clone)]
pub(crate) struct Caption {
    pub role: TranscriptRole,
    item: VoiceId,
    index: u32,
    pub text: String,
    final_text: bool,
}
#[derive(Clone)]
pub(crate) struct Snapshot {
    pub phase: Phase,
    pub microphone: bool,
    pub sending: bool,
    pub playing: bool,
    pub captions: Vec<Caption>,
    pub error: Option<&'static str>,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            phase: Phase::Connecting,
            microphone: false,
            sending: false,
            playing: false,
            captions: Vec::new(),
            error: None,
        }
    }
}
impl Snapshot {
    fn caption(
        &mut self,
        role: TranscriptRole,
        item: VoiceId,
        index: u32,
        value: String,
        final_text: bool,
    ) {
        if let Some(entry) =
            self.captions.iter_mut().find(|c| c.role == role && c.item == item && c.index == index)
        {
            if entry.final_text {
                return;
            }
            if final_text {
                entry.text = value;
            } else if entry.text.len() + value.len() <= 65536 {
                entry.text.push_str(&value);
            }
            entry.final_text = final_text;
        } else {
            if self.captions.len() == 100 {
                self.captions.remove(0);
            }
            self.captions.push(Caption { role, item, index, text: value, final_text });
        }
    }
}
#[derive(Clone, Default)]
struct Control {
    end: bool,
    muted: bool,
    interrupt: u64,
}
pub(crate) struct Conversation {
    control: watch::Sender<Control>,
    snapshot: Arc<Mutex<Snapshot>>,
    worker: Option<JoinHandle<()>>,
}
impl Conversation {
    pub(crate) fn start(endpoint: GatewayEndpoint, session: String) -> Result<Self, &'static str> {
        let (control, receiver) = watch::channel(Control::default());
        let snapshot = Arc::new(Mutex::new(Snapshot::default()));
        let output = snapshot.clone();
        let worker = std::thread::Builder::new()
            .name("voice-conversation".into())
            .spawn(move || {
                let result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|_| "无法启动语音通话。")
                    .and_then(|runtime| {
                        runtime.block_on(run(endpoint, session, receiver, output.clone()))
                    });
                let mut state = output.lock().unwrap_or_else(|e| e.into_inner());
                state.microphone = false;
                state.sending = false;
                state.playing = false;
                state.phase = if result.is_err() { Phase::Error } else { Phase::Ending };
                state.error = result.err();
            })
            .map_err(|_| "无法启动语音通话。")?;
        Ok(Self { control, snapshot, worker: Some(worker) })
    }
    pub(crate) fn snapshot(&self) -> Snapshot {
        self.snapshot.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    pub(crate) fn mute(&self) {
        self.control.send_modify(|c| c.muted = !c.muted);
    }
    pub(crate) fn interrupt(&self) {
        self.control.send_modify(|c| c.interrupt = c.interrupt.wrapping_add(1));
    }
    pub(crate) fn end(&mut self) {
        self.control.send_modify(|c| c.end = true);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
impl Drop for Conversation {
    fn drop(&mut self) {
        self.end();
    }
}

struct Media<S = cpal::Stream> {
    capture: Option<Capture>,
    playback: Option<Playback<S>>,
}
impl<S> Media<S> {
    fn end(&mut self) {
        self.capture.take();
        self.interrupt();
    }
    fn interrupt(&mut self) {
        if let Some(mut playback) = self.playback.take() {
            playback.stop();
        }
    }
}
impl<S> Drop for Media<S> {
    fn drop(&mut self) {
        self.end();
    }
}

async fn run(
    endpoint: GatewayEndpoint,
    session: String,
    mut control: watch::Receiver<Control>,
    output: Arc<Mutex<Snapshot>>,
) -> Result<(), &'static str> {
    // The outer cancellation also drops connection setup, device and in-flight network sends.
    let mut cancelled = control.clone();
    tokio::select! {
        biased;
        _ = async {
            loop {
                if cancelled.borrow().end { break; }
                if cancelled.changed().await.is_err() { break; }
            }
        } => Ok(()),
        result = connected(endpoint, session, &mut control, output) => result,
    }
}
async fn connected(
    endpoint: GatewayEndpoint,
    session: String,
    control: &mut watch::Receiver<Control>,
    output: Arc<Mutex<Snapshot>>,
) -> Result<(), &'static str> {
    if !endpoint.is_loopback() {
        return Err("当前语音传输仅支持本机网关。");
    }
    let client = GatewayClient::new(endpoint.clone()).map_err(|_| "无法连接语音网关。")?;
    let config: vw_config_types::voice::VoiceConfig =
        client.voice_config().await.map_err(|_| "无法读取语音设置。")?;
    if !config.realtime_enabled {
        return Err("请先在网关设置中启用语音聊天。");
    }
    let start = VoiceSessionStart {
        request_id: VoiceId::try_from(format!("{:032x}", rand::random::<u128>()))
            .map_err(|_| "通话标识无效。")?,
        chat_session_id: VoiceId::try_from(session).map_err(|_| "聊天会话标识无效。")?,
        generation: VoiceCounter::FIRST,
        provider_id: VoiceId::try_from(config.provider_id.ok_or("请先配置语音服务。")?)
            .map_err(|_| "语音服务标识无效。")?,
        turn_mode: TurnMode::ServerVad,
    };
    let mut stream = VoiceStream::connect(&endpoint, &start)
        .await
        .map_err(|_| "无法建立语音通道，请检查网关语音设置。")?;
    use super::permissions::{Permission, SystemPermissions, VoicePermissions};
    VoicePermissions::new(SystemPermissions)
        .request_from_user_action(Permission::Microphone)
        .map_err(|_| "请允许麦克风访问后重试。")?;
    let live = LiveCapture::start().map_err(|_| "无法采集麦克风，请检查权限或设备。")?;
    pump(&mut stream, live, control, output).await
}
async fn pump(
    stream: &mut VoiceStream,
    live: LiveCapture,
    control: &mut watch::Receiver<Control>,
    output: Arc<Mutex<Snapshot>>,
) -> Result<(), &'static str> {
    let (capture, live) = live.into_parts();
    let mut media = Media { capture: Some(capture), playback: None };
    let mut last = Control::default();
    let mut response: Option<VoiceId> = None;
    let mut audio_item: Option<(VoiceId, u32)> = None;
    let mut response_done = true;
    let mut awaiting_interrupt = false;
    let mut was_sending = false;
    let mut tick = tokio::time::interval(Duration::from_millis(20));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            biased;
            changed = control.changed() => {
                if changed.is_err() { return Ok(()); }
                let next = control.borrow_and_update().clone();
                live.clear();
                if next.end { return Ok(()); }
                if next.muted != last.muted {
                    stream.send(stream.generation(), VoiceEvent::InputMute { muted: next.muted }).await.map_err(|_| "语音静音操作失败。")?;
                }
                if next.interrupt != last.interrupt && !awaiting_interrupt {
                    let position = media.playback.as_ref().map_or(0, Playback::played_ms);
                    media.interrupt();
                    if let Some(id) = &response {
                        if let Some((item, index)) = &audio_item {
                            stream.send(stream.generation(), VoiceEvent::PlaybackProgress { response_id: id.clone(), item_id: item.clone(), content_index: *index, audio_end_ms: position }).await.map_err(|_| "语音播放状态发送失败。")?;
                        }
                        stream.send(stream.generation(), VoiceEvent::ResponseInterrupt {
                            response_id: id.clone(), item_id: audio_item.as_ref().map(|v| v.0.clone()),
                            content_index: audio_item.as_ref().map(|v| v.1), audio_end_ms: audio_item.as_ref().map(|_| position),
                        }).await.map_err(|_| "无法打断语音回答。")?;
                        awaiting_interrupt = true;
                    }
                }
                last = next;
            }
            frame = stream.next() => {
                let frame = frame.map_err(|_| "语音连接已断开，请重新开始。");
                let frame = frame?;
                match frame.event {
                    VoiceEvent::GenerationChanged { .. } => {
                        media.interrupt(); live.clear(); awaiting_interrupt = false;
                        response = None; audio_item = None; response_done = true;
                    }
                    VoiceEvent::ResponseStarted { response_id } if !awaiting_interrupt => {
                        response = Some(response_id); response_done = false; live.clear();
                    }
                    VoiceEvent::AudioDelta { response_id, item_id, content_index, audio, .. } if !awaiting_interrupt => {
                        if response.as_ref() != Some(&response_id) { return Err("语音回答标识不匹配。"); }
                        if audio_item.as_ref().is_some_and(|item| item != &(item_id.clone(), content_index)) { return Err("不支持同时播放多个语音项目。"); }
                        audio_item = Some((item_id, content_index));
                        let playback = if let Some(playback) = &mut media.playback { playback } else {
                            media.playback = Some(Playback::start().map_err(|_| "扬声器不可用。")?);
                            media.playback.as_mut().ok_or("扬声器不可用。")?
                        };
                        playback.push_pcm(&STANDARD.decode(audio).map_err(|_| "语音音频格式无效。")?).map_err(|_| "语音播放缓冲区溢出或设备断开。")?;
                        live.clear();
                    }
                    VoiceEvent::ResponseDone { response_id, .. } if response.as_ref() == Some(&response_id) => { response_done = true; }
                    VoiceEvent::TranscriptDelta { role, item_id, content_index, delta } if !awaiting_interrupt => {
                        output.lock().unwrap_or_else(|e| e.into_inner()).caption(role, item_id, content_index, delta, false);
                    }
                    VoiceEvent::TranscriptFinal { role, item_id, content_index, text } if !awaiting_interrupt => {
                        output.lock().unwrap_or_else(|e| e.into_inner()).caption(role, item_id, content_index, text, true);
                    }
                    VoiceEvent::SessionClosed { .. } => return Ok(()),
                    VoiceEvent::Error(_) => return Err("语音服务出现错误，请检查服务配置。"),
                    _ => {}
                }
            }
            _ = tick.tick() => {
                if media.capture.as_mut().is_some_and(|capture| capture.try_result().is_some()) { return Err("麦克风已停止，请检查设备或通话时长。"); }
                if let Some(playback) = &mut media.playback {
                    if playback.error().is_some() { return Err("扬声器已断开。"); }
                    if let (Some(id), Some((item, index))) = (&response, &audio_item) {
                        stream.send(stream.generation(), VoiceEvent::PlaybackProgress { response_id: id.clone(), item_id: item.clone(), content_index: *index, audio_end_ms: playback.played_ms() }).await.map_err(|_| "语音播放状态发送失败。")?;
                    }
                    if response_done && !playback.pending() { media.interrupt(); response = None; audio_item = None; }
                } else if response_done && !awaiting_interrupt { response = None; }
                let playing = media.playback.as_ref().is_some_and(Playback::pending);
                let sending = !last.muted && response.is_none() && !awaiting_interrupt;
                if !sending || !was_sending { live.clear(); }
                was_sending = sending;
                // Half-duplex echo protection: never upload while generating/playing speech.
                for _ in 0..50 {
                    let Some(pcm) = live.next_frame() else { break; };
                    if sending && !control.borrow().muted {
                        let bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
                        stream.send(stream.generation(), VoiceEvent::AudioAppend(AudioPayload { audio: STANDARD.encode(bytes), samples_per_channel: pcm.len() })).await.map_err(|_| "麦克风音频发送失败。")?;
                    }
                }
                let mut snapshot = output.lock().unwrap_or_else(|e| e.into_inner());
                snapshot.microphone = true; snapshot.sending = sending; snapshot.playing = playing;
                snapshot.phase = if last.muted { Phase::Muted } else if response.is_some() { Phase::Speaking } else { Phase::Listening };
            }
        }
    }
}
#[cfg(test)]
#[path = "conversation_tests.rs"]
mod tests;

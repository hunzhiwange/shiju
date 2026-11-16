//! Domain-owned voice session state and local cleanup.
use super::{voice_draft::{Draft, Scope}, voice_dictation::{self as dictation, Device}, voice_local_chat as local_chat};
use gateway_endpoint::GatewayEndpoint;
use iced::widget::text_editor;
#[derive(Default)]
pub struct State {
    pub draft: Draft,
    pub waveform: crate::components::voice_waveform::Waveform,
    pub screen: desktop_voice::screen_context::State,
    pub insertion: desktop_voice::insertion::State,
    pub hotkeys: desktop_voice::hotkeys::Runtime,
    pub finish_requested: bool,
    pub recordings: super::voice_recordings::State,
    pub audio: Option<Vec<u8>>,
    pub retain_requested: bool,
    pub settings: desktop_settings_voice::State,
    pub conversation: Option<desktop_voice::conversation::Conversation>,
    pub local_conversation: Option<local_chat::State>,
    pub local_stop: Option<local_chat::Stop>,
    pub conversation_scope: Option<Scope>,
    pub conversation_error_reported: bool,
    pub conversation_endpoint: Option<GatewayEndpoint>,
    pub endpoint: Option<GatewayEndpoint>,
    pub cursor: Option<text_editor::Cursor>,
    pub device: Option<Device>,
    pub cancel: Option<tokio::sync::watch::Sender<bool>>,
    pub route: Option<dictation::Route>,
    pub native_cancel: Option<desktop_voice::local::Cancellation>,
}
impl State {
    pub fn chat_active(&self) -> bool {
        self.conversation.is_some()
            || self.local_conversation.is_some()
            || self.local_stop.is_some()
    }
    pub fn ready(&self) -> bool {
        self.device.is_some()
    }
}
pub fn stop(state: &mut State) {
    state.waveform.clear();
    if let Some(cancel) = state.native_cancel.take() {
        cancel.cancel();
    }
    if let Some(cancel) = state.cancel.take() {
        let _ = cancel.send(true);
    }
    if let Some(device) = state.device.take()
        && let Ok(mut device) = device.lock()
    {
        drop(device.take());
    }
    state.insertion.target = None;
    state.finish_requested = false;
    state.route = None;
    state.audio = None;
    state.retain_requested = false;
}
pub fn retain_completed(state: &mut State, preferences_path: &Result<std::path::PathBuf, String>, recordings_path: &Result<std::path::PathBuf, String>) -> Option<String> {
    let audio = state.audio.take()?;
    if !state.retain_requested {
        return None;
    }
    let result = (|| {
        let preferences = desktop_voice::preferences::Preferences::read(preferences_path.as_ref().map_err(Clone::clone)?)?;
        if preferences.retain_recordings {
            let store = voice_recordings::Store::open(recordings_path.as_ref().map_err(Clone::clone)?)?;
            store.save(true, &audio)?;
            state.recordings.entries = store.list()?;
        }
        Ok::<_, String>(())
    })();
    result.err()
}

pub enum CaptureEvent {
    Started(u64, Result<dictation::Prepared, String>),
    Tick(u64),
    Transcribed(u64, Result<(String, Vec<u8>), String>),
}
pub enum CaptureStep {
    Idle,
    Task(iced::Task<CaptureEvent>),
    Finish,
    Error(String),
    Completed(u64, Result<String, String>),
}
pub fn start_capture(state: &mut State, preferences: desktop_voice::preferences::Preferences, scope: Scope, endpoint: GatewayEndpoint, cursor: text_editor::Cursor, text: String) -> iced::Task<CaptureEvent> {
    let Some(generation) = state.draft.start(scope.clone(), text) else { return iced::Task::none(); };
    state.waveform.clear();
    state.cursor = Some(cursor);
    state.retain_requested = preferences.retain_recordings;
    state.endpoint = Some(endpoint.clone());
    let (tx, mut rx) = tokio::sync::watch::channel(false);
    state.cancel = Some(tx);
    let cancellation = rx.clone();
    let native_cancel = desktop_voice::local::Cancellation::default();
    state.native_cancel = Some(native_cancel.clone());
    iced::Task::perform(async move {
        tokio::select! {
            _ = rx.changed() => Err("听写已取消。".into()),
            result = tokio::time::timeout(std::time::Duration::from_secs(20), dictation::prepare(preferences, endpoint, scope.session_id().map(str::to_owned), generation, cancellation, native_cancel)) => result.unwrap_or_else(|_| Err("准备录音超时，请重试。".into())),
        }
    }, move |result| CaptureEvent::Started(generation, result))
}
fn tick(generation: u64) -> CaptureStep {
    CaptureStep::Task(iced::Task::perform(async { tokio::time::sleep(std::time::Duration::from_millis(100)).await }, move |_| CaptureEvent::Tick(generation)))
}
pub fn capture_event(state: &mut State, event: CaptureEvent) -> CaptureStep {
    use super::voice_draft::Phase;
    match event {
        CaptureEvent::Started(generation, result) => {
            if state.draft.generation != generation || state.draft.phase != Phase::Recording || state.draft.scope.is_none() { return CaptureStep::Idle; }
            match result {
                Ok(prepared) => {
                    state.device = Some(prepared.device); state.route = Some(prepared.route);
                    if state.finish_requested { CaptureStep::Finish } else { tick(generation) }
                }
                Err(error) => CaptureStep::Error(error),
            }
        }
        CaptureEvent::Tick(generation) => {
            if generation != state.draft.generation || state.draft.phase != Phase::Recording { return CaptureStep::Idle; }
            if let Some(summary) = state.device.as_ref().and_then(|device| device.lock().ok()?.as_ref().map(|c| c.volume_summary())) {
                state.waveform.push(summary.sequence, summary.peak);
            }
            let outcome = state.device.as_ref().and_then(|device| device.lock().ok()?.as_mut()?.try_result());
            if let Some(outcome) = outcome {
                let local_limit = matches!(state.route, Some(dictation::Route::Local { .. })) && matches!(outcome, Err(audio_capture::CaptureError::Limit | audio_capture::CaptureError::Timeout));
                return CaptureStep::Error(if local_limit { "本机听写最多60秒，录音已停止；请重新开始，并在60秒内点击完成。" } else { "录音已停止，请检查麦克风或缩短录音后重试。" }.into());
            }
            tick(generation)
        }
        CaptureEvent::Transcribed(generation, result) => {
            if state.draft.generation != generation || state.draft.phase != Phase::Transcribing { return CaptureStep::Idle; }
            let result = result.map(|(text, wav)| { state.audio = Some(wav); text });
            CaptureStep::Completed(generation, result)
        }
    }
}
pub fn finish_capture(state: &mut State) -> CaptureStep {
    use super::voice_draft::Phase;
    state.hotkeys.machine.reset();
    if state.draft.phase != Phase::Recording { return CaptureStep::Idle; }
    if state.device.is_none() { state.finish_requested = true; return CaptureStep::Idle; }
    state.finish_requested = false;
    let (Some(device), Some(route)) = (state.device.take(), state.route.take()) else { return CaptureStep::Error("录音会话已失效，请重新开始听写。".into()); };
    state.waveform.clear();
    state.draft.phase = Phase::Transcribing;
    let generation = state.draft.generation;
    let (tx, mut rx) = tokio::sync::watch::channel(false);
    state.cancel = Some(tx);
    let native_cancel = desktop_voice::local::Cancellation::default();
    state.native_cancel = Some(native_cancel.clone());
    CaptureStep::Task(iced::Task::perform(async move {
        tokio::select! {
            _ = rx.changed() => Err("听写已取消。".into()),
            result = dictation::transcribe(device, route, native_cancel) => result,
        }
    }, move |result| CaptureEvent::Transcribed(generation, result)))
}

pub fn onboarding(state: &mut State, event: crate::components::voice_onboarding::OnboardingEvent) -> Result<Option<String>, String> {
    use crate::components::voice_onboarding::OnboardingEvent::*;
    match event {
        Open => { state.insertion.onboarding = true; state.insertion.refresh(); }
        Close => state.insertion.onboarding = false,
        Refresh => state.insertion.refresh(),
        Discard => { if state.insertion.pending.take().is_some() { state.draft.cancel(); state.endpoint = None; } }
        Copy => return Ok(state.insertion.pending.clone()),
        Permission(permission) => {
            use desktop_voice::permissions::{SystemPermissions, VoicePermissions};
            VoicePermissions::new(SystemPermissions).request_from_user_action(permission).map_err(|error| error.to_string())?;
            state.insertion.refresh();
        }
    }
    Ok(None)
}
pub struct Completion {
    pub paste: Option<String>,
    pub error: Option<String>,
    pub edited: bool,
}
pub fn complete(state: &mut State, generation: u64, result: Result<String, String>, scope: Option<Scope>, current: &str, preferences_path: Result<std::path::PathBuf, String>, recordings_path: Result<std::path::PathBuf, String>) -> Completion {
    use super::voice_draft::Phase;
    let mut output = Completion { paste: None, error: None, edited: false };
    if state.draft.generation != generation || state.draft.phase != Phase::Transcribing { return output; }
    let text = match result {
        Ok(text) if !text.trim().is_empty() => text,
        result => {
            stop(state); state.draft.phase = Phase::Error;
            output.error = Some(result.err().unwrap_or_else(|| "没有识别到文字，请重试。".into()));
            return output;
        }
    };
    if let Some(target) = state.insertion.target.take() {
        let outcome = target.and_then(|target| target.insert(&text));
        let retention_error = retain_completed(state, &preferences_path, &recordings_path);
        state.draft.phase = Phase::Idle;
        stop(state);
        if let Err(error) = outcome {
            state.insertion.pending = Some(text);
            output.error = Some(format!("{error} 听写结果已保留，请在语音设置的跨应用听写试用中显式复制。"));
            return output;
        }
        state.draft.cancel(); output.error = retention_error;
        return output;
    }
    let Some(scope) = scope else { return output; };
    output.error = retain_completed(state, &preferences_path, &recordings_path);
    output.paste = state.draft.complete(generation, &scope, current, text);
    stop(state);
    output.edited = output.error.is_none() && state.draft.pending.is_some();
    output
}
pub fn end_cloud(state: &mut State) {
    state.screen.cancel(); state.screen.enabled = false; state.conversation.take();
    state.conversation_scope = None; state.conversation_endpoint = None;
}
pub fn cancel_dictation(state: &mut State) {
    stop(state); state.draft.cancel(); state.endpoint = None; state.cursor = None;
}
pub fn start_cloud(state: &mut State, scope: Scope, endpoint: GatewayEndpoint, load_microphone: fn() -> Result<Option<String>, String>) -> Result<(), String> {
    let session = scope.session_id().ok_or("云端语音聊天需要已建立的聊天会话。")?;
    let conversation = desktop_voice::conversation::Conversation::start(endpoint.clone(), session.to_owned(), load_microphone)?;
    state.conversation_error_reported = false; state.conversation = Some(conversation);
    state.conversation_scope = Some(scope); state.conversation_endpoint = Some(endpoint);
    Ok(())
}

pub fn invalidate_screen(state: &mut State) {
    if state.screen.enabled
        && state.local_conversation.is_none()
        && state.conversation.as_ref().is_none_or(|c| {
            matches!(
                c.snapshot().phase,
                desktop_voice::conversation::Phase::Connecting
                    | desktop_voice::conversation::Phase::Error
                    | desktop_voice::conversation::Phase::Ending
            )
        })
    {
        state.screen.cancel();
        state.screen.enabled = false;
    }
    if state.screen.generation.is_some()
        && state.screen.generation
            != state.conversation.as_ref().and_then(|c| c.snapshot().generation)
    {
        state.screen.cancel();
    }
    if state.screen.enabled && !desktop_voice::screen_context::permitted() {
        state.screen.cancel();
        state.screen.enabled = false;
    }
}

fn auth_changed(old: Option<&GatewayEndpoint>, current: &GatewayEndpoint) -> bool {
    old.is_some_and(|old| old.auth.as_ref().and_then(|a| a.skey.as_ref()) != current.auth.as_ref().and_then(|a| a.skey.as_ref()))
}
pub fn conversation_changed(state: &State, current: &Option<Scope>, endpoint: &GatewayEndpoint) -> bool {
    state.chat_active() && (&state.conversation_scope != current || auth_changed(state.conversation_endpoint.as_ref(), endpoint))
}
pub fn draft_changed(state: &State, current: &Option<Scope>, endpoint: &GatewayEndpoint) -> bool {
    state.draft.scope.is_some() && (&state.draft.scope != current || auth_changed(state.endpoint.as_ref(), endpoint))
}

pub enum StartCheck { Ready, Idle, Error(&'static str), Info(&'static str) }
pub fn check_start(state: &mut State, chat: bool) -> StartCheck {
    use super::voice_draft::Phase;
    if state.insertion.pending.is_some() { return StartCheck::Error("请先复制或丢弃未插入的跨应用听写结果。"); }
    if state.recordings.replay.is_some() || state.settings.preview.is_some() || state.recordings.busy {
        return StartCheck::Error(if chat { "请先停止回放或试听再开始语音聊天。" } else { "请先结束回放、试听或重新识别再开始听写。" });
    }
    if chat {
        if let Some(chat) = state.local_conversation.as_mut() { chat.show = true; return StartCheck::Idle; }
        if state.chat_active() { return StartCheck::Idle; }
        if matches!(state.draft.phase, Phase::Recording | Phase::Transcribing) { return StartCheck::Info("请先结束听写再开始语音聊天。"); }
    } else if state.chat_active() { return StartCheck::Info("请先结束语音聊天再开始听写。"); }
    StartCheck::Ready
}

#[derive(Clone)]
pub enum Event {
    Local(super::voice_local_chat::Event),
    Screen(super::voice_screen::ScreenEvent),
    Onboarding(crate::components::voice_onboarding::OnboardingEvent),
    Recordings(super::voice_recordings::Event),
    Transcribed(u64, Result<(String, Vec<u8>), String>),
    ChatStart,
    ChatMute,
    ChatInterrupt,
    ChatEnd,
    ChatTick,
    HotkeyTick,
    Start,
    Finish,
    Cancel,
    Apply,
    Started(u64, Result<dictation::Prepared, String>),
    Completed(u64, Result<String, String>),
    Tick(u64),
}
// Do not log transcripts, endpoint credentials or capture payloads through Message::Debug.
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Local(..) => "VoiceLocalChat",
            Self::Screen(..) => "VoiceScreen",
            Self::Onboarding(..) => "VoiceOnboarding",
            Self::Recordings(..) => "VoiceRecordings",
            Self::Transcribed(..) => "VoiceTranscribed",
            Self::ChatStart => "VoiceChatStart",
            Self::ChatMute => "VoiceChatMute",
            Self::ChatInterrupt => "VoiceChatInterrupt",
            Self::ChatEnd => "VoiceChatEnd",
            Self::ChatTick => "VoiceChatTick",
            Self::HotkeyTick => "VoiceHotkeyTick",
            Self::Start => "VoiceStart",
            Self::Finish => "VoiceFinish",
            Self::Cancel => "VoiceCancel",
            Self::Apply => "VoiceApply",
            Self::Started(..) => "VoiceStarted",
            Self::Completed(..) => "VoiceCompleted",
            Self::Tick(..) => "VoiceTick",
        })
    }
}

/// Insert into the caller-selected editor; session selection remains a host boundary.
pub fn paste(editor: &mut text_editor::Content, cursor: Option<text_editor::Cursor>, text: String) {
    if let Some(cursor) = cursor {
        editor.move_to(cursor);
    }
    editor.perform(text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(text))));
}
pub fn clear_pending(state: &mut State) {
    state.settings.preview.take();
    state.insertion.pending = None;
    state.hotkeys.machine.reset();
    state.recordings.stop();
}
pub fn fail(state: &mut State) {
    stop(state);
    state.draft.phase = super::voice_draft::Phase::Error;
}
pub fn cloud_error(state: &mut State) -> Option<String> {
    if state.conversation_error_reported {
        return None;
    }
    let error = state.conversation.as_ref()?.snapshot().error?;
    state.conversation_error_reported = true;
    Some(error)
}

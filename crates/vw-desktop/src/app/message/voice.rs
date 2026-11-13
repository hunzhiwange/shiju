//! Dictation and conversation routing. Local audio stays on the client; screen sharing is explicit.
use super::voice_draft::{Composer, Draft, Phase, Scope};
use crate::app::voice::capture::Capture;
use crate::app::{App, Message};
use iced::{Task, widget::text_editor};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use vw_gateway_client::GatewayEndpoint;

#[path = "voice_screen.rs"]
mod screen;
pub use screen::ScreenEvent;

#[path = "voice_dictation.rs"]
pub(super) mod dictation;

#[path = "voice_local_chat.rs"]
pub(crate) mod local_chat;

type Device = Arc<Mutex<Option<Capture>>>;
#[derive(Default)]
pub(crate) struct State {
    pub draft: Draft,
    pub waveform: crate::app::components::input_panel::voice_waveform::Waveform,
    pub screen: crate::app::voice::screen_context::State,
    pub insertion: crate::app::voice::insertion::State,
    pub hotkeys: crate::app::voice::hotkeys::Runtime,
    finish_requested: bool,
    pub recordings: super::voice_recordings::State,
    audio: Option<Vec<u8>>,
    retain_requested: bool,
    pub settings: super::settings::voice::State,
    pub conversation: Option<crate::app::voice::conversation::Conversation>,
    pub local_conversation: Option<local_chat::State>,
    pub local_stop: Option<local_chat::Stop>,
    conversation_scope: Option<Scope>,
    conversation_error_reported: bool,
    conversation_endpoint: Option<GatewayEndpoint>,
    endpoint: Option<GatewayEndpoint>,
    cursor: Option<text_editor::Cursor>,
    device: Option<Device>,
    cancel: Option<tokio::sync::watch::Sender<bool>>,
    route: Option<dictation::Route>,
    native_cancel: Option<crate::app::voice::local::Cancellation>,
}
#[derive(Debug, Clone)]
pub enum OnboardingEvent {
    Open,
    Close,
    Refresh,
    Copy,
    Discard,
    Permission(crate::app::voice::permissions::Permission),
}
#[derive(Clone)]
pub enum Event {
    Local(local_chat::Event),
    Screen(ScreenEvent),
    Onboarding(OnboardingEvent),
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
impl State {
    pub(crate) fn chat_active(&self) -> bool {
        self.conversation.is_some()
            || self.local_conversation.is_some()
            || self.local_stop.is_some()
    }
    pub(crate) fn ready(&self) -> bool {
        self.device.is_some()
    }
}
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
pub(super) fn scope(app: &App) -> Option<Scope> {
    let endpoint = crate::app::gateway_client_endpoint();
    Some(Scope {
        endpoint: endpoint.base_url(),
        tenant: serde_json::to_string(endpoint.tenant_scope()).ok()?,
        project: app.project_path.clone(),
        composer: match &app.active_session_id {
            Some(id) => Composer::Session(id.clone()),
            None => Composer::Draft(app.session_view_cache.navigation_generation),
        },
    })
}
fn stop(state: &mut State) {
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
pub(crate) fn end_conversation(app: &mut App) {
    local_chat::end(app);
    app.voice.screen.cancel();
    app.voice.screen.enabled = false;
    app.voice.conversation.take();
    app.voice.conversation_scope = None;
    app.voice.conversation_endpoint = None;
}
pub(crate) fn cancel(app: &mut App) {
    app.voice.settings.preview.take();
    app.voice.insertion.pending = None;
    app.voice.hotkeys.machine.reset();
    app.voice.recordings.stop();
    end_conversation(app);
    stop(&mut app.voice);
    app.voice.draft.cancel();
    app.voice.endpoint = None;
    app.voice.cursor = None;
}
/// Called before and after dispatch so a switch away and back cannot revive old work.
pub(crate) fn invalidate(app: &mut App) {
    if app.voice.screen.enabled
        && app.voice.local_conversation.is_none()
        && app.voice.conversation.as_ref().is_none_or(|c| {
            matches!(
                c.snapshot().phase,
                crate::app::voice::conversation::Phase::Connecting
                    | crate::app::voice::conversation::Phase::Error
                    | crate::app::voice::conversation::Phase::Ending
            )
        })
    {
        app.voice.screen.cancel();
        app.voice.screen.enabled = false;
    }
    if app.voice.screen.generation.is_some()
        && app.voice.screen.generation
            != app.voice.conversation.as_ref().and_then(|c| c.snapshot().generation)
    {
        app.voice.screen.cancel();
    }
    if app.voice.screen.enabled && !crate::app::voice::screen_context::permitted() {
        app.voice.screen.cancel();
        app.voice.screen.enabled = false;
    }
    super::settings::voice::invalidate(app);
    super::voice_recordings::invalidate(app);
    if app.voice.chat_active() {
        let endpoint = crate::app::gateway_client_endpoint();
        let auth_changed = app.voice.conversation_endpoint.as_ref().is_some_and(|old| {
            old.auth.as_ref().and_then(|a| a.skey.as_ref())
                != endpoint.auth.as_ref().and_then(|a| a.skey.as_ref())
        });
        if app.voice.conversation_scope != scope(app) || auth_changed {
            end_conversation(app);
        }
    }
    if app.voice.draft.scope.is_none() {
        return;
    }
    let current = scope(app);
    let endpoint = crate::app::gateway_client_endpoint();
    let auth_changed = app.voice.endpoint.as_ref().is_some_and(|old| {
        old.auth.as_ref().and_then(|auth| auth.skey.as_ref())
            != endpoint.auth.as_ref().and_then(|auth| auth.skey.as_ref())
    });
    if app.voice.draft.scope.is_some() && (app.voice.draft.scope != current || auth_changed) {
        cancel(app);
    } else if app.voice.draft.scope.is_some() {
        let text = app
            .current_session_runtime_ref()
            .map(|runtime| runtime.input_editor.text())
            .unwrap_or_default();
        app.voice.draft.observe(&text);
    }
}
fn fail(app: &mut App, message: String) -> Task<Message> {
    stop(&mut app.voice);
    app.voice.draft.phase = Phase::Error;
    app.show_toast(crate::app::state::ToastKind::Error, message)
}
fn tick(generation: u64) -> Task<Message> {
    super::after(Duration::from_millis(100), Message::Voice(Event::Tick(generation)))
}
pub(super) fn paste(app: &mut App, text: String, restore_cursor: bool) {
    let cursor = app.voice.cursor;
    let editor = &mut app.current_session_runtime_mut().input_editor;
    if restore_cursor && let Some(cursor) = cursor {
        editor.move_to(cursor);
    }
    editor.perform(text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(text))));
    if app.active_session_id.is_none() {
        app.input_editor = app.current_session_runtime_mut().input_editor.clone();
    }
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    invalidate(app);
    match event {
        Event::Local(event) => local_chat::update(app, event),
        Event::Screen(event) => screen::update(app, event),
        Event::Onboarding(event) => {
            use OnboardingEvent::*;
            match event {
                Open => {
                    app.voice.insertion.onboarding = true;
                    app.voice.insertion.refresh();
                }
                Close => app.voice.insertion.onboarding = false,
                Refresh => app.voice.insertion.refresh(),
                Discard => {
                    if app.voice.insertion.pending.take().is_some() {
                        app.voice.draft.cancel();
                        app.voice.endpoint = None;
                    }
                }
                Copy => {
                    if let Some(text) = &app.voice.insertion.pending {
                        return iced::clipboard::write(text.clone());
                    }
                }
                Permission(permission) => {
                    use crate::app::voice::permissions::{SystemPermissions, VoicePermissions};
                    if let Err(error) = VoicePermissions::new(SystemPermissions)
                        .request_from_user_action(permission)
                    {
                        return app.show_error_toast(error.to_string());
                    }
                    app.voice.insertion.refresh();
                }
            }
            Task::none()
        }
        Event::HotkeyTick => super::voice_hotkeys::tick(app),
        Event::Recordings(event) => super::voice_recordings::update(app, event),
        Event::Transcribed(generation, result) => {
            if app.voice.draft.generation != generation
                || app.voice.draft.phase != Phase::Transcribing
            {
                return Task::none();
            }
            let result = result.map(|(text, wav)| {
                app.voice.audio = Some(wav);
                text
            });
            update(app, Event::Completed(generation, result))
        }
        Event::ChatStart => {
            if app.voice.insertion.pending.is_some() {
                return app.show_error_toast("请先复制或丢弃未插入的跨应用听写结果。");
            }
            if app.voice.recordings.replay.is_some()
                || app.voice.settings.preview.is_some()
                || app.voice.recordings.busy
            {
                return app.show_error_toast("请先停止回放或试听再开始语音聊天。");
            }
            if let Some(chat) = app.voice.local_conversation.as_mut() {
                chat.show = true;
                return Task::none();
            }
            if app.voice.chat_active() {
                return Task::none();
            }
            if matches!(app.voice.draft.phase, Phase::Recording | Phase::Transcribing) {
                return app.show_toast(
                    crate::app::state::ToastKind::Info,
                    "请先结束听写再开始语音聊天。",
                );
            }
            let preferences = match crate::app::voice::preferences::Preferences::load() {
                Ok(p) => p,
                Err(error) => return app.show_error_toast(error),
            };
            if preferences.engine == crate::app::voice::preferences::VoiceEngine::Local {
                return local_chat::start(app, preferences);
            }
            let Some(scope) = scope(app) else {
                return fail(app, "请先选择聊天会话。".into());
            };
            let endpoint = crate::app::gateway_client_endpoint();
            let Some(session) = scope.session_id() else {
                return fail(app, "云端语音聊天需要已建立的聊天会话。".into());
            };
            match crate::app::voice::conversation::Conversation::start(
                endpoint.clone(),
                session.to_owned(),
            ) {
                Ok(conversation) => {
                    app.voice.conversation_error_reported = false;
                    app.voice.conversation = Some(conversation);
                    app.voice.conversation_scope = Some(scope);
                    app.voice.conversation_endpoint = Some(endpoint);
                    Task::none()
                }
                Err(error) => app.show_toast(crate::app::state::ToastKind::Error, error),
            }
        }
        Event::ChatMute => {
            if app.voice.local_conversation.is_some() {
                return local_chat::mute(app);
            }
            if let Some(chat) = &app.voice.conversation {
                chat.mute();
            }
            Task::none()
        }
        Event::ChatInterrupt => {
            if app.voice.local_conversation.is_some() {
                return local_chat::interrupt(app);
            }
            if let Some(chat) = &app.voice.conversation {
                chat.interrupt();
            }
            Task::none()
        }
        Event::ChatEnd => {
            end_conversation(app);
            local_chat::tick(app)
        }
        Event::ChatTick => {
            if app.voice.local_conversation.is_some() || app.voice.local_stop.is_some() {
                return local_chat::tick(app);
            }
            if !app.voice.conversation_error_reported
                && let Some(error) =
                    app.voice.conversation.as_ref().and_then(|c| c.snapshot().error)
            {
                app.voice.conversation_error_reported = true;
                return app.show_toast(crate::app::state::ToastKind::Error, error);
            }
            Task::none()
        }
        Event::Cancel => {
            cancel(app);
            Task::none()
        }
        Event::Start => {
            if app.voice.insertion.pending.is_some() {
                return app.show_error_toast("请先复制或丢弃未插入的跨应用听写结果。");
            }
            if app.voice.recordings.replay.is_some()
                || app.voice.settings.preview.is_some()
                || app.voice.recordings.busy
            {
                return app.show_error_toast("请先结束回放、试听或重新识别再开始听写。");
            }
            if app.voice.chat_active() {
                return app.show_toast(
                    crate::app::state::ToastKind::Info,
                    "请先结束语音聊天再开始听写。",
                );
            }
            let preferences = match crate::app::voice::preferences::Preferences::load() {
                Ok(preferences) => preferences,
                Err(error) => return fail(app, error),
            };
            let Some(scope) = scope(app) else {
                return fail(app, "无法确定当前听写草稿。".into());
            };
            if preferences.engine == crate::app::voice::preferences::VoiceEngine::Cloud
                && scope.session_id().is_none()
            {
                return fail(app, "云端听写需要已建立的聊天会话；本机听写可用于新草稿。".into());
            }
            let runtime = app.current_session_runtime_mut();
            let cursor = runtime.input_editor.cursor();
            let text = runtime.input_editor.text();
            let Some(generation) = app.voice.draft.start(scope.clone(), text) else {
                return Task::none();
            };
            app.voice.waveform.clear();
            app.voice.cursor = Some(cursor);
            app.voice.retain_requested = preferences.retain_recordings;
            let endpoint = crate::app::gateway_client_endpoint();
            app.voice.endpoint = Some(endpoint.clone());
            let (tx, mut rx) = tokio::sync::watch::channel(false);
            app.voice.cancel = Some(tx);
            let cancellation = rx.clone();
            let native_cancel = crate::app::voice::local::Cancellation::default();
            app.voice.native_cancel = Some(native_cancel.clone());
            Task::perform(
                async move {
                    tokio::select! {
                        _ = rx.changed() => Err("听写已取消。".into()),
                        result = tokio::time::timeout(Duration::from_secs(20), dictation::prepare(preferences, endpoint, scope.session_id().map(str::to_owned), generation, cancellation, native_cancel)) => result.unwrap_or_else(|_| Err("准备录音超时，请重试。".into())),
                    }
                },
                move |result| Message::Voice(Event::Started(generation, result)),
            )
        }
        Event::Started(generation, result) => {
            if app.voice.draft.generation != generation
                || app.voice.draft.phase != Phase::Recording
                || app.voice.draft.scope.is_none()
            {
                return Task::none();
            }
            match result {
                Ok(prepared) => {
                    app.voice.device = Some(prepared.device);
                    app.voice.route = Some(prepared.route);
                    if app.voice.finish_requested {
                        return update(app, Event::Finish);
                    }
                    tick(generation)
                }
                Err(error) => fail(app, error),
            }
        }
        Event::Tick(generation) => {
            if generation != app.voice.draft.generation || app.voice.draft.phase != Phase::Recording
            {
                return Task::none();
            }
            if let Some(summary) = app
                .voice
                .device
                .as_ref()
                .and_then(|device| device.lock().ok()?.as_ref().map(|c| c.volume_summary()))
            {
                app.voice.waveform.push(summary.sequence, summary.peak);
            }
            let outcome = app
                .voice
                .device
                .as_ref()
                .and_then(|device| device.lock().ok()?.as_mut()?.try_result());
            if let Some(outcome) = outcome {
                let local_limit = matches!(app.voice.route, Some(dictation::Route::Local { .. }))
                    && matches!(
                        outcome,
                        Err(crate::app::voice::capture::CaptureError::Limit
                            | crate::app::voice::capture::CaptureError::Timeout)
                    );
                return fail(
                    app,
                    if local_limit {
                        "本机听写最多60秒，录音已停止；请重新开始，并在60秒内点击完成。"
                    } else {
                        "录音已停止，请检查麦克风或缩短录音后重试。"
                    }
                    .into(),
                );
            }
            tick(generation)
        }
        Event::Finish => {
            app.voice.hotkeys.machine.reset();
            if app.voice.draft.phase != Phase::Recording {
                return Task::none();
            }
            if app.voice.device.is_none() {
                app.voice.finish_requested = true;
                return Task::none();
            }
            app.voice.finish_requested = false;
            let (Some(device), Some(route)) = (app.voice.device.take(), app.voice.route.take())
            else {
                return fail(app, "录音会话已失效，请重新开始听写。".into());
            };
            app.voice.waveform.clear();
            app.voice.draft.phase = Phase::Transcribing;
            let generation = app.voice.draft.generation;
            let (tx, mut rx) = tokio::sync::watch::channel(false);
            app.voice.cancel = Some(tx);
            let native_cancel = crate::app::voice::local::Cancellation::default();
            app.voice.native_cancel = Some(native_cancel.clone());
            Task::perform(
                async move {
                    tokio::select! {
                        _ = rx.changed() => Err("听写已取消。".into()),
                        result = dictation::transcribe(device, route, native_cancel) => result,
                    }
                },
                move |result| Message::Voice(Event::Transcribed(generation, result)),
            )
        }
        Event::Completed(generation, result) => {
            if app.voice.draft.generation != generation
                || app.voice.draft.phase != Phase::Transcribing
            {
                return Task::none();
            }
            match result {
                Err(error) => fail(app, error),
                Ok(text) => {
                    if text.trim().is_empty() {
                        return fail(app, "没有识别到文字，请重试。".into());
                    }
                    if let Some(target) = app.voice.insertion.target.take() {
                        let outcome = target.and_then(|target| target.insert(&text));
                        let retention_error = retain_completed(&mut app.voice);
                        app.voice.draft.phase = Phase::Idle;
                        stop(&mut app.voice);
                        if let Err(error) = outcome {
                            app.voice.insertion.pending = Some(text);
                            return app.show_error_toast(format!(
                                "{error} 听写结果已保留，请在语音设置的跨应用听写试用中显式复制。"
                            ));
                        }
                        app.voice.draft.cancel();
                        if let Some(error) = retention_error {
                            return app.show_error_toast(error);
                        }
                        return Task::none();
                    }
                    let Some(scope) = scope(app) else {
                        return Task::none();
                    };
                    let current = app
                        .current_session_runtime_ref()
                        .map(|r| r.input_editor.text())
                        .unwrap_or_default();
                    let retention_error = retain_completed(&mut app.voice);
                    if let Some(text) = app.voice.draft.complete(generation, &scope, &current, text)
                    {
                        paste(app, text, true);
                    }
                    stop(&mut app.voice);
                    if let Some(error) = retention_error {
                        return app.show_error_toast(error);
                    }
                    if app.voice.draft.pending.is_some() {
                        return app.show_toast(
                            crate::app::state::ToastKind::Info,
                            "草稿已编辑，听写结果已保留；点击应用听写插入。",
                        );
                    }
                    Task::none()
                }
            }
        }
        Event::Apply => {
            if let Some(text) = app.voice.draft.pending.take() {
                paste(app, text, false);
            }
            cancel(app);
            Task::none()
        }
    }
}
fn retain_completed(state: &mut State) -> Option<String> {
    let audio = state.audio.take()?;
    if !state.retain_requested {
        return None;
    }
    let result = (|| {
        let preferences = crate::app::voice::preferences::Preferences::load()?;
        if preferences.retain_recordings {
            let store = crate::app::voice::recordings::Store::local()?;
            store.save(true, &audio)?;
            state.recordings.entries = store.list()?;
        }
        Ok::<_, String>(())
    })();
    result.err()
}
#[cfg(test)]
#[path = "voice_composer_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "voice_hotkey_lifecycle_tests.rs"]
mod hotkey_lifecycle_tests;

#[cfg(test)]
#[path = "voice_new_draft_tests.rs"]
mod new_draft_tests;

#[cfg(test)]
#[path = "voice_ui_tests.rs"]
mod ui_tests;

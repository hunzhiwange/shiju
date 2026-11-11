//! Composer dictation. Device capture stays local; only WAV bytes reach the gateway.
use super::voice_draft::{Draft, Phase, Scope};
use crate::app::voice::capture::Capture;
use crate::app::{App, Message};
use iced::{Task, widget::text_editor};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use vw_gateway_client::vw_api_types::voice::{TranscriptionRequest, VoiceCounter, VoiceId};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

type Device = Arc<Mutex<Option<Capture>>>;
#[derive(Default)]
pub(crate) struct State {
    pub draft: Draft,
    pub conversation: Option<crate::app::voice::conversation::Conversation>,
    conversation_scope: Option<Scope>,
    conversation_error_reported: bool,
    conversation_endpoint: Option<GatewayEndpoint>,
    endpoint: Option<GatewayEndpoint>,
    cursor: Option<text_editor::Cursor>,
    device: Option<Device>,
    cancel: Option<tokio::sync::watch::Sender<bool>>,
    metadata: Option<TranscriptionRequest>,
}
#[derive(Clone)]
pub enum Event {
    ChatStart,
    ChatMute,
    ChatInterrupt,
    ChatEnd,
    ChatTick,
    Start,
    Finish,
    Cancel,
    Apply,
    Started(u64, Result<(Device, TranscriptionRequest), String>),
    Completed(u64, Result<String, String>),
    Tick(u64),
}
// Do not log transcripts, endpoint credentials or capture payloads through Message::Debug.
impl State {
    pub(crate) fn ready(&self) -> bool {
        self.device.is_some()
    }
}
impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ChatStart => "VoiceChatStart",
            Self::ChatMute => "VoiceChatMute",
            Self::ChatInterrupt => "VoiceChatInterrupt",
            Self::ChatEnd => "VoiceChatEnd",
            Self::ChatTick => "VoiceChatTick",
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
fn scope(app: &App) -> Option<Scope> {
    let endpoint = crate::app::gateway_client_endpoint();
    Some(Scope {
        endpoint: endpoint.base_url(),
        tenant: serde_json::to_string(endpoint.tenant_scope()).ok()?,
        project: app.project_path.clone(),
        session: app.active_session_id.clone()?,
    })
}
fn stop(state: &mut State) {
    if let Some(cancel) = state.cancel.take() {
        let _ = cancel.send(true);
    }
    if let Some(device) = state.device.take()
        && let Ok(mut device) = device.lock()
    {
        drop(device.take());
    }
    state.metadata = None;
}
pub(crate) fn end_conversation(app: &mut App) {
    app.voice.conversation.take();
    app.voice.conversation_scope = None;
    app.voice.conversation_endpoint = None;
}
pub(crate) fn cancel(app: &mut App) {
    end_conversation(app);
    stop(&mut app.voice);
    app.voice.draft.cancel();
    app.voice.endpoint = None;
    app.voice.cursor = None;
}
/// Called before and after dispatch so a switch away and back cannot revive old work.
pub(crate) fn invalidate(app: &mut App) {
    if app.voice.conversation.is_some() {
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
fn paste(app: &mut App, text: String, restore_cursor: bool) {
    let cursor = app.voice.cursor;
    let editor = &mut app.current_session_runtime_mut().input_editor;
    if restore_cursor && let Some(cursor) = cursor {
        editor.move_to(cursor);
    }
    editor.perform(text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(text))));
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    invalidate(app);
    match event {
        Event::ChatStart => {
            if app.voice.conversation.is_some() {
                return Task::none();
            }
            if matches!(app.voice.draft.phase, Phase::Recording | Phase::Transcribing) {
                return app.show_toast(
                    crate::app::state::ToastKind::Info,
                    "请先结束听写再开始语音聊天。",
                );
            }
            let Some(scope) = scope(app) else {
                return fail(app, "请先选择聊天会话。".into());
            };
            let endpoint = crate::app::gateway_client_endpoint();
            match crate::app::voice::conversation::Conversation::start(
                endpoint.clone(),
                scope.session.clone(),
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
            if let Some(chat) = &app.voice.conversation {
                chat.mute();
            }
            Task::none()
        }
        Event::ChatInterrupt => {
            if let Some(chat) = &app.voice.conversation {
                chat.interrupt();
            }
            Task::none()
        }
        Event::ChatEnd => {
            end_conversation(app);
            Task::none()
        }
        Event::ChatTick => {
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
            if app.voice.conversation.is_some() {
                return app.show_toast(
                    crate::app::state::ToastKind::Info,
                    "请先结束语音聊天再开始听写。",
                );
            }
            let Some(scope) = scope(app) else {
                return fail(app, "请先选择聊天会话。".into());
            };
            let Some(runtime) = app.current_session_runtime_ref() else {
                return Task::none();
            };
            let cursor = runtime.input_editor.cursor();
            let text = runtime.input_editor.text();
            let Some(generation) = app.voice.draft.start(scope.clone(), text) else {
                return Task::none();
            };
            app.voice.cursor = Some(cursor);
            let endpoint = crate::app::gateway_client_endpoint();
            app.voice.endpoint = Some(endpoint.clone());
            let (tx, mut rx) = tokio::sync::watch::channel(false);
            app.voice.cancel = Some(tx);
            let cancellation = rx.clone();
            Task::perform(
                async move {
                    tokio::select! {
                        _ = rx.changed() => Err("听写已取消。".into()),
                        result = tokio::time::timeout(Duration::from_secs(20), start(endpoint, scope.session, generation, cancellation)) => result.unwrap_or_else(|_| Err("准备录音超时，请重试。".into())),
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
                Ok((device, metadata)) => {
                    app.voice.device = Some(device);
                    app.voice.metadata = Some(metadata);
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
            let outcome = app
                .voice
                .device
                .as_ref()
                .and_then(|device| device.lock().ok()?.as_mut()?.try_result());
            if outcome.is_some() {
                return fail(app, "录音已停止，请检查麦克风或缩短录音后重试。".into());
            }
            tick(generation)
        }
        Event::Finish => {
            if app.voice.draft.phase != Phase::Recording {
                return Task::none();
            }
            let (Some(device), Some(metadata), Some(endpoint)) =
                (app.voice.device.take(), app.voice.metadata.take(), app.voice.endpoint.clone())
            else {
                return Task::none();
            };
            app.voice.draft.phase = Phase::Transcribing;
            let generation = app.voice.draft.generation;
            let (tx, mut rx) = tokio::sync::watch::channel(false);
            app.voice.cancel = Some(tx);
            Task::perform(
                async move {
                    tokio::select! {
                        _ = rx.changed() => Err("听写已取消。".into()),
                        result = transcribe(device, endpoint, metadata) => result,
                    }
                },
                move |result| Message::Voice(Event::Completed(generation, result)),
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
                    let Some(scope) = scope(app) else {
                        return Task::none();
                    };
                    let current = app
                        .current_session_runtime_ref()
                        .map(|r| r.input_editor.text())
                        .unwrap_or_default();
                    if let Some(text) = app.voice.draft.complete(generation, &scope, &current, text)
                    {
                        paste(app, text, true);
                    }
                    stop(&mut app.voice);
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
async fn start(
    endpoint: GatewayEndpoint,
    session: String,
    generation: u64,
    cancellation: tokio::sync::watch::Receiver<bool>,
) -> Result<(Device, TranscriptionRequest), String> {
    if !endpoint.is_loopback() {
        return Err("当前语音传输仅支持本机网关。".into());
    }
    let client = GatewayClient::new(endpoint).map_err(|_| "无法连接语音网关。")?;
    // Read the tenant's voice section only; never deserialize or log provider credentials.
    let config: vw_config_types::voice::VoiceConfig =
        client.voice_config().await.map_err(|_| "无法读取语音设置。")?;
    if !config.dictation_enabled {
        return Err("请先在网关设置中启用听写。".into());
    }
    let provider = config.provider_id.ok_or("请先配置语音服务。")?;
    let metadata = TranscriptionRequest {
        request_id: VoiceId::try_from(format!("{:032x}", rand::random::<u128>()))
            .map_err(str::to_owned)?,
        chat_session_id: VoiceId::try_from(session).map_err(str::to_owned)?,
        provider_id: VoiceId::try_from(provider).map_err(str::to_owned)?,
        generation: VoiceCounter::try_from(generation).map_err(str::to_owned)?,
    };
    let device = tokio::task::spawn_blocking(move || {
        use crate::app::voice::permissions::{Permission, SystemPermissions, VoicePermissions};
        if *cancellation.borrow() || cancellation.has_changed().is_err() {
            return Err("听写已取消。".into());
        }
        VoicePermissions::new(SystemPermissions)
            .request_from_user_action(Permission::Microphone)
            .map_err(|_| "请在系统设置中允许麦克风访问。".to_owned())?;
        Capture::start(None)
            .and_then(|capture| {
                if *cancellation.borrow() || cancellation.has_changed().is_err() {
                    drop(capture);
                    return Err(crate::app::voice::capture::CaptureError::Cancelled);
                }
                Ok(Arc::new(Mutex::new(Some(capture))))
            })
            .map_err(|_| "无法开始录音，请检查麦克风与系统权限。".to_owned())
    })
    .await
    .map_err(|_| "无法开始录音。")??;
    Ok((device, metadata))
}
async fn transcribe(
    device: Device,
    endpoint: GatewayEndpoint,
    metadata: TranscriptionRequest,
) -> Result<String, String> {
    let wav = tokio::task::spawn_blocking(move || {
        let capture =
            device.lock().map_err(|_| "录音状态不可用。")?.take().ok_or("录音已取消。")?;
        capture
            .finish()
            .and_then(|recording| recording.wav())
            .map_err(|_| "录音无效或没有检测到声音。")
    })
    .await
    .map_err(|_| "无法完成录音。")??;
    let client = GatewayClient::new(endpoint).map_err(|_| "无法连接语音网关。")?;
    tokio::time::timeout(Duration::from_secs(90), client.voice_transcribe(&metadata, wav))
        .await
        .map_err(|_| "听写请求超时。")?
        .map(|result| result.text)
        .map_err(|error| error.to_string())
}
#[cfg(test)]
#[path = "voice_composer_tests.rs"]
mod tests;

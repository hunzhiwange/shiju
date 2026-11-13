//! Per-turn local media, bound to an ordinary chat request and its final completion.
use super::{Device, Event as VoiceEvent, dictation};
use crate::app::{
    App, Message,
    models::{ChatMessage as StoredMessage, ChatRole},
    voice::{local::Cancellation, preferences::Preferences},
};
use iced::Task;
use std::time::Duration;
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    Ready,
    Preparing,
    Recording,
    Recognizing,
    WaitingModel,
    Speaking,
}
pub(crate) struct State {
    pub stage: Stage,
    pub show: bool,
    pub muted: bool,
    pub captions: Vec<(bool, String)>,
    pub notice: Option<String>,
    call: u128,
    round: u64,
    session: String,
    preferences: Preferences,
    native: Cancellation,
    cancel: Option<tokio::sync::watch::Sender<bool>>,
    device: Option<Device>,
    route: Option<dictation::Route>,
    request: Option<u64>,
    watermark: usize,
}
impl State {
    fn cancel_media(&mut self) {
        self.native.cancel();
        if let Some(tx) = self.cancel.take() {
            let _ = tx.send(true);
        }
        if let Some(device) = self.device.take()
            && let Ok(mut d) = device.lock()
        {
            drop(d.take());
        }
        self.route = None;
        self.round = self.round.wrapping_add(1);
        self.stage = Stage::Ready;
    }
    fn caption(&mut self, assistant: bool, text: String) {
        self.captions.push((assistant, text));
        while self.captions.len() > 40
            || self.captions.iter().map(|(_, s)| s.len()).sum::<usize>() > 65_536
        {
            self.captions.remove(0);
        }
    }
}
impl Drop for State {
    fn drop(&mut self) {
        self.cancel_media();
    }
}
#[derive(Clone)]
pub enum Event {
    Begin,
    Finish,
    ViewChat,
    Prepared(u128, u64, Result<dictation::Prepared, String>),
    Recognized(u128, u64, Result<String, String>),
    Spoken(u128, u64, Result<(), String>),
    Stopped(u128, Result<(), String>),
}
pub(crate) struct Stop {
    call: u128,
    endpoint: GatewayEndpoint,
    request: String,
    sent: bool,
}
fn message(event: Event) -> Message {
    Message::Voice(VoiceEvent::Local(event))
}
fn ready_to_send(app: &App) -> Result<(), &'static str> {
    let Some(runtime) = app.current_session_runtime_ref() else {
        return Err("请先选择聊天会话。");
    };
    if runtime.is_requesting || runtime.active_agent_request.is_some() || !runtime.queue.is_empty()
    {
        return Err("请先完成或停止当前聊天请求及排队消息。");
    }
    if !runtime.input_editor.text().trim().is_empty() {
        return Err("请先发送或清空聊天输入框中的草稿。");
    }
    if runtime.task_mode_enabled || runtime.workflow_mode_enabled || runtime.acp_agent.is_some() {
        return Err("本机逐轮语音请使用普通聊天模型，并关闭任务与工作流模式。");
    }
    if runtime.full_access_saving
        || runtime.full_access_save_failed
        || runtime.acp_source_pending.is_some()
        || runtime.acp_options_pending
        || (app.active_session_view_state.ui_preparing && !app.active_session_view_state.base_ready)
    {
        return Err("会话正在加载或保存权限，请处理聊天提示后重试。");
    }
    if app.permission_modal_request_id.is_some() || app.question_modal_request_id.is_some() {
        return Err("请先处理当前聊天的审批或问题。");
    }
    Ok(())
}
pub(super) fn start(app: &mut App, preferences: Preferences) -> Task<Message> {
    if let Some(chat) = app.voice.local_conversation.as_mut() {
        chat.show = true;
        return Task::none();
    }
    if app.voice.local_stop.is_some() {
        return app.show_error_toast("正在停止上一轮聊天，请稍后重试。");
    }
    if let Err(error) =
        preferences.validate().and_then(|_| ready_to_send(app).map_err(str::to_owned))
    {
        return app.show_error_toast(error);
    }
    let Some(scope) = super::scope(app) else {
        return app.show_error_toast("请先选择聊天会话。");
    };
    // This path still owns an existing-session SendToSession request.
    let Some(session) = scope.session_id().map(str::to_owned) else {
        return app.show_error_toast("本机逐轮语音聊天暂不支持未发送的新草稿；听写不受此限制。");
    };
    app.voice.conversation_endpoint = Some(crate::app::gateway_client_endpoint());
    app.voice.conversation_scope = Some(scope.clone());
    app.voice.local_conversation = Some(State {
        stage: Stage::Ready,
        show: true,
        muted: false,
        captions: Vec::new(),
        notice: None,
        call: rand::random(),
        round: 0,
        session,
        preferences,
        native: Cancellation::default(),
        cancel: None,
        device: None,
        route: None,
        request: None,
        watermark: 0,
    });
    Task::none()
}
/// Capture the old authenticated endpoint before scope invalidation clears it.
pub(super) fn end(app: &mut App) {
    if let Some(mut chat) = app.voice.local_conversation.take() {
        stop_owned(app, &mut chat);
    }
}
fn stop_owned(app: &mut App, chat: &mut State) {
    chat.cancel_media();
    if let Some(id) = chat.request.take()
        && let Some(request) = app.get_session_runtime(&chat.session).active_agent_request.as_ref()
        && request.id == id
        && let Some(endpoint) = app.voice.conversation_endpoint.clone()
    {
        app.voice.local_stop = Some(Stop {
            call: chat.call,
            endpoint,
            request: crate::app::impls::desktop_stream_request_id(request),
            sent: false,
        });
    }
}
pub(super) fn tick(app: &mut App) -> Task<Message> {
    if let Some(stop) = app.voice.local_stop.as_mut()
        && !stop.sent
    {
        stop.sent = true;
        let endpoint = stop.endpoint.clone();
        let request = stop.request.clone();
        let call = stop.call;
        return Task::perform(
            async move {
                tokio::time::timeout(Duration::from_secs(5), async {
                    GatewayClient::new(endpoint)?.chat_stream_stop(&request).await
                })
                .await
                .map_err(|_| "停止聊天请求超时，请查看原会话确认。".to_owned())?
                .map(|_| ())
                .map_err(|_| "无法停止聊天请求，请在原会话中重试停止。".to_owned())
            },
            move |result| message(Event::Stopped(call, result)),
        );
    }
    let Some(chat) = app.voice.local_conversation.as_ref() else {
        return Task::none();
    };
    let failed_request = chat.stage == Stage::WaitingModel
        && chat.request.is_some_and(|id| app.find_session_by_request_id(id).is_none());
    let failed_capture = chat.stage == Stage::Recording
        && chat.device.as_ref().is_some_and(|device| {
            device.lock().ok().and_then(|mut d| d.as_mut()?.try_result()).is_some()
        });
    if failed_request || failed_capture {
        let chat = app.voice.local_conversation.as_mut().unwrap();
        chat.cancel_media();
        chat.request = None;
        chat.notice = Some(
            if failed_capture {
                "录音已停止，请检查麦克风并在60秒内完成录音。"
            } else {
                "聊天请求已结束或失败，请查看聊天后重试。"
            }
            .into(),
        );
    }
    Task::none()
}
pub(super) fn mute(app: &mut App) -> Task<Message> {
    if let Some(chat) = app.voice.local_conversation.as_mut() {
        chat.muted = !chat.muted;
        if chat.muted
            && matches!(chat.stage, Stage::Preparing | Stage::Recording | Stage::Recognizing)
        {
            chat.cancel_media();
        }
    }
    Task::none()
}
pub(super) fn interrupt(app: &mut App) -> Task<Message> {
    app.voice.screen.cancel();
    if let Some(mut chat) = app.voice.local_conversation.take() {
        stop_owned(app, &mut chat);
        chat.notice = Some("已打断本轮；可以重新开始录音。".into());
        app.voice.local_conversation = Some(chat);
    }
    tick(app)
}
fn outcome(app: &mut App, error: String) -> Task<Message> {
    if let Some(chat) = app.voice.local_conversation.as_mut() {
        chat.cancel_media();
        chat.notice = Some(error.clone());
    }
    app.show_error_toast(error)
}
pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Stopped(call, result) = event {
        if app.voice.local_stop.as_ref().is_some_and(|s| s.call == call) {
            app.voice.local_stop = None;
            if let Err(error) = result {
                return app.show_error_toast(error);
            }
        }
        return Task::none();
    }
    let Some(chat) = app.voice.local_conversation.as_mut() else {
        return Task::none();
    };
    match event {
        Event::ViewChat => {
            chat.show = false;
            Task::none()
        }
        Event::Begin => {
            if chat.stage != Stage::Ready || chat.muted || app.voice.local_stop.is_some() {
                return Task::none();
            }
            if let Err(error) = ready_to_send(app) {
                return app.show_error_toast(error);
            }
            app.voice.screen.cancel();
            let chat = app.voice.local_conversation.as_mut().unwrap();
            chat.round = chat.round.wrapping_add(1);
            chat.notice = None;
            chat.stage = Stage::Preparing;
            chat.native = Cancellation::default();
            let (tx, rx) = tokio::sync::watch::channel(false);
            chat.cancel = Some(tx);
            let call = chat.call;
            let round = chat.round;
            let future = dictation::prepare(
                chat.preferences.clone(),
                crate::app::gateway_client_endpoint(),
                Some(chat.session.clone()),
                round,
                rx,
                chat.native.clone(),
            );
            Task::perform(
                async move {
                    tokio::time::timeout(Duration::from_secs(20), future)
                        .await
                        .unwrap_or_else(|_| Err("准备本机录音超时，请重试。".into()))
                },
                move |result| message(Event::Prepared(call, round, result)),
            )
        }
        Event::Prepared(call, round, result) => {
            if chat.call != call || chat.round != round || chat.stage != Stage::Preparing {
                return Task::none();
            }
            match result {
                Ok(prepared) => {
                    chat.device = Some(prepared.device);
                    chat.route = Some(prepared.route);
                    chat.stage = Stage::Recording;
                    Task::none()
                }
                Err(error) => outcome(app, error),
            }
        }
        Event::Finish => {
            if chat.stage != Stage::Recording {
                return Task::none();
            }
            let (Some(device), Some(route)) = (chat.device.take(), chat.route.take()) else {
                return outcome(app, "录音已失效，请重新开始。".into());
            };
            if !matches!(route, dictation::Route::Local { .. }) {
                return outcome(app, "本机语音路由不匹配，录音不会上传。".into());
            }
            chat.stage = Stage::Recognizing;
            let call = chat.call;
            let round = chat.round;
            let cancel = chat.native.clone();
            Task::perform(
                async move { dictation::transcribe(device, route, cancel).await.map(|(text, _)| text) },
                move |result| message(Event::Recognized(call, round, result)),
            )
        }
        Event::Recognized(call, round, result) => {
            if chat.call != call || chat.round != round || chat.stage != Stage::Recognizing {
                return Task::none();
            }
            let input = match result {
                Ok(text) if !text.trim().is_empty() && !text.trim_start().starts_with('/') => {
                    text.trim().to_owned()
                }
                Ok(_) => {
                    return outcome(
                        app,
                        "未识别到普通聊天文字；斜杠命令请在聊天输入框中发送。".into(),
                    );
                }
                Err(error) => return outcome(app, error),
            };
            if let Err(error) = ready_to_send(app) {
                return outcome(app, error.into());
            }
            send_request(app, input.clone(), input)
        }

        Event::Spoken(call, round, result) => {
            if chat.call != call || chat.round != round || chat.stage != Stage::Speaking {
                return Task::none();
            }
            chat.cancel_media();
            match result {
                Ok(()) => Task::none(),
                Err(error) => outcome(app, error),
            }
        }
        Event::Stopped(..) => Task::none(),
    }
}
fn send_request(app: &mut App, input: String, caption: String) -> Task<Message> {
    if let Err(error) = ready_to_send(app) {
        return outcome(app, error.into());
    }
    let chat = app.voice.local_conversation.as_mut().unwrap();
    chat.watermark = app.chat.len();
    let session_id = chat.session.clone();
    let task = crate::app::message::chat::update(
        app,
        crate::app::message::chat::ChatMessage::SendToSession {
            session_id: session_id.clone(),
            input,
        },
    );
    let request = app
        .current_session_runtime_ref()
        .and_then(|r| r.active_agent_request.as_ref())
        .map(|r| (r.id, r.session.clone()));
    if let Some((id, session)) = request
        && session == session_id
    {
        let chat = app.voice.local_conversation.as_mut().unwrap();
        chat.request = Some(id);
        chat.stage = Stage::WaitingModel;
        chat.caption(false, caption);
        task
    } else {
        Task::batch([task, outcome(app, "模型未开始，请先查看并处理聊天中的提示。".into())])
    }
}

/// Invoked only after ordinary stream completion flushes all final text.
pub(crate) fn model_completed(
    app: &mut App,
    id: u64,
    session: &str,
    messages: &[StoredMessage],
) -> Task<Message> {
    if app.voice.conversation_scope != super::scope(app) {
        return Task::none();
    }
    let Some(chat) = app.voice.local_conversation.as_mut() else {
        return Task::none();
    };
    if chat.request != Some(id) || chat.session != session || chat.stage != Stage::WaitingModel {
        return Task::none();
    }
    chat.request = None;
    let prose = messages
        .iter()
        .skip(chat.watermark)
        .rfind(|m| m.role == ChatRole::Assistant)
        .map(|m| crate::app::voice::local_conversation::speech_text(&m.content))
        .unwrap_or_default();
    if prose.is_empty() {
        return outcome(app, "回复没有可朗读的正文，请查看聊天。".into());
    }
    let spoken = crate::app::voice::local_conversation::spoken_prefix(&prose);
    chat.notice = (spoken != prose).then(|| "长回复仅朗读开头，完整内容请查看聊天。".into());
    chat.caption(true, prose);
    chat.stage = Stage::Speaking;
    chat.native = Cancellation::default();
    let call = chat.call;
    let round = chat.round;
    Task::perform(
        crate::app::voice::local_conversation::speak(
            spoken,
            chat.preferences.local_voice.clone(),
            chat.native.clone(),
        ),
        move |result| message(Event::Spoken(call, round, result)),
    )
}
#[cfg(test)]
#[path = "voice_local_chat_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "voice_local_screen_tests.rs"]
mod screen_tests;

#[path = "voice_local_screen.rs"]
mod screen;
pub(super) use screen::{screen_call, share_screen};

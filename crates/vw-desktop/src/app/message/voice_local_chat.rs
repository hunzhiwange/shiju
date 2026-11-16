//! Per-turn local media, bound to an ordinary chat request and its final completion.
use super::Event as VoiceEvent;
use crate::app::{
    App, Message,
    models::{ChatMessage as StoredMessage, ChatRole},
    voice::preferences::Preferences,
};
use iced::Task;
use std::time::Duration;
use vw_gateway_client::GatewayClient;
pub use desktop_voice_ui::components::voice_chat::Stage;
pub use desktop_voice_ui::message::voice_local_chat::{State, Stop, Event};
fn message(event: Event) -> Message {
    Message::Voice(VoiceEvent::Local(event))
}
fn ready_to_send(app: &App) -> Result<(), &'static str> {
    use desktop_voice_ui::message::voice_local_chat::{Readiness, ready};
    let runtime = app.current_session_runtime_ref();
    ready(Readiness {
        has_runtime: runtime.is_some(),
        requesting: runtime.is_some_and(|r| r.request_lifecycle.is_requesting || r.request_lifecycle.active_agent_request.is_some() || !r.request_lifecycle.queue.is_empty()),
        has_draft: runtime.is_some_and(|r| !r.input_editor.text().trim().is_empty()),
        special_mode: runtime.is_some_and(|r| r.task_mode_enabled || r.workflow_mode_enabled || r.acp_agent.is_some()),
        permissions_loading: runtime.is_some_and(|r| r.full_access_saving || r.full_access_save_failed || r.acp_options_state.source_pending.is_some() || r.acp_options_state.pending || (app.chat_view.active_session_view_state.ui_preparing && !app.chat_view.active_session_view_state.base_ready)),
        approval_pending: app.session_state.permission_modal_request_id.is_some() || app.session_state.question_modal_request_id.is_some(),
    })
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
    app.voice.local_conversation = Some(desktop_voice_ui::message::voice_local_chat::new(session, preferences));
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
        && let Some(request) = app.get_session_runtime(&chat.session).request_lifecycle.active_agent_request.as_ref()
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
                    gateway_session::GatewaySessionClient::new(GatewayClient::new(endpoint)?.transport().clone()).chat_stream_stop(&request).await
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
    let request_active = chat.request.is_some_and(|id| app.find_session_by_request_id(id).is_some());
    desktop_voice_ui::message::voice_local_chat::observe(app.voice.local_conversation.as_mut().unwrap(), request_active);
    Task::none()
}
pub(super) fn mute(app: &mut App) -> Task<Message> {
    if let Some(chat) = app.voice.local_conversation.as_mut() { desktop_voice_ui::message::voice_local_chat::mute(chat); }
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
    let Some(chat) = app.voice.local_conversation.as_ref() else { return Task::none(); };
    if matches!(event, Event::Begin) {
        if chat.stage != Stage::Ready || chat.muted || app.voice.local_stop.is_some() { return Task::none(); }
        if let Err(error) = ready_to_send(app) { return app.show_error_toast(error); }
        app.voice.screen.cancel();
    }
    let (task, effect) = desktop_voice_ui::message::voice_local_chat::update_media(
        app.voice.local_conversation.as_mut().unwrap(), crate::app::gateway_client_endpoint(), event,
    );
    let task = task.map(message);
    match effect {
        Some(desktop_voice_ui::message::voice_local_chat::Effect::Send(input)) => {
            let send = send_request(app, input.clone(), input);
            Task::batch([task, send])
        }
        Some(desktop_voice_ui::message::voice_local_chat::Effect::Error(error)) => Task::batch([task, app.show_error_toast(error)]),
        None => task,
    }
}

fn send_request(app: &mut App, input: String, caption: String) -> Task<Message> {
    if let Err(error) = ready_to_send(app) {
        return outcome(app, error.into());
    }
    let chat = app.voice.local_conversation.as_mut().unwrap();
    chat.watermark = app.chat_view.chat.len();
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
        .and_then(|r| r.request_lifecycle.active_agent_request.as_ref())
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
    match desktop_voice_ui::message::voice_local_chat::model_completed(chat, id, session, messages.iter().map(|m| (m.role == ChatRole::Assistant, m.content.as_str()))) {
        Ok(task) => task.map(message), Err(error) => outcome(app, error),
    }
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

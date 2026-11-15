//! 子会话缓存与桌面动作接线。
use crate::app::{App, Message, message};
use crate::app::session::AgentChildSession;
use iced::Element;
use desktop_chat_input::input_panel::agent_sessions as shared;
#[cfg(test)]
pub(super) use shared::{agent_session_status_label, agent_session_tooltip_text, agent_sessions_badge_label};
fn emit(action: shared::Action) -> Message {
    Message::Chat(match action {
        shared::Action::StopPlanRun(task) => message::ChatMessage::StopPlanRun(task),
        shared::Action::ToggleAgentSessionsPopover => message::ChatMessage::ToggleAgentSessionsPopover,
        shared::Action::CloseAgentSessionsPopover => message::ChatMessage::CloseAgentSessionsPopover,
    })
}
pub(crate) fn cached_running_agent_sessions(app: &App) -> std::sync::Arc<Vec<AgentChildSession>> {
    let mut cached = app.session_ui_projection.agent_sessions.borrow_mut();
    let mut key = app.session_ui_projection.agent_session_key.borrow_mut();
    if key.0.as_deref() != app.active_session_id.as_deref() || key.1 != app.chat.len() {
        cached.take();
        *key = (app.active_session_id.clone(), app.chat.len());
    }
    cached.get_or_insert_with(|| std::sync::Arc::new(running_agent_sessions(app))).clone()
}


pub(crate) fn running_agent_sessions(app: &App) -> Vec<AgentChildSession> {
    shared::running_agent_sessions(app.active_session_id.as_deref(), &app.chat,
        app.sessions.iter().chain(app.project_sessions.values().flatten()),
        &|id| app.cached_chat_messages(id),
        &|id| app.session_runtime_states.get(id).is_some_and(|r| r.request_lifecycle.is_requesting),
        &|id, raw| crate::app::i18n::other::session_title(app, id, raw))
}
#[cfg(test)]
pub(super) fn agent_session_stop_message(child: &AgentChildSession) -> Option<Message> { shared::agent_session_stop_message(child).map(emit) }
#[cfg(test)]
pub(super) fn agent_session_row<'a>(app: &App, child: &AgentChildSession) -> Element<'a, Message> {
    shared::agent_session_row(crate::app::i18n::I18n::new(app.desktop_locale), app.status_animation_frame, child, emit)
}
#[cfg(test)]
pub(super) fn agent_sessions_popover<'a>(app: &App, children: &[AgentChildSession]) -> Element<'a, Message> {
    shared::agent_sessions_popover(crate::app::i18n::I18n::new(app.desktop_locale), app.status_animation_frame, children, emit)
}
pub(super) fn agent_sessions_entry<'a>(app: &App, children: &[AgentChildSession]) -> Option<Element<'a, Message>> {
    shared::agent_sessions_entry(crate::app::i18n::I18n::new(app.desktop_locale), app.status_animation_frame, app.input_popovers.agent_sessions, children, emit)
}

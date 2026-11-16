//! Desktop orchestration and message mapping for send controls.
use crate::app::{App, Message, message};
use iced::Element;
#[cfg(test)]
use crate::app::state::ChatSendBehavior;
use crate::app::task::TaskSplitMode;
use vw_api_chat_identity::ReasoningEffortDto;
use desktop_chat_input::input_panel::{send_controls as controls, send_message::Message as Event};
#[cfg(test)]
pub(super) use controls::{is_dark_theme, permission_access_button_style, prominent_action_background, prominent_action_foreground, prominent_action_style, send_behavior_icon};
pub use controls::bottom_bar;
fn map(event: Event) -> Message {
    match event {
        Event::SelectChatSendBehavior(value) => Message::View(message::ViewMessage::SelectChatSendBehavior(value)),
        Event::ToggleSendModePopover => Message::View(message::ViewMessage::ToggleSendModePopover),
        Event::CloseSendModePopover => Message::View(message::ViewMessage::CloseSendModePopover),
        Event::ToggleFullAccessPermission => Message::Chat(message::ChatMessage::ToggleFullAccessPermission),
        Event::WorkflowModeToggled(value) => Message::Chat(message::ChatMessage::WorkflowModeToggled(value)),
        Event::SendPressed => Message::Chat(message::ChatMessage::SendPressed),
        Event::CancelPressed => Message::Chat(message::ChatMessage::CancelPressed),
        #[cfg(not(target_arch = "wasm32"))]
        Event::VoiceChatStart => Message::Voice(crate::app::message::voice::Event::ChatStart),
        Event::AddTaskFromInput(value) => Message::TaskBoard(message::TaskBoardMessage::AddTaskFromInput(value)),
        Event::AddTaskFromInputWithOptions { content, priority, model, reasoning_effort, task_split_mode, subtasks } => Message::TaskBoard(message::TaskBoardMessage::AddTaskFromInputWithOptions { content, priority, model, reasoning_effort, task_split_mode, subtasks }),
    }
}
pub(super) fn has_permission_context(app: &App) -> bool {
    app.active_session_id
        .as_deref()
        .and_then(|session_id| app.known_session_directory(session_id))
        .is_some_and(|directory| !directory.trim().is_empty())
        || app.navigation_tabs.project_path.as_deref().is_some_and(|directory| !directory.trim().is_empty())
}

pub fn send_button(app: &App, enabled: bool, can_send: bool, is_requesting: bool) -> Element<'_, Message> {
    #[cfg(not(target_arch = "wasm32"))]
    let voice_available = !app.voice.chat_active()
        && !matches!(
            app.voice.draft.phase,
            crate::app::message::voice_draft::Phase::Recording
                | crate::app::message::voice_draft::Phase::Transcribing
        )
        && {
            let runtime = app.current_session_runtime();
            !runtime.task_mode_enabled
                && !runtime.workflow_mode_enabled
                && runtime.acp_agent.is_none()
        };
    #[cfg(target_arch = "wasm32")]
    let voice_available = false;
    controls::send_button(crate::app::i18n::I18n::new(app.desktop_locale), app.chat_send_behavior, app.popover_visibility.send_mode, voice_available, enabled, can_send, is_requesting).map(map)
}
pub fn pool_button(
    i18n: crate::app::i18n::I18n,
    enabled: bool,
    task_mode_enabled: bool,
    input_text: String,
    task_mode_priority: String,
    task_mode_model: String,
    task_mode_reasoning_effort: Option<ReasoningEffortDto>,
    task_mode_task_split_mode: TaskSplitMode,
    task_mode_subtasks: Vec<String>,
) -> Element<'static, Message> { controls::pool_button(i18n, enabled, task_mode_enabled, input_text, task_mode_priority, task_mode_model, task_mode_reasoning_effort, task_mode_task_split_mode, task_mode_subtasks).map(map) }
pub fn acp_permission_modes(
    full_access: bool,
    can_change: bool,
) -> Element<'static, Message> { controls::acp_permission_modes(full_access, can_change).map(map) }
pub fn full_access_button(
    i18n: crate::app::i18n::I18n,
    enabled: bool,
    active: bool,
) -> Element<'static, Message> { controls::full_access_button(i18n, enabled, active).map(map) }
pub fn workflow_mode_button(
    i18n: crate::app::i18n::I18n,
    active: bool,
) -> Element<'static, Message> { controls::workflow_mode_button(i18n, active).map(map) }
pub fn cancel_button(i18n: crate::app::i18n::I18n, submit_anim: u8) -> Element<'static, Message> { controls::cancel_button(i18n, submit_anim).map(map) }
#[cfg(test)]
pub fn send_behavior_popover(
    i18n: crate::app::i18n::I18n,
    selected: ChatSendBehavior,
) -> Element<'static, Message> { controls::send_behavior_popover(i18n, selected).map(map) }

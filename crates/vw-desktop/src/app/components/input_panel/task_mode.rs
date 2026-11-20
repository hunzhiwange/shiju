use crate::app::{App, Message, message};
use crate::app::task::TaskSplitMode;
use vw_api_chat_identity::ReasoningEffortDto;
use iced::Element;
use desktop_chat_input::input_panel::task_mode as domain;
fn map_message(value: domain::Message) -> Message {
 use domain::Message as Local;
 match value {
 Local::None => Message::None,
 Local::Copy(text) => Message::CopyCode(text),
 Local::Append(text) => crate::app::components::text_editor_context_menu::text_context_message(desktop_ui_controls::text_editor_context_menu::TextContextEffect::Append(text)),
 Local::OpenUrl(url) => crate::app::components::text_editor_context_menu::text_context_message(desktop_ui_controls::text_editor_context_menu::TextContextEffect::OpenUrl(url)),
 Local::Selection { session_id, message_idx, text_idx, text } => Message::ChatMarkdownSelectionChanged { session_id, message_idx, text_idx, text },
 Local::PriorityChanged(v) => Message::Chat(message::ChatMessage::TaskModePriorityChanged(v)),
 Local::ModelChanged(v) => Message::Chat(message::ChatMessage::TaskModeModelChanged(v)),
 Local::TaskSplitModeChanged(v) => Message::Chat(message::ChatMessage::TaskModeTaskSplitModeChanged(v)),
 Local::ExecutorChanged(v) => Message::Chat(message::ChatMessage::TaskModeExecutorChanged(v)),
 Local::ReasoningEffortChanged(v) => Message::Chat(message::ChatMessage::TaskModeReasoningEffortChanged(v)),
 Local::ToggleExecutorPopover => Message::View(message::ViewMessage::ToggleExecutorPopover),
 Local::CloseExecutorPopover => Message::View(message::ViewMessage::CloseExecutorPopover),
}
}
pub fn task_mode_form<'a>(
    app: &'a App,
    task_mode_enabled: bool,
    task_mode_priority: &str,
    task_mode_model: &str,
    task_mode_task_split_mode: TaskSplitMode,
    task_mode_executor: Option<String>,
    task_mode_reasoning_effort: Option<ReasoningEffortDto>,
) -> Element<'a, Message> {
domain::task_mode_form(domain::Input {
 i18n: crate::app::i18n::I18n::new(app.general_settings.desktop_locale), theme: &app.appearance.app_theme,
 providers: &app.model_settings.providers, agents: &app.session_state.acp_agents, expanded: app.popover_visibility.executor,
}, task_mode_enabled, task_mode_priority,
 task_mode_model, task_mode_task_split_mode, task_mode_executor, task_mode_reasoning_effort).map(map_message)
}

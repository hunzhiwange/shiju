use crate::app::{App, Message, message};
use crate::app::state::SessionRuntimeState;
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
 Local::SubtaskEditorAction { index, action } => Message::Chat(message::ChatMessage::TaskModeSubtaskEditorAction { index, action }),
 Local::SubtaskChanged { index, value } => Message::Chat(message::ChatMessage::TaskModeSubtaskChanged { index, value }),
 Local::MoveSubtaskUp(v) => Message::Chat(message::ChatMessage::TaskModeMoveSubtaskUp(v)),
 Local::MoveSubtaskDown(v) => Message::Chat(message::ChatMessage::TaskModeMoveSubtaskDown(v)),
 Local::RemoveSubtask(v) => Message::Chat(message::ChatMessage::TaskModeRemoveSubtask(v)),
 Local::AddSubtask => Message::Chat(message::ChatMessage::TaskModeAddSubtask),
 Local::ExecutorChanged(v) => Message::Chat(message::ChatMessage::TaskModeExecutorChanged(v)),
 Local::ReasoningEffortChanged(v) => Message::Chat(message::ChatMessage::TaskModeReasoningEffortChanged(v)),
 Local::ToggleExecutorPopover => Message::View(message::ViewMessage::ToggleExecutorPopover),
 Local::CloseExecutorPopover => Message::View(message::ViewMessage::CloseExecutorPopover),
}
}
pub fn task_mode_form<'a>(
    app: &'a App,
    runtime: Option<&'a SessionRuntimeState>,
    task_mode_enabled: bool,
    task_mode_priority: &str,
    task_mode_model: &str,
    task_mode_task_split_mode: TaskSplitMode,
    task_mode_executor: Option<String>,
    task_mode_reasoning_effort: Option<ReasoningEffortDto>,
    task_mode_subtasks: &'a [String],
) -> Element<'a, Message> {
domain::task_mode_form(domain::Input {
 i18n: crate::app::i18n::I18n::new(app.desktop_locale), theme: &app.app_theme,
 providers: &app.model_settings.providers, agents: &app.acp_agents, expanded: app.show_executor_popover,
}, runtime.map(|r| r.task_mode_subtask_editors.as_slice()), task_mode_enabled, task_mode_priority,
 task_mode_model, task_mode_task_split_mode, task_mode_executor, task_mode_reasoning_effort, task_mode_subtasks).map(map_message)
}

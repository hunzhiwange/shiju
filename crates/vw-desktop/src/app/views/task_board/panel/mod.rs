use crate::app::{App, Message};
mod executor_selector;
mod logs_modal;
mod model_selector;
mod reasoning_selector;
mod styles;
pub(crate) use executor_selector::build_bulk_executor_selector;
pub(crate) use model_selector::build_bulk_model_selector;
pub(crate) use reasoning_selector::build_bulk_reasoning_effort_selector;
fn input(app: &App) -> desktop_task_board::panel::PanelView<'_> {
 desktop_task_board::panel::PanelView {
 app_theme: &app.app_theme,
 desktop_locale: app.desktop_locale,
 agents_settings: &app.agents_settings,
 project_path: &app.navigation_tabs.project_path,
 task_board_bulk_agent: &app.task_board_bulk_agent,
 task_board_bulk_task_split_mode: app.task_board_bulk_task_split_mode,
 task_board_detail_tab: app.task_board_detail_tab,
 task_board_draft: &app.task_board_draft,
 task_board_import_prompt_format: app.task_board_import_prompt_format,
 task_board_settings: &app.task_board_settings,
 task_board_viewing_logs: &app.task_board_viewing_logs,
 task_board_worktree_snapshot: &app.task_board_worktree_snapshot,
 task_board_logs_editor_id: &app.task_board_logs_editor_id,
 task_board_logs_context_menu_pos: app.task_board_logs_context_menu_pos,
 current_line_height: app.current_line_height,
 task_board_logs_scroll_top_line: app.task_board_logs_scroll_top_line,
 task_board_new_subtask_content: &app.task_board_new_subtask_content,
 task_board_import_editor: &app.task_board_import_editor,
 task_board_prompt_editor: &app.task_board_prompt_editor,
 task_board_logs_editor: &app.task_board_logs_editor,
 task_board_clear_prompt_after_create: app.task_board_clear_prompt_after_create,
 task_board_close_after_create: app.task_board_close_after_create,
 task_board_close_after_edit: app.task_board_close_after_edit,
 task_board_create_submit_success: app.task_board_create_submit_success,
 task_board_edit_submit_success: app.task_board_edit_submit_success,
 task_board_import_prompt_collapsed: app.task_board_import_prompt_collapsed,
 task_board_is_import_mode: app.task_board_is_import_mode,
 task_board_logs_context_menu_open: app.task_board_logs_context_menu_open,
 }
}
fn map(message: desktop_task_board::panel::PanelMessage) -> Message {
 match message { desktop_task_board::panel::PanelMessage::TaskBoard(m) => Message::TaskBoard(m), desktop_task_board::panel::PanelMessage::Model(m) => model_selector::map(m),
 desktop_task_board::panel::PanelMessage::None => Message::None,
 desktop_task_board::panel::PanelMessage::Copy(s) => Message::CopyCode(s),
 desktop_task_board::panel::PanelMessage::Append(s) => crate::app::components::text_editor_context_menu::text_context_message(desktop_ui_controls::text_editor_context_menu::TextContextEffect::Append(s)),
 desktop_task_board::panel::PanelMessage::OpenUrl(s) => crate::app::components::text_editor_context_menu::text_context_message(desktop_ui_controls::text_editor_context_menu::TextContextEffect::OpenUrl(s)),
 desktop_task_board::panel::PanelMessage::Selection { session_id, message_idx, text_idx, text } => Message::ChatMarkdownSelectionChanged { session_id, message_idx, text_idx, text } }
}
pub fn build_task_panel(app: &App, edit: bool) -> iced::Element<'_, Message> {
 desktop_task_board::panel::build_with_controls(
 input(app), edit, model_selector::view(app), executor_selector::input(app),
 || reasoning_selector::domain_build(app, edit, false),
 ).map(map)
}
pub(crate) fn build_bulk_task_split_selector(app: &App) -> iced::Element<'_, Message> { desktop_task_board::panel::build_bulk_task_split_selector(input(app)).map(map) }
pub(crate) fn build_bulk_agent_selector(app: &App) -> iced::Element<'_, Message> { desktop_task_board::panel::build_bulk_agent_selector(input(app)).map(map) }

#[cfg(test)]
mod tests;

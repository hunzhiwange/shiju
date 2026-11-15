use crate::app::{App, Message};
use crate::app::message::TaskBoardMessage;
use crate::app::state::TaskBoardViewMode;
mod board;
mod common;
mod control;
mod drag;
mod modals;
mod panel;
pub fn view(app: &App) -> iced::Element<'_, Message> {
 let input = desktop_task_board::page::PageView { task_board_create_modal_open: app.task_board_create_modal_open, task_board_worktree_pixel_office: app.task_board_worktree_pixel_office, kanban: app.task_board_view_mode == TaskBoardViewMode::Kanban, viewing_logs: app.task_board_viewing_logs.is_some(), task_board_settings_modal_open: app.task_board_settings_modal_open };
 desktop_task_board::page::view(input, move |width| {
 desktop_task_board::page::build_view(input, width, desktop_task_board::page::PageSlots {
 header: control::build_control_buttons(app), board: (!input.task_board_worktree_pixel_office).then(|| board::build_kanban_board(app)), drag: drag::build_drag_preview_layer(app),
 create_panel: width.is_some().then(|| panel::build_task_panel(app, false)), detail_panel: (width.is_none() && input.viewing_logs).then(|| panel::build_task_panel(app, true)), settings: input.task_board_settings_modal_open.then(|| modals::build_task_settings_modal(app)),
 context_closed: Message::TaskBoard(TaskBoardMessage::ContextMenuClosed), close_logs: Message::TaskBoard(TaskBoardMessage::CloseTaskLogs), close_settings: Message::TaskBoard(TaskBoardMessage::CloseSettingsModal),
 }) })
}

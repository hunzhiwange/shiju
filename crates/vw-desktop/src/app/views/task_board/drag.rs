use crate::app::{App, Message};
pub fn build_drag_preview_layer(app: &App) -> iced::Element<'_, Message> {
 desktop_task_board::drag::build_drag_preview_layer(desktop_task_board::drag::DragView {
 task_board_dragging: &app.task_board.task_board_dragging, task_board_tasks: &app.task_board.task_board_tasks,
 desktop_locale: app.general_settings.desktop_locale, cursor_position: app.cursor_position, window_size: app.window_size,
 }).map(Message::TaskBoard)
}

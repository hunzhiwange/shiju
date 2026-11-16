//! 日志领域状态借用与桌面接线。
use super::*;
pub(crate) fn sync_task_logs_editor(app: &mut crate::app::App, task: Option<&Task>) {
    desktop_task_board::helpers::logs::sync_task_logs_editor(&mut logs_state(app), task)
}

pub(crate) fn refresh_task_log_cache(app: &mut crate::app::App, task: &Task) {
    desktop_task_board::helpers::logs::refresh_task_log_cache(&mut logs_state(app), task)
}

pub(crate) fn set_viewing_logs(app: &mut crate::app::App, task: Option<Task>) {
    desktop_task_board::helpers::logs::set_viewing_logs(&mut logs_state(app), task)
}
pub(crate) fn sync_viewing_logs(app: &mut crate::app::App, task_id: &str) {
    desktop_task_board::helpers::logs::sync_viewing_logs(&mut logs_state(app), task_id)
}
pub(crate) fn close_logs_context_menu(app: &mut crate::app::App) {
    desktop_task_board::helpers::logs::close_logs_context_menu(&mut logs_state(app))
}

fn logs_state(app: &mut crate::app::App) -> desktop_task_board::helpers::logs::TaskLogsState<'_> {
    desktop_task_board::helpers::logs::TaskLogsState {
        task_board_logs_editor: &mut app.task_board.task_board_logs_editor,
        task_board_logs_auto_scroll: &mut app.task_board.task_board_logs_auto_scroll,
        task_board_logs_scroll_top_line: &mut app.task_board.task_board_logs_scroll_top_line,
        task_board_logs_scroll_remainder: &mut app.task_board.task_board_logs_scroll_remainder,
        task_board_logs_viewport_height: &mut app.task_board.task_board_logs_viewport_height,
        current_line_height: &mut app.preview_state.current_line_height,
        desktop_locale: &mut app.general_settings.desktop_locale,
        task_board_log_cache: &mut app.task_board.task_board_log_cache,
        task_board_tasks: &mut app.task_board.task_board_tasks,
        task_board_viewing_logs: &mut app.task_board.task_board_viewing_logs,
        task_board_logs_context_menu_open: &mut app.task_board.task_board_logs_context_menu_open,
        task_board_logs_context_menu_pos: &mut app.task_board.task_board_logs_context_menu_pos,
    }
}

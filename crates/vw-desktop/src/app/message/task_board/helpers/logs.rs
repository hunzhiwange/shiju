//! 日志领域状态借用与桌面接线。
use super::*;
pub(crate) fn sync_task_logs_editor(app: &mut crate::app::App, task: Option<&Task>) {
 desktop_task_board::helpers::logs::sync_task_logs_editor(&mut logs_state(app), task)
}
pub(crate) fn apply_cached_logs_to_task(app: &mut crate::app::App, task: &mut Task) {
 desktop_task_board::helpers::logs::apply_cached_logs_to_task(&mut logs_state(app), task)
}
pub(crate) fn refresh_task_log_cache(app: &mut crate::app::App, task: &Task) {
 desktop_task_board::helpers::logs::refresh_task_log_cache(&mut logs_state(app), task)
}
pub(crate) fn refresh_task_log_cache_by_id(app: &mut crate::app::App, task_id: &str) {
 desktop_task_board::helpers::logs::refresh_task_log_cache_by_id(&mut logs_state(app), task_id)
}
pub(crate) fn sync_task_log_cache_for_loaded_tasks(app: &mut crate::app::App) {
 desktop_task_board::helpers::logs::sync_task_log_cache_for_loaded_tasks(&mut logs_state(app))
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
pub(crate) fn task_logs_max_scroll_top_line(app: &crate::app::App) -> f32 {
 desktop_task_board::helpers::logs::max_scroll_top_line(app.task_board_logs_viewport_height, app.current_line_height, app.task_board_logs_editor.line_count())
}
pub(crate) fn apply_task_logs_scroll_lines(app: &mut crate::app::App, delta_lines: i32) {
 desktop_task_board::helpers::logs::apply_task_logs_scroll_lines(&mut logs_state(app), delta_lines)
}
pub(crate) fn restore_task_logs_scroll(app: &mut crate::app::App, target_top_line: f32) {
 desktop_task_board::helpers::logs::restore_task_logs_scroll(&mut logs_state(app), target_top_line)
}
pub(crate) fn scroll_task_logs_to_bottom(app: &mut crate::app::App) {
 desktop_task_board::helpers::logs::scroll_task_logs_to_bottom(&mut logs_state(app))
}
#[cfg(test)]
#[path = "logs_tests.rs"]
mod logs_tests;

fn logs_state(app: &mut crate::app::App) -> desktop_task_board::helpers::logs::TaskLogsState<'_> {
    desktop_task_board::helpers::logs::TaskLogsState {
        task_board_logs_editor: &mut app.task_board_logs_editor,
        task_board_logs_auto_scroll: &mut app.task_board_logs_auto_scroll,
        task_board_logs_scroll_top_line: &mut app.task_board_logs_scroll_top_line,
        task_board_logs_scroll_remainder: &mut app.task_board_logs_scroll_remainder,
        task_board_logs_viewport_height: &mut app.task_board_logs_viewport_height,
        current_line_height: &mut app.current_line_height,
        desktop_locale: &mut app.desktop_locale,
        task_board_log_cache: &mut app.task_board_log_cache,
        task_board_tasks: &mut app.task_board_tasks,
        task_board_viewing_logs: &mut app.task_board_viewing_logs,
        task_board_logs_context_menu_open: &mut app.task_board_logs_context_menu_open,
        task_board_logs_context_menu_pos: &mut app.task_board_logs_context_menu_pos,
    }
}

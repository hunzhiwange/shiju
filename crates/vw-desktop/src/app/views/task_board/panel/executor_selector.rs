use crate::app::{App, Message};
pub(super) fn input(app: &App) -> desktop_task_board::panel::executor_selector::ExecutorView<'_> {
 desktop_task_board::panel::executor_selector::ExecutorView {
 task_board_draft: &app.task_board_draft, task_board_bulk_acp_agent: &app.task_board_bulk_acp_agent,
 task_board_executor_popover: app.task_board_executor_popover, task_board_bulk_executor_popover: app.task_board_bulk_executor_popover,
 desktop_locale: app.desktop_locale, acp_agents: &app.acp_agents,
 }
}
pub fn build_bulk_executor_selector(app: &App) -> iced::Element<'_, Message> {
 desktop_task_board::panel::executor_selector::build_bulk_executor_selector(input(app)).map(Message::TaskBoard)
}
#[cfg(test)]
#[path = "executor_selector_ui_refresh_35_tests.rs"]
mod executor_selector_ui_refresh_35_tests;

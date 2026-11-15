use crate::app::{App, Message};
use crate::app::task::Task;
pub fn build_task_settings_modal(app: &App) -> iced::Element<'_, Message> {
 desktop_task_board::modals::build_task_settings_modal(desktop_task_board::modals::SettingsView {
 task_board_settings: &app.task_board_settings, task_board_settings_modal_tab: app.task_board_settings_modal_tab,
 desktop_locale: app.desktop_locale, app_theme: &app.app_theme,
 }, Message::TaskBoard, |i18n, theme, title, description| crate::app::components::system_settings_common::settings_group_heading_with_help(i18n, theme, title, description))
}
pub fn build_context_menu(i18n: crate::app::i18n::I18n, task: Task) -> iced::Element<'static, Message> {
 desktop_task_board::modals::build_context_menu(i18n, task).map(Message::TaskBoard)
}
#[cfg(test)]
#[path = "modals_tests.rs"]
pub(super) mod modals_tests;

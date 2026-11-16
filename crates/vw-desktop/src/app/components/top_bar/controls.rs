use crate::app::{App, Message};
use desktop_navigation::components::top_bar::controls as domain;
pub(super) fn settings_button(i18n: crate::app::i18n::I18n) -> iced::Element<'static, Message> { domain::settings_button(i18n).map(super::mapping::map_message) }
pub(super) fn project_view_tools(app: &App) -> iced::Element<'_, Message> {
 domain::project_view_tools(&domain::State {
 screen: app.screen, workspace_mode: app.navigation_mode.workspace_mode, desktop_locale: app.desktop_locale,
 show_settings: app.show_settings, show_diff: app.show_diff, show_file_manager: app.file_manager.show_file_manager,
 work_sidebar_collapsed: app.navigation_sidebar.work_sidebar_collapsed, work_resources_visible: app.navigation_workspace.work_resources_visible,
 terminal_visible: app.terminal.is_visible,
 #[cfg(not(target_arch = "wasm32"))] pet_visible: app.task_pet_window_id.is_some(),
 #[cfg(target_arch = "wasm32")] pet_visible: false,
 }).map(super::mapping::map_message)
}

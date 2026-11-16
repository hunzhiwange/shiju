//! 宿主输入装配与领域消息映射。
use crate::app::{App, Message};
use iced::Element;
use desktop_settings_runtime::system_settings_runtime as domain;
fn inputs(app: &App) -> domain::View<'_> {
 domain::View { runtime_settings: &app.runtime_settings,
select_focus_scope: super::system_settings::SELECT_FOCUS_SCOPE,
 desktop_locale: app.general_settings.desktop_locale, app_theme: &app.appearance.app_theme,

 render_help: super::system_settings_hooks::render_help,
 }
}
pub fn view(app: &App) -> Element<'_, Message> {
 domain::view(inputs(app)).map(super::system_settings_hooks::map_message)
}
#[cfg(test)]
#[path = "system_settings_runtime_tests.rs"]
mod system_settings_runtime_tests;

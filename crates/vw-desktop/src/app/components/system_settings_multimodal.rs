//! 宿主输入装配与领域消息映射。
use crate::app::{App, Message};
use iced::Element;
use desktop_settings_runtime::system_settings_multimodal as domain;
fn inputs(app: &App) -> domain::View<'_> {
 domain::View { multimodal_settings: &app.multimodal_settings,
model_settings: &app.model_settings,
 desktop_locale: app.desktop_locale, app_theme: &app.app_theme,

 render_help: super::system_settings_hooks::render_help,
 }
}
pub fn view(app: &App) -> Element<'_, Message> {
 domain::view(inputs(app)).map(super::system_settings_hooks::map_message)
}
#[cfg(test)]
#[path = "system_settings_multimodal_tests.rs"]
pub(super) mod system_settings_multimodal_tests;

//! 宿主输入装配与领域消息映射。
use crate::app::{App, Message};
use iced::Element;
use desktop_settings_runtime::system_settings_query_classification as domain;
fn inputs(app: &App) -> domain::View<'_> {
 domain::View { query_classification_settings: &app.query_classification_settings,
model_routes: &app.model_routes_settings.routes,
 desktop_locale: app.general_settings.desktop_locale, app_theme: &app.appearance.app_theme,

 render_help: super::system_settings_hooks::render_help,
 }
}
pub fn view(app: &App) -> Element<'_, Message> {
 domain::view(inputs(app)).map(super::system_settings_hooks::map_message)
}

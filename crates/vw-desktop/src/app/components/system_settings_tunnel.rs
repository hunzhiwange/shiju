//! 网络设置页输入装配与宿主消息映射。
use crate::app::{App, Message};
use iced::Element;
use desktop_settings_network::system_settings_tunnel as domain;
fn inputs(app: &App) -> domain::View<'_> {
    domain::View {
        tunnel_settings: &app.tunnel_settings,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
        render_help: super::system_settings_browser::render_help,
    }
}
pub fn view(app: &App) -> Element<'_, Message> { domain::view(inputs(app)).map(super::system_settings_browser::map_message) }
pub fn content(app: &App) -> Element<'_, Message> { domain::content(inputs(app)).map(super::system_settings_browser::map_message) }
#[cfg(test)]
#[path = "system_settings_tunnel_tests.rs"]
pub(super) mod system_settings_tunnel_tests;

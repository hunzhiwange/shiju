//! 网络设置页输入装配与宿主消息映射。
use crate::app::{App, Message};
use iced::Element;
use desktop_settings_network::system_settings_proxy as domain;
fn inputs(app: &App) -> domain::View<'_> {
    domain::View {
        proxy_settings: &app.proxy_settings,
        desktop_locale: app.general_settings.desktop_locale,
        app_theme: &app.appearance.app_theme,
        render_help: super::system_settings_browser::render_help,
        last_copied_code_hash: app.clipboard_state.last_copied_code_hash,
    }
}
pub fn view(app: &App) -> Element<'_, Message> { domain::view(inputs(app)).map(super::system_settings_browser::map_message) }
pub fn content(app: &App) -> Element<'_, Message> { domain::content(inputs(app)).map(super::system_settings_browser::map_message) }
#[cfg(test)]
#[path = "system_settings_proxy_tests.rs"]
pub(super) mod system_settings_proxy_tests;

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> { domain::view_overlays(inputs(app), dialog, super::system_settings_browser::map_message) }

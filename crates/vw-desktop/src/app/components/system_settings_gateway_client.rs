//! 网络设置页的宿主消息映射与输入装配。
use crate::app::{App, Message};
use iced::Element;
use desktop_settings_network::system_settings_gateway_client as domain;

fn inputs(app: &App) -> domain::View<'_> {
    domain::View {
        gateway_client_settings: &app.gateway_client_settings,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
        last_copied_code_hash: app.last_copied_code_hash,
        render_help: super::system_settings_browser::render_help,
    }
}
pub fn view(app: &App) -> Element<'_, Message> {
    domain::view(inputs(app)).map(super::system_settings_browser::map_message)
}
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    match domain::overlay(inputs(app)) {
        Some(overlay) => iced::widget::stack![dialog, overlay.map(super::system_settings_browser::map_message)].into(),
        None => dialog,
    }
}

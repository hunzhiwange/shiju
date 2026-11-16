//! 网络设置页的宿主消息映射与输入装配。
use crate::app::{App, Message};
use crate::app::message::settings::SettingsMessage;
use iced::Element;
use desktop_settings_network::system_settings_browser as domain;

fn inputs(app: &App) -> domain::View<'_> {
    domain::View {
        browser_settings: &app.browser_settings,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
        render_help: super::system_settings_browser::render_help,
    }
}
pub fn view(app: &App) -> Element<'_, Message> {
    domain::view(inputs(app)).map(super::system_settings_browser::map_message)
}
pub(super) fn content(app: &App) -> Element<'_, Message> {
    domain::content(inputs(app)).map(map_message)
}
pub(super) fn map_message(message: desktop_settings_network::Message) -> Message {
    use desktop_settings_network::Message as Local;
    match message {
        Local::HttpRequest(message) => Message::Settings(SettingsMessage::HttpRequest(message)),
        Local::WebSearch(message) => Message::Settings(SettingsMessage::WebSearch(message)),
        Local::Tunnel(message) => Message::Settings(SettingsMessage::Tunnel(message)),
        Local::ProxyEnabledToggled(value) => Message::Settings(SettingsMessage::ProxyEnabledToggled(value)),
        Local::ProxyScopeTextChanged(value) => Message::Settings(SettingsMessage::ProxyScopeTextChanged(value)),
        Local::ProxyHttpChanged(value) => Message::Settings(SettingsMessage::ProxyHttpChanged(value)),
        Local::ProxyHttpsChanged(value) => Message::Settings(SettingsMessage::ProxyHttpsChanged(value)),
        Local::ProxyAllChanged(value) => Message::Settings(SettingsMessage::ProxyAllChanged(value)),
        Local::ProxyNoProxyChanged(value) => Message::Settings(SettingsMessage::ProxyNoProxyChanged(value)),
        Local::ProxyServicesChanged(value) => Message::Settings(SettingsMessage::ProxyServicesChanged(value)),
        Local::ProxyHelpOpen => Message::Settings(SettingsMessage::ProxyHelpOpen),
        Local::ProxyHelpClose => Message::Settings(SettingsMessage::ProxyHelpClose),
        Local::NetworkTabSelected(tab) => Message::Settings(SettingsMessage::SystemTabSelected(match tab { desktop_settings_network::NetworkTab::WebSearch => super::system_settings::SystemTab::WebSearch,desktop_settings_network::NetworkTab::HttpRequest => super::system_settings::SystemTab::HttpRequest,desktop_settings_network::NetworkTab::Browser => super::system_settings::SystemTab::Browser,desktop_settings_network::NetworkTab::Proxy => super::system_settings::SystemTab::Proxy,desktop_settings_network::NetworkTab::Tunnel => super::system_settings::SystemTab::Tunnel, })),
        Local::None => Message::None,
        Local::Browser(message) => Message::Settings(SettingsMessage::Browser(message)),
        Local::Gateway(message) => Message::Settings(SettingsMessage::Gateway(message)),
        Local::GatewayClient(message) => Message::Settings(SettingsMessage::GatewayClient(message)),
        Local::BrowserHelp => Message::Settings(SettingsMessage::SystemHelpOpen(super::system_settings::SystemTab::Browser)),
        Local::GatewayHealthTick => Message::GatewayHealthTick,
        Local::CopyCode(text) => Message::CopyCode(text),
    }
}
pub(super) fn render_help<'a>(title: &'a str, description: std::borrow::Cow<'a, str>, color: Option<iced::Color>, i18n: desktop_i18n::I18n) -> Element<'a, desktop_settings_network::Message> {
    super::chat_panel::message_view::selectable_titled_text(title, description, color, i18n)
        .map(|message| match message {
            Message::None => desktop_settings_network::Message::None,
            Message::CopyCode(text) => desktop_settings_network::Message::CopyCode(text),
            // selectable_titled_text has no ChatOwner and only publishes these two effects.
            _ => unreachable!("plain help selection published a chat-only message"),
        })
}

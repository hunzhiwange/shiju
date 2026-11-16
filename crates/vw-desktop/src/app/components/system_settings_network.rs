//! 网络设置导航映射与领域输入装配。
use crate::app::{App, Message};
use super::system_settings::SystemTab;
use super::system_settings_search::SettingsTabDescriptor;
use desktop_settings_network::{NetworkTab, system_settings_network as domain};
#[cfg(test)]
use desktop_i18n::settings_models::connection_text::text as connection_text;
use iced::Element;
pub(super) const NETWORK_TABS: &[SettingsTabDescriptor<SystemTab>] = &[
SettingsTabDescriptor::new(SystemTab::WebSearch, domain::NETWORK_TABS[0].label, domain::NETWORK_TABS[0].keywords),
SettingsTabDescriptor::new(SystemTab::HttpRequest, domain::NETWORK_TABS[1].label, domain::NETWORK_TABS[1].keywords),
SettingsTabDescriptor::new(SystemTab::Browser, domain::NETWORK_TABS[2].label, domain::NETWORK_TABS[2].keywords),
SettingsTabDescriptor::new(SystemTab::Proxy, domain::NETWORK_TABS[3].label, domain::NETWORK_TABS[3].keywords),
SettingsTabDescriptor::new(SystemTab::Tunnel, domain::NETWORK_TABS[4].label, domain::NETWORK_TABS[4].keywords),
];
fn inputs(app: &App) -> domain::View<'_> { domain::View { system_settings_tab: match app.navigation_panels.system_settings_tab { SystemTab::HttpRequest => NetworkTab::HttpRequest,SystemTab::Browser => NetworkTab::Browser,SystemTab::Proxy => NetworkTab::Proxy,SystemTab::Tunnel => NetworkTab::Tunnel, _ => NetworkTab::WebSearch }, web_search_settings: &app.web_search_settings, http_request_settings: &app.http_request_settings, browser_settings: &app.browser_settings, proxy_settings: &app.proxy_settings, tunnel_settings: &app.tunnel_settings, desktop_locale: app.desktop_locale, app_theme: &app.app_theme, render_help: super::system_settings_browser::render_help, last_copied_code_hash: app.last_copied_code_hash } }
pub fn view(app: &App) -> Element<'_, Message> { domain::view(inputs(app)).map(super::system_settings_browser::map_message) }
#[cfg(test)]
#[path = "system_settings_network_tests.rs"]
mod system_settings_network_tests;

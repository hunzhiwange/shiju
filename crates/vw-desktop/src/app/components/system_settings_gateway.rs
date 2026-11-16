//! 网络设置页的宿主消息映射与输入装配。
use crate::app::{App, Message};
#[cfg(test)]
use crate::app::message::settings::SettingsMessage;
use iced::Element;
use desktop_settings_network::system_settings_gateway as domain;

fn inputs(app: &App) -> domain::View<'_> {
    domain::View {
        gateway_settings: &app.gateway_settings,
        desktop_locale: app.general_settings.desktop_locale,
        app_theme: &app.appearance.app_theme,
        last_copied_code_hash: app.clipboard_state.last_copied_code_hash,
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
use crate::app::state::GatewaySettingsTab;
use super::system_settings_search::SettingsTabDescriptor;
pub(super) const GATEWAY_TABS: &[SettingsTabDescriptor<GatewaySettingsTab>] = &[
    SettingsTabDescriptor::new(
        GATEWAY_TAB_LABELS[0].1,
        GATEWAY_TAB_LABELS[0].0,
        &[
            "configuration",
            "監聽地址",
            "监听地址",
            "端口",
            "auth_enabled",
            "公网绑定",
            "反向代理",
            "webhook",
            "幂等性",
            "node-control",
        ],
    ),
    SettingsTabDescriptor::new(
        GATEWAY_TAB_LABELS[1].1,
        GATEWAY_TAB_LABELS[1].0,
        &["skey management", "skey 管理", "密鑰", "skey", "密钥", "过期时间", "鉴权"],
    ),
];

use domain::GATEWAY_TAB_LABELS;
#[cfg(test)]
use crate::app::message::settings::GatewayMessage;
#[cfg(test)]
use desktop_i18n::settings_models::connection_text::text as connection_text;
#[cfg(test)]
#[path = "system_settings_gateway_tests.rs"]
mod system_settings_gateway_tests;

//! 网络设置领域效果的宿主适配。
use crate::app::{App, Message};
use iced::Task;
use super::messages::*;
use desktop_settings_network::message::proxy::{self, Effect};
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let message = match message {
SettingsMessage::ProxyEnabledToggled(value) => desktop_settings_network::Message::ProxyEnabledToggled(value),
SettingsMessage::ProxyScopeTextChanged(value) => desktop_settings_network::Message::ProxyScopeTextChanged(value),
SettingsMessage::ProxyHttpChanged(value) => desktop_settings_network::Message::ProxyHttpChanged(value),
SettingsMessage::ProxyHttpsChanged(value) => desktop_settings_network::Message::ProxyHttpsChanged(value),
SettingsMessage::ProxyAllChanged(value) => desktop_settings_network::Message::ProxyAllChanged(value),
SettingsMessage::ProxyNoProxyChanged(value) => desktop_settings_network::Message::ProxyNoProxyChanged(value),
SettingsMessage::ProxyServicesChanged(value) => desktop_settings_network::Message::ProxyServicesChanged(value),
SettingsMessage::ProxyHelpOpen => desktop_settings_network::Message::ProxyHelpOpen,
SettingsMessage::ProxyHelpClose => desktop_settings_network::Message::ProxyHelpClose,
_ => return Task::none(),
};
    match proxy::update(&mut app.proxy_settings, message) {
Effect::None => Task::none(),
Effect::Persist(value) => crate::app::update_proxy_config_async(move |config| proxy::apply_config(config, value)),
}
}

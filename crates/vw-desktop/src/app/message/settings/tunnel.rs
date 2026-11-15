//! 网络设置领域效果的宿主适配。
use crate::app::{App, Message};
use iced::Task;
use super::messages::*;
use desktop_settings_network::message::tunnel::{self, Effect};
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Tunnel(message) = message else { return Task::none(); };
    let message = desktop_settings_network::Message::Tunnel(message);
    match tunnel::update(&mut app.tunnel_settings, message) {
Effect::None => Task::none(),
Effect::Persist(value) => crate::app::update_tunnel_config_async(move |config| *config = value),
}
}

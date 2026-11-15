//! 网络设置领域效果的宿主适配。
use crate::app::{App, Message};
use iced::Task;
use super::messages::*;
use desktop_settings_network::message::http_request::{self, Effect};
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::HttpRequest(message) = message else { return Task::none(); };
    let message = desktop_settings_network::Message::HttpRequest(message);
    match http_request::update(&mut app.http_request_settings, message) {
Effect::None => Task::none(),
Effect::Persist(value) => crate::app::update_http_request_config_async(move |config| http_request::apply_config(config, value)),
}
}

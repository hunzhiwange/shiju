//! 网络设置领域效果的宿主适配。
use crate::app::{App, Message};
use iced::Task;
use super::messages::*;
use desktop_settings_network::message::web_search::{self, Effect};
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::WebSearch(message) = message else { return Task::none(); };
    let message = desktop_settings_network::Message::WebSearch(message);
    match web_search::update(&mut app.web_search_settings, message) {
Effect::None => Task::none(),
Effect::Persist(value) => crate::app::update_web_search_config_async(move |config| web_search::apply_config(config, value)),
}
}

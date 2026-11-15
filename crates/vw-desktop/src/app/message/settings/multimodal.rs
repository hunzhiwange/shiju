//! 领域更新效果的宿主持久化适配。
use crate::app::{App, Message};
use iced::Task;
use super::messages::*;
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
let SettingsMessage::Multimodal(message) = message else { return Task::none(); };
let message = desktop_settings_runtime::messages::SettingsMessage::Multimodal(message);
let Some(config) = desktop_settings_runtime::multimodal::update(&mut app.multimodal_settings, message) else { return Task::none(); };
crate::app::update_multimodal_config_async(move |current| *current = config)
}
#[cfg(test)]
#[path = "multimodal_tests.rs"]
mod multimodal_tests;

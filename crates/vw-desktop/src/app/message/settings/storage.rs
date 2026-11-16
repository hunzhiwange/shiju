use super::messages::SettingsMessage;
use crate::app::{App, Message};
use desktop_settings_memory::storage_update as domain;
use iced::Task;
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Storage(message) = message else {
        return Task::none();
    };
    let Some(config) = domain::prepare_save(&mut app.storage_settings, message) else {
        return Task::none();
    };
    crate::app::update_storage_config_async(move |storage| {
        storage.provider.config = config.provider.config
    })
}
#[cfg(test)]
#[path = "storage_tests.rs"]
mod storage_tests;

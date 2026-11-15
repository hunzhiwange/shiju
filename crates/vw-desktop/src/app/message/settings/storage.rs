use crate::app::{App, Message};
use super::messages::SettingsMessage;
#[cfg(test)]
use super::messages::StorageMessage;
use iced::Task;
use desktop_settings_memory::storage_update as domain;
fn persist_storage_settings(app: &mut App) -> Result<Task<Message>, String> {
    let mut config = config_memory::StorageConfig::default();
    domain::config(&app.storage_settings, &mut config)?;
    Ok(crate::app::update_storage_config_async(move |storage| storage.provider.config = config.provider.config))
}
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Storage(message) = message else { return Task::none(); };
    domain::update(&mut app.storage_settings, message);
    match persist_storage_settings(app) {
        Ok(task) => { app.storage_settings.save_error = None; task }
        Err(error) => { app.storage_settings.save_error = Some(error); Task::none() }
    }
}
#[cfg(test)]
#[path = "storage_tests.rs"]
mod storage_tests;

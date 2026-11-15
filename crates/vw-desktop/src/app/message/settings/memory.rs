use crate::app::{App, Message};
use super::messages::SettingsMessage;
#[cfg(test)]
use super::messages::MemoryMessage;
use iced::Task;
use desktop_settings_memory::memory_update as domain;
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Memory(message) = message else { return Task::none(); };
    let effect = domain::update(&mut app.memory_settings, message);
    if effect == domain::Effect::ClearEmbeddingRouteFeedback { app.embedding_routes_settings.save_error = None; app.embedding_routes_settings.save_success = false; }
    if effect != domain::Effect::Persist { return Task::none(); }
    let config = domain::config(&app.memory_settings);
    crate::app::config::update_memory_config_async(move |memory| *memory = config)
}
#[cfg(test)]
#[path = "memory_tests.rs"]
mod memory_tests;

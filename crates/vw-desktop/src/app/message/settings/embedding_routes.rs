use crate::app::{App, Message};
use super::messages::SettingsMessage;
use iced::Task;
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
 let SettingsMessage::EmbeddingRoutes(message) = message else { return Task::none() };
 match desktop_settings_providers::embedding_routes_update::update(&mut app.embedding_routes_settings, message) {
  Some(routes) => crate::app::config::update_embedding_routes_config_async(move |cfg| *cfg = routes),
  None => Task::none(),
 }
}

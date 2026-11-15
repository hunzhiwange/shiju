use crate::app::{App, Message};
use crate::app::message::settings::EmbeddingRoutesMessage;
use iced::Element;
use super::system_settings_models::{view_style, map_view_message};
fn map_message(message: desktop_settings_providers::view_message::Message) -> Message {
    map_view_message(message)
}
pub fn embedded_view(app: &App) -> Element<'_, Message> {
    desktop_settings_providers::embedding_routes_view::embedded_view(&app.embedding_routes_settings, view_style(app)).map(map_message)
}
#[cfg(test)]
#[path = "system_settings_embedding_routes_ui_refresh_11_tests.rs"]
mod ui_refresh_11_tests;

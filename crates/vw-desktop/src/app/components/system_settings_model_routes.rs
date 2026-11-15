use crate::app::{App, Message};
use iced::Element;
use super::system_settings_models::{map_view_message, view_style};
pub fn view(app: &App) -> Element<'_, Message> {
    desktop_settings_providers::model_routes_view::view(&app.model_routes_settings.routes, view_style(app)).map(map_view_message)
}

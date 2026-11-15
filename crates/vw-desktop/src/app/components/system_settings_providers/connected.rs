use crate::app::{App, Message};
pub fn view(app: &App) -> iced::Element<'_, Message> {
 desktop_settings_providers::connected::view(&app.provider_settings, super::super::system_settings_models::view_style(app)).map(super::super::system_settings_models::map_view_message)
}

use crate::app::{App, Message};
pub fn embedded_view(app: &App) -> iced::Element<'_, Message> {
    desktop_settings_memory::system_settings_storage::embedded_view(&app.storage_settings, app.desktop_locale, &app.app_theme)
        .map(|event| Message::Settings(crate::app::message::SettingsMessage::Storage(event)))
}

use crate::app::{App, Message};
pub(crate) fn view(app: &App) -> iced::Element<'_, Message> {
 let s = &app.skills_settings;
 desktop_settings_skills::plugins::view(desktop_settings_skills::plugins::ViewState { desktop_locale: app.general_settings.desktop_locale, app_theme: &app.appearance.app_theme, query: &s.query, plugin_catalog: &s.plugin_catalog, selected_plugin_id: &s.selected_plugin_id, plugin_loading: s.plugin_loading, plugin_status_message: &s.plugin_status_message, plugin_status_message_translation: &s.plugin_status_message_translation, plugin_status_is_error: s.plugin_status_is_error }).map(super::browser::message)
}

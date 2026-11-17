use crate::{state::*, messages::PageMessage};
use iced::Element;
pub type Message = PageMessage<crate::messages::StorageMessage>;
pub struct Fixture {
    pub memory_settings: MemorySettingsState,
    pub storage_settings: StorageSettingsState,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: iced::Theme,
}
impl Default for Fixture {
    fn default() -> Self { Self { memory_settings: Default::default(), storage_settings: Default::default(), desktop_locale: Default::default(), app_theme: iced::Theme::Dark } }
}
pub fn view(app: &Fixture) -> Element<'_, Message> {
    crate::system_settings_memory::view(&app.memory_settings, &app.storage_settings, app.desktop_locale, &app.app_theme, iced::widget::Space::new().into())
}
pub fn embedded_view(app: &Fixture) -> Element<'_, Message> {
    crate::system_settings_storage::embedded_view(&app.storage_settings, app.desktop_locale, &app.app_theme).map(PageMessage::Storage)
}
pub fn active_save_error(app: &Fixture) -> Option<&str> { crate::system_settings_memory::active_save_error(&app.memory_settings, &app.storage_settings) }
pub fn field_row_with_help<'a>(app: &Fixture, label: &'static str, description: &'static str, control: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    crate::fields::field_row_with_help(app.desktop_locale, &app.app_theme, label, description, control)
}
pub fn text_row_with_help<'a>(app: &Fixture, label: &'static str, description: &'static str, placeholder: &'static str, value: &'a str, on_input: impl Fn(String) -> Message + 'a) -> Element<'a, Message> {
    crate::fields::text_row_with_help(app.desktop_locale, &app.app_theme, label, description, placeholder, value, on_input)
}
pub fn bool_row<'a>(app: &Fixture, label: &'static str, description: &'static str, checked: bool, checkbox_label: &'static str, on_toggle: impl Fn(bool) -> Message + 'a) -> Element<'a, Message> {
    crate::fields::bool_row(app.desktop_locale, &app.app_theme, label, description, checked, checkbox_label, on_toggle)
}
pub fn number_row<'a>(label: &'static str, value: String, suffix: &'static str, min: f32, max: f32, step: f32, precision: u8, on_change: impl Fn(f32) -> Message + 'a) -> Element<'a, Message> {
    crate::fields::number_row(label, value, suffix, min, max, step, precision, Message::None, on_change)
}

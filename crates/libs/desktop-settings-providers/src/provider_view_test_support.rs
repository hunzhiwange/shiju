use crate::{provider_state::ProviderSettingsState, view_message::Message};
pub struct Fixture { pub provider_settings: ProviderSettingsState, pub desktop_locale: config_ui::DesktopLocale, pub app_theme: iced::Theme }
impl Fixture { pub fn new() -> (Self, ()) { (Self { provider_settings: ProviderSettingsState::default(), desktop_locale: config_ui::DesktopLocale::default(), app_theme: iced::Theme::Dark }, ()) } }
pub fn style(app: &Fixture) -> crate::view_style::ViewStyle<'_> { crate::view_style::ViewStyle { locale: app.desktop_locale, theme: &app.app_theme, help_text: |_, description, _, _| iced::widget::text(description).into() } }

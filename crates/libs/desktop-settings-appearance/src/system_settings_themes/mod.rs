pub mod editor;
pub mod gallery;
pub mod preview;
pub mod state;
pub mod messages;
pub struct View<'a, Message> {
 pub desktop_locale: config_ui::DesktopLocale,
 pub app_theme: &'a iced::Theme,
 pub active_custom_theme_id: &'a Option<String>,
 pub themes_settings: &'a state::ThemesSettingsState,
 pub map: fn(messages::ThemesMessage) -> Message,
 pub none: Message,
 pub help: crate::headings::HelpRenderer<Message>,
}

#[cfg(test)] mod gallery_tests;

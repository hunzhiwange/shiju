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

pub mod update;

/// Mount the editor with the same modal padding and cancel message as the gallery.
pub fn editor_overlay<'a, Message: Clone + 'static>(app: &View<'a, Message>, base: iced::Element<'a, Message>) -> iced::Element<'a, Message> {
    desktop_ui_controls::system_settings_common::settings_modal_overlay(
        Some(base), (app.map)(messages::ThemesMessage::CancelEdit),
        iced::widget::container(editor::view(app)).padding(12),
    )
}

#[cfg(test)]
mod update_tests;

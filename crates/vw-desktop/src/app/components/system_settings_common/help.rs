use crate::app::i18n::I18n;
use crate::app::{App, Message};
use desktop_ui_controls::system_settings_common::help as shared;
use iced::{
    Element,
    widget::{Row, Text},
};
pub use shared::{
    settings_close_button, settings_close_button_style, settings_close_icon_style,
    settings_help_button, settings_help_button_with_locale, settings_help_copy_state,
};
use std::borrow::Cow;
pub(crate) fn settings_hover_help_label<'a, 'theme>(
    i18n: I18n,
    theme: impl Into<Option<&'theme iced::Theme>>,
    title: &'a str,
    label: Text<'a>,
    description: impl Into<Cow<'a, str>>,
) -> Row<'a, Message> {
    shared::settings_selectable_help_label(i18n, theme, title, label, description)
}
pub fn with_settings_help_modal<'a>(
    app: &App,
    base: Element<'a, Message>,
    title: &'a str,
    help_text: impl Into<String>,
    close_message: Message,
) -> Element<'a, Message> {
    shared::localized_settings_help_modal(
        app.desktop_locale,
        app.last_copied_code_hash,
        base,
        title,
        help_text,
        Message::CopyCode,
        close_message,
    )
}
#[cfg(test)]
#[path = "help_ui_refresh_03_tests.rs"]
mod help_ui_refresh_03_tests;

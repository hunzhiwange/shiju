use crate::app::{App, Message};
use crate::app::i18n::I18n;
use crate::app::components::chat_panel::message_view::selectable_titled_text;
use iced::{Element, widget::{Row, Text}};
use std::borrow::Cow;
use desktop_ui_controls::system_settings_common::help as shared;
pub use shared::{settings_close_button, settings_close_icon_style, settings_close_button_style, settings_help_button, settings_help_button_with_locale, settings_help_copy_state};
pub(crate) fn settings_hover_help_label<'a, 'theme>(
    i18n: I18n,
    theme: impl Into<Option<&'theme iced::Theme>>,
    title: &'a str,
    label: Text<'a>,
    description: impl Into<Cow<'a, str>>,
) -> Row<'a, Message> {
    let color = theme.into().map(|theme| theme.palette().text);
    shared::settings_hover_help_label(label, selectable_titled_text(title, description, color, i18n))
}
pub fn with_settings_help_modal<'a>(
    app: &App,
    base: Element<'a, Message>,
    title: &'a str,
    help_text: impl Into<String>,
    close_message: Message,
) -> Element<'a, Message> {
    let help_text = help_text.into();
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let help_text = i18n.__(&help_text).to_string();
    let (copy_icon, copy_tip) = settings_help_copy_state(app.last_copied_code_hash, &help_text);
    let copy_tip = desktop_i18n::settings_runtime::text(app.desktop_locale, copy_tip);
    let copy_message = Message::CopyCode(help_text.clone());
    shared::with_settings_help_modal(base, i18n.__(title).to_string(), help_text, copy_icon, copy_tip, copy_message, close_message)
}
#[cfg(test)]
#[path="help_ui_refresh_03_tests.rs"]
mod help_ui_refresh_03_tests;

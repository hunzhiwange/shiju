use crate::{SecurityView, security_message::Message};
use iced::{Element, Length, widget::{text, Text, Row}};
use std::borrow::Cow;
use desktop_ui_controls::system_settings_common::*;
pub fn settings_hover_help_label<'a>(view: &SecurityView<'_>, title: &'a str, label: Text<'a>, description: impl Into<Cow<'a, str>>) -> Row<'a, Message> {
    let content = (view.help_text)(title, description.into(), Some(view.app_theme.palette().text), desktop_i18n::I18n::new(view.desktop_locale));
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(label, content)
}
pub fn settings_page_heading_with_help<'a>(view: &SecurityView<'_>, title: &'a str, description: impl Into<Cow<'a, str>>) -> Element<'a, Message> {
    settings_hover_help_label(view, title, text(title).size(28), description).wrap().into()
}
pub fn settings_group_heading_with_help<'a>(view: &SecurityView<'_>, title: &'a str, description: impl Into<Cow<'a, str>>) -> Element<'a, Message> {
    settings_hover_help_label(view, title, text(title).size(17), description).wrap().into()
}
pub fn settings_flexible_row_with_help<'a>(view: &SecurityView<'_>, label: &'a str, description: impl Into<Cow<'a, str>>, width: f32, control: impl Into<Element<'a, Message>>) -> iced::widget::Container<'a, Message> {
    let label = settings_hover_help_label(view, label, settings_field_label(label), description).width(Length::Fill).wrap();
    settings_flexible_row_with_label(label, width, control)
}
pub fn with_settings_help_modal<'a, M: Clone + 'static>(view: &SecurityView<'_>, base: Element<'a, M>, title: &'a str, help_text: impl Into<String>, close: Message, map: fn(Message) -> M) -> Element<'a, M> {
    let i18n = desktop_i18n::I18n::new(view.desktop_locale);
    let help_text = i18n.__(&help_text.into()).to_string();
    let (icon, tip) = settings_help_copy_state(view.last_copied_code_hash, &help_text);
    let tip = desktop_i18n::settings_runtime::text(view.desktop_locale, tip);
    desktop_ui_controls::system_settings_common::help::with_settings_help_modal(base, i18n.__(title).to_string(), help_text.clone(), icon, tip, map(Message::CopyCode(help_text)), map(close))
}

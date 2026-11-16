//! Channel help reuses selectable text and the shared hover/pin overlay.
use crate::messages::Message;
use desktop_i18n::I18n;
use iced::{
    Element, Theme,
    widget::{Row, Text, text},
};
use std::borrow::Cow;
pub fn settings_hover_help_label<'a, 'theme>(
    i18n: I18n,
    theme: impl Into<Option<&'theme Theme>>,
    title: &'a str,
    label: Text<'a>,
    description: impl Into<Cow<'a, str>>,
) -> Row<'a, Message> {
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(
        label,
        desktop_chat_view::message_view::markdown_selection::selectable_titled_text(
            title,
            description,
            theme.into().map(|theme| theme.palette().text),
            i18n,
        ),
    )
}
pub fn settings_group_heading_with_help<'a, 'theme>(
    i18n: I18n,
    theme: impl Into<Option<&'theme Theme>>,
    title: &'a str,
    description: impl Into<Cow<'a, str>>,
) -> Element<'a, Message> {
    settings_hover_help_label(i18n, theme, title, text(title).size(17), description).wrap().into()
}
pub fn settings_flexible_row_with_help<'a>(
    i18n: I18n,
    theme: &Theme,
    label: &'a str,
    description: impl Into<Cow<'a, str>>,
    max_control_width: f32,
    control: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    let label = settings_hover_help_label(
        i18n,
        theme,
        label,
        desktop_ui_controls::system_settings_common::settings_field_label(label),
        description,
    )
    .width(iced::Length::Fill)
    .wrap();
    desktop_ui_controls::system_settings_common::panels::settings_flexible_row_with_label(
        label,
        max_control_width,
        control,
    )
}

use super::{settings_field_label, settings_hover_help_label};
use crate::app::Message;
use crate::app::i18n::I18n;
use desktop_ui_controls::system_settings_common::panels as shared;
use iced::widget::text;
use iced::{Element, Length, Theme};
pub use shared::*;
use std::borrow::Cow;
pub(crate) fn settings_page_heading_with_help<'a>(
    i18n: I18n,
    theme: &Theme,
    title: &'a str,
    description: impl Into<Cow<'a, str>>,
) -> Element<'a, Message> {
    settings_hover_help_label(i18n, theme, title, text(title).size(28), description).wrap().into()
}
pub(crate) fn settings_group_heading_with_help<'a, 'theme>(
    i18n: I18n,
    theme: impl Into<Option<&'theme Theme>>,
    title: &'a str,
    description: impl Into<Cow<'a, str>>,
) -> Element<'a, Message> {
    settings_hover_help_label(i18n, theme, title, text(title).size(17), description).wrap().into()
}
pub(crate) fn settings_flexible_row_with_help<'a>(
    i18n: I18n,
    theme: &Theme,
    label: &'a str,
    description: impl Into<Cow<'a, str>>,
    max_control_width: f32,
    control: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    let label =
        settings_hover_help_label(i18n, theme, label, settings_field_label(label), description)
            .width(Length::Fill)
            .wrap();
    settings_flexible_row_with_label(label, max_control_width, control)
}

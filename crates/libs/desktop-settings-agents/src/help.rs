//! Reuses the host's selectable help renderer without depending on the chat domain.
use crate::message::Event;
use desktop_i18n::I18n;
use iced::{Element, Theme, widget::{Row, Text, text}};
use std::borrow::Cow;
pub type Renderer = for<'a, 'theme> fn(I18n, Option<&'theme Theme>, &'a str, Text<'a>, Cow<'a, str>) -> Row<'a, Event>;
pub fn settings_hover_help_label<'a, 'theme>(renderer: Renderer, i18n: I18n, theme: impl Into<Option<&'theme Theme>>, title: &'a str, label: Text<'a>, description: impl Into<Cow<'a, str>>) -> Row<'a, Event> {
    renderer(i18n, theme.into(), title, label, description.into())
}
pub fn settings_page_heading_with_help<'a>(renderer: Renderer, i18n: I18n, theme: &Theme, title: &'a str, description: impl Into<Cow<'a, str>>) -> Element<'a, Event> {
    settings_hover_help_label(renderer, i18n, theme, title, text(title).size(28), description).wrap().into()
}
pub fn settings_group_heading_with_help<'a, 'theme>(renderer: Renderer, i18n: I18n, theme: impl Into<Option<&'theme Theme>>, title: &'a str, description: impl Into<Cow<'a, str>>) -> Element<'a, Event> {
    settings_hover_help_label(renderer, i18n, theme, title, text(title).size(17), description).wrap().into()
}
pub fn settings_flexible_row_with_help<'a>(renderer: Renderer, i18n: I18n, theme: &Theme, label: &'a str, description: impl Into<Cow<'a, str>>, max_control_width: f32, control: impl Into<Element<'a, Event>>) -> iced::widget::Container<'a, Event> {
    let label = settings_hover_help_label(renderer, i18n, theme, label, desktop_ui_controls::system_settings_common::settings_field_label(label), description).width(iced::Length::Fill).wrap();
    desktop_ui_controls::system_settings_common::panels::settings_flexible_row_with_label(label, max_control_width, control)
}

//! 可选择帮助文本由宿主渲染；页面仅注入可选择帮助文本。

use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::{help as shared, panels};
use iced::{Element, Theme, Length, widget::{text, Text, Row}};
use std::borrow::Cow;

pub type HelpRenderer<Message> = for<'a> fn(&'a str, Cow<'a, str>, Option<iced::Color>, I18n) -> Element<'a, Message>;

pub fn settings_hover_help_label<'a, 'theme, Message: Clone + 'a>(
    render: HelpRenderer<Message>, i18n: I18n, theme: impl Into<Option<&'theme Theme>>,
    title: &'a str, label: Text<'a>, description: impl Into<Cow<'a, str>>,
) -> Row<'a, Message> {
    shared::settings_hover_help_label(label, render(title, description.into(), theme.into().map(|t| t.palette().text), i18n))
}
pub fn settings_group_heading_with_help<'a, 'theme, Message: Clone + 'a>(render: HelpRenderer<Message>, i18n: I18n, theme: impl Into<Option<&'theme Theme>>, title: &'a str, description: impl Into<Cow<'a, str>>) -> Element<'a, Message> {
    settings_hover_help_label(render, i18n, theme, title, text(title).size(17), description).wrap().into()
}
pub fn settings_flexible_row_with_help<'a, Message: Clone + 'a>(render: HelpRenderer<Message>, i18n: I18n, theme: &Theme, label: &'a str, description: impl Into<Cow<'a, str>>, width: f32, control: impl Into<Element<'a, Message>>) -> iced::widget::Container<'a, Message> {
    let label = settings_hover_help_label(render, i18n, theme, label, desktop_ui_controls::system_settings_common::settings_field_label(label), description).width(Length::Fill).wrap();
    panels::settings_flexible_row_with_label(label, width, control)
}

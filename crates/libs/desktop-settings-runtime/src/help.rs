//! 可选择帮助文本由宿主渲染；运行时页只声明复制效果。
use crate::Message;
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::{help as shared, panels};
use iced::{Element, Theme, Length, widget::{text, Text, Row}};
use std::borrow::Cow;

pub type HelpRenderer = for<'a> fn(&'a str, Cow<'a, str>, Option<iced::Color>, I18n) -> Element<'a, Message>;

pub fn settings_hover_help_label<'a, 'theme>(
    render: HelpRenderer, i18n: I18n, theme: impl Into<Option<&'theme Theme>>,
    title: &'a str, label: Text<'a>, description: impl Into<Cow<'a, str>>,
) -> Row<'a, Message> {
    shared::settings_hover_help_label(label, render(title, description.into(), theme.into().map(|t| t.palette().text), i18n))
}
pub fn settings_page_heading_with_help<'a>(render: HelpRenderer, i18n: I18n, theme: &Theme, title: &'a str, description: impl Into<Cow<'a, str>>) -> Element<'a, Message> {
    settings_hover_help_label(render, i18n, theme, title, text(title).size(28), description).wrap().into()
}
pub fn settings_group_heading_with_help<'a, 'theme>(render: HelpRenderer, i18n: I18n, theme: impl Into<Option<&'theme Theme>>, title: &'a str, description: impl Into<Cow<'a, str>>) -> Element<'a, Message> {
    settings_hover_help_label(render, i18n, theme, title, text(title).size(17), description).wrap().into()
}
pub fn settings_flexible_row_with_help<'a>(render: HelpRenderer, i18n: I18n, theme: &Theme, label: &'a str, description: impl Into<Cow<'a, str>>, width: f32, control: impl Into<Element<'a, Message>>) -> iced::widget::Container<'a, Message> {
    let label = settings_hover_help_label(render, i18n, theme, label, desktop_ui_controls::system_settings_common::settings_field_label(label), description).width(Length::Fill).wrap();
    panels::settings_flexible_row_with_label(label, width, control)
}
pub fn with_settings_help_modal<'a>(locale: config_ui::DesktopLocale, last_copied_code_hash: Option<u64>, base: Element<'a, Message>, title: &'a str, help_text: impl Into<String>, close: Message) -> Element<'a, Message> {
    let i18n = I18n::new(locale);
    let help_text = help_text.into();
    let help_text = i18n.__(&help_text).to_string();
    let (icon, tip) = shared::settings_help_copy_state(last_copied_code_hash, &help_text);
    let tip = desktop_i18n::settings_runtime::text(locale, tip);
    shared::with_settings_help_modal(base, i18n.__(title).to_string(), help_text.clone(), icon, tip, Message::CopyCode(help_text), close)
}

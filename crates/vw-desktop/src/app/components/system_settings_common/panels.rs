use crate::app::Message;
use crate::app::i18n::I18n;
use iced::{Element, Theme, Length};
use iced::widget::{text, container};
use std::borrow::Cow;
use super::{settings_hover_help_label, settings_field_label};
use desktop_ui_controls::system_settings_common::panels as shared;
pub use shared::*;
pub fn settings_panel<'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    shared::settings_panel(content)
}
pub fn settings_section_content<'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    shared::settings_section_content(content)
}
pub fn settings_page_heading<'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    shared::settings_page_heading(title, description)
}
pub fn settings_group_heading<'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    shared::settings_group_heading(title, description)
}
pub fn settings_group_card<'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    shared::settings_group_card(content)
}
pub fn settings_row<'a>(
    label: &'a str,
    description: &'a str,
    control_width: f32,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    shared::settings_row(label, description, control_width, control)
}
pub(crate) fn settings_row_with_flexible_control<'a>(
    label: &'a str,
    description: &'a str,
    max_control_width: f32,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    shared::settings_row_with_flexible_control(label, description, max_control_width, control)
}
pub(crate) fn settings_flexible_row<'a>(
    label: &'a str,
    description: &'a str,
    max_control_width: f32,
    control: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    shared::settings_flexible_row(label, description, max_control_width, control)
}
pub(crate) fn settings_responsive_row<'a, Control>(
    label: &'a str,
    description: &'a str,
    control: impl Fn() -> Control + 'a,
) -> Element<'a, Message>
where
    Control: Into<Element<'a, Message>>,
{
    shared::settings_responsive_row(label, description, control)
}
pub fn settings_modal_header<'a>(
    title: impl Into<Element<'a, Message>>,
    actions: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    shared::settings_modal_header(title, actions)
}
pub fn settings_modal_layout<'a>(
    header: impl Into<Element<'a, Message>>,
    body: impl Into<Element<'a, Message>>,
    footer: Option<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    shared::settings_modal_layout(header, body, footer)
}
pub(crate) fn settings_modal_layout_with_scroll_id<'a>(
    header: impl Into<Element<'a, Message>>,
    body: impl Into<Element<'a, Message>>,
    footer: Option<Element<'a, Message>>,
    scroll_id: Option<&'static str>,
) -> iced::widget::Container<'a, Message> {
    shared::settings_modal_layout_with_scroll_id(header, body, footer, scroll_id)
}
pub fn settings_modal_card<'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    shared::settings_modal_card(content)
}
pub fn settings_modal_overlay<'a>(
    base: Option<Element<'a, Message>>,
    close_message: Message,
    card: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    shared::settings_modal_overlay(base, close_message, card)
}
pub(crate) fn settings_optional_modal_overlay<'a>(
    base: Element<'a, Message>,
    modal: Option<(Message, Element<'a, Message>)>,
) -> Element<'a, Message> {
    shared::settings_optional_modal_overlay(base, modal)
}
pub(crate) fn settings_modal_overlay_with_escape<'a>(
    base: Element<'a, Message>,
    close_message: Message,
    card: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    shared::settings_modal_overlay_with_escape(base, close_message, card)
}
pub(crate) fn settings_modal_overlay_with_escape_and_backdrop_dismiss<'a>(
    base: Element<'a, Message>,
    close_message: Message,
    card: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    shared::settings_modal_overlay_with_escape_and_backdrop_dismiss(base, close_message, card)
}
pub(crate) fn settings_modal_overlay_with_backdrop_dismiss<'a>(
    base: Element<'a, Message>,
    close_message: Message,
    card: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    shared::settings_modal_overlay_with_backdrop_dismiss(base, close_message, card)
}
pub fn settings_section_card<'a>(
    title: &'a str,
    description: &'a str,
) -> iced::widget::Container<'a, Message> {
    shared::settings_section_card(title, description)
}
pub fn settings_page_intro<'a>(title: &'a str, description: &'a str) -> Element<'a, Message> {
    shared::settings_page_intro(title, description)
}
pub fn settings_divider() -> Element<'static, Message> {
    shared::settings_divider()
}
pub fn settings_value_badge(value: impl ToString) -> Element<'static, Message> {
    shared::settings_value_badge(value)
}
pub fn settings_control_with_hint<'a>(
    control: impl Into<Element<'a, Message>>,
    hint: &'a str,
) -> Element<'a, Message> {
    shared::settings_control_with_hint(control, hint)
}
pub fn settings_error_banner<'a>(
    message: impl iced::widget::text::IntoFragment<'a>,
) -> Element<'a, Message> {
    shared::settings_error_banner(message)
}
pub fn settings_success_banner<'a>(
    message: impl iced::widget::text::IntoFragment<'a>,
) -> Element<'a, Message> {
    shared::settings_success_banner(message)
}
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
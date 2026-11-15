//! Reuses the host's selectable help renderer without depending on the chat domain.
use super::autonomy_message::Message;
use desktop_i18n::I18n;
use iced::{Element, Theme, widget::{Row, Text, text}};
use std::borrow::Cow;
pub type Renderer = for<'a, 'theme> fn(I18n, Option<&'theme Theme>, &'a str, Text<'a>, Cow<'a, str>) -> Row<'a, Message>;
pub fn settings_hover_help_label<'a, 'theme>(renderer: Renderer, i18n: I18n, theme: impl Into<Option<&'theme Theme>>, title: &'a str, label: Text<'a>, description: impl Into<Cow<'a, str>>) -> Row<'a, Message> {
    renderer(i18n, theme.into(), title, label, description.into())
}
pub fn settings_page_heading_with_help<'a>(renderer: Renderer, i18n: I18n, theme: &Theme, title: &'a str, description: impl Into<Cow<'a, str>>) -> Element<'a, Message> {
    settings_hover_help_label(renderer, i18n, theme, title, text(title).size(28), description).wrap().into()
}
pub fn settings_group_heading_with_help<'a, 'theme>(renderer: Renderer, i18n: I18n, theme: impl Into<Option<&'theme Theme>>, title: &'a str, description: impl Into<Cow<'a, str>>) -> Element<'a, Message> {
    settings_hover_help_label(renderer, i18n, theme, title, text(title).size(17), description).wrap().into()
}
pub fn settings_flexible_row_with_help<'a>(renderer: Renderer, i18n: I18n, theme: &Theme, label: &'a str, description: impl Into<Cow<'a, str>>, max_control_width: f32, control: impl Into<Element<'a, Message>>) -> iced::widget::Container<'a, Message> {
    let label = settings_hover_help_label(renderer, i18n, theme, label, desktop_ui_controls::system_settings_common::settings_field_label(label), description).width(iced::Length::Fill).wrap();
    desktop_ui_controls::system_settings_common::panels::settings_flexible_row_with_label(label, max_control_width, control)
}

use desktop_ui_controls::system_settings_common::settings_help_copy_state;
pub fn with_settings_help_modal<'a, M: Clone + 'static>(view: &super::autonomy_inputs::View<'_>, base: Element<'a, M>, title: &'a str, help_text: impl Into<String>, close: Message, map: fn(Message) -> M) -> Element<'a, M> {
    let i18n = desktop_i18n::I18n::new(view.desktop_locale);
    let help_text = i18n.__(&help_text.into()).to_string();
    let (icon, tip) = settings_help_copy_state(view.last_copied_code_hash, &help_text);
    let tip = desktop_i18n::settings_runtime::text(view.desktop_locale, tip);
    desktop_ui_controls::system_settings_common::help::with_settings_help_modal(base, i18n.__(title).to_string(), help_text.clone(), icon, tip, map(Message::CopyCode(help_text)), map(close))
}

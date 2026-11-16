use super::Message;
use super::ViewContext;
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::help as shared;
use iced::Element;
use iced::widget::Row;
use iced::widget::Text;
use std::borrow::Cow;
pub fn settings_hover_help_label<'a, 'theme>(
    i18n: I18n,
    theme: impl Into<Option<&'theme iced::Theme>>,
    title: &'a str,
    label: Text<'a>,
    description: impl Into<Cow<'a, str>>,
) -> Row<'a, Message> {
    shared::settings_hover_help_label(
        label,
        desktop_chat_view::message_view::markdown_selection::selectable_titled_text(
            title,
            description,
            theme.into().map(|theme| theme.palette().text),
            i18n,
        ),
    )
}
pub fn with_settings_help_modal<'a>(
    app: &ViewContext<'_>,
    base: Element<'a, Message>,
    title: &str,
    help_text: impl Into<String>,
    close_message: Message,
) -> Element<'a, Message> {
    let help_text = help_text.into();
    let i18n = I18n::new(app.desktop_locale);
    let help_text = i18n.__(&help_text).to_owned();
    let (icon, tip) = shared::settings_help_copy_state(app.last_copied_code_hash, &help_text);
    let tip = desktop_i18n::settings_runtime::text(app.desktop_locale, tip);
    let copy = Message::CopyCode(help_text.clone());
    shared::with_settings_help_modal(
        base,
        i18n.__(title).to_owned(),
        help_text,
        icon,
        tip,
        copy,
        close_message,
    )
}

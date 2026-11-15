use super::*;
use iced::widget::column;
use desktop_ui_controls::system_settings_common::settings_field_label;

pub fn build_editor_field<'a, Message: Clone + 'static>(
    label: &'a str,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    build_editor_field_validated(label, control, None)
}

pub fn build_editor_field_validated<'a, Message: Clone + 'static>(
    label: &'a str,
    control: Element<'a, Message>,
    error: Option<String>,
) -> Element<'a, Message> {
    let mut content = column![settings_field_label(label), control].spacing(6);

    if let Some(error) = error {
        content = content.push(text(error).size(12).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(theme.extended_palette().danger.base.color) }
        }));
    }

    content.into()
}

pub fn build_inline_error(message: impl Into<String>) -> Element<'static, Message> {
    text(message.into())
        .size(12)
        .style(|theme: &Theme| iced::widget::text::Style {
            color: Some(theme.extended_palette().danger.base.color),
        })
        .into()
}

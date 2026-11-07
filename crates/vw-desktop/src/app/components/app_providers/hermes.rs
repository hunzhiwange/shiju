use super::super::system_settings_common::*;
use crate::app::{
    Message,
    message::app_providers::{
        edit::{EditMessage, message},
        hermes::Field,
    },
    state::app_providers::{
        edit::{EditForm, Mode},
        hermes::Draft,
    },
};
use iced::{
    Element, Length,
    widget::{checkbox, column, text, text_editor, text_input},
};

pub(super) fn fields<'a>(form: &'a EditForm, draft: &'a Draft) -> Element<'a, Message> {
    let enabled = !form.saving;
    let create = matches!(form.mode, Mode::Create);
    let event = |field| message(EditMessage::Hermes(field));
    let hint = |value| text(value).size(12).style(settings_muted_text_style);
    let mut fields = column![
        settings_divider(),
        text("Hermes 原生供应商节点").size(16),
        hint("编辑 custom_providers 中的单个 JSON 节点，保存时由执行端校验。"),
        hint("官方授权和模型发现暂不可用。"),
    ]
    .spacing(9)
    .width(Length::Fill);
    fields = fields.push(hint(
        "可编辑 base_url、api_mode、model、models 和 rate_limit_delay。API 密钥请在下方填写。",
    ));
    let mut editor = text_editor(&draft.declaration)
        .height(Length::Fixed(250.))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_editor_style_for(enabled));
    if enabled {
        editor = editor.on_action(move |action| event(Field::Declaration(action)));
    }
    fields = fields.push(editor);
    if draft.has_saved_key {
        fields = fields
            .push(
                checkbox(form.replace_key)
                    .label("替换已保存的 API 密钥")
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(enabled.then_some(|v| message(EditMessage::Replace(v)))),
            )
            .push(
                checkbox(form.clear_key)
                    .label("移除已保存的 API 密钥")
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(enabled.then_some(|v| message(EditMessage::Clear(v)))),
            )
            .push(hint(
                "更改 API 地址或协议时，请替换或移除已保存的密钥。执行端保留环境凭据引用。",
            ));
    }
    if !draft.has_saved_key || form.replace_key {
        fields = fields.push(text("API 密钥").size(13)).push(
            text_input("可选", &form.key)
                .secure(true)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_text_input_style)
                .on_input_maybe(enabled.then_some(|v| message(EditMessage::Key(v)))),
        );
    }
    fields
        .push(
            checkbox(draft.apply)
                .label(if create {
                    "保存后添加到 Hermes"
                } else {
                    "已添加时同步更新 Hermes"
                })
                .style(settings_checkbox_style)
                .on_toggle_maybe(enabled.then_some(move |value| event(Field::Apply(value)))),
        )
        .into()
}

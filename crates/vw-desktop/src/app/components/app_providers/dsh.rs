use super::super::system_settings_common::*;
use crate::app::{
    Message,
    message::app_providers::{
        dsh::Field,
        edit::{EditMessage, message},
    },
    state::app_providers::{
        dsh::Draft,
        edit::{EditForm, Mode},
    },
};
use iced::{
    Element, Length,
    widget::{checkbox, column, text, text_editor, text_input},
};

pub(super) fn fields<'a>(form: &'a EditForm, draft: &'a Draft) -> Element<'a, Message> {
    let enabled = !form.saving;
    let create = matches!(form.mode, Mode::Create);
    let event = |field| message(EditMessage::Dsh(field));
    let hint = |value| text(value).size(12).style(settings_muted_text_style);
    let mut fields =
        column![settings_field_label("供应商声明"), hint("官方授权和模型发现暂不可用。"),]
            .spacing(8);
    fields = fields.push(hint(
        "可编辑 api、options.baseURL 和 models。保留执行端值标记代表隐藏字段；API 密钥请在下方填写。",
    ));
    if create {
        fields = fields.push(hint("models 使用模型标识作为键；新增供应商需要 API 密钥。"));
    }
    let mut editor = text_editor(&draft.declaration)
        .height(Length::Fixed(250.))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_editor_style_for(enabled));
    if enabled {
        editor = editor.on_action(move |action| event(Field::Declaration(action)));
    }
    fields = fields.push(editor).push(settings_divider());
    if draft.has_saved_key {
        fields = fields
            .push(
                checkbox(form.replace_key)
                    .label("替换已保存的 API 密钥")
                    .text_size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(enabled.then_some(|v| message(EditMessage::Replace(v)))),
            )
            .push(
                checkbox(form.clear_key)
                    .label("移除已保存的 API 密钥")
                    .text_size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(enabled.then_some(|v| message(EditMessage::Clear(v)))),
            )
            .push(hint(
                "更改 API 地址或协议时，请替换或移除已保存的密钥。执行端不会把已有请求头发送到新地址或协议。",
            ));
    }
    if !draft.has_saved_key || form.replace_key {
        fields = fields.push(
            column![
                settings_field_label("API 密钥"),
                text_input("可选", &form.key)
                    .secure(true)
                    .padding(SETTINGS_CONTROL_PADDING)
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_text_input_style)
                    .on_input_maybe(enabled.then_some(|v| message(EditMessage::Key(v)))),
            ]
            .spacing(6),
        );
    }
    let fields = fields.push(settings_divider()).push(
        checkbox(draft.apply)
            .label(if create { "保存后添加到 Dsh" } else { "已添加时同步更新 Dsh" })
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_checkbox_style)
            .on_toggle_maybe(enabled.then_some(move |value| event(Field::Apply(value)))),
    );
    column![
        settings_group_heading(
            "Dsh 原生供应商节点",
            "编辑 Dsh profile 中单个供应商节点的 YAML，保存时由执行端校验。"
        ),
        settings_panel(fields)
    ]
    .spacing(12)
    .width(Length::Fill)
    .into()
}

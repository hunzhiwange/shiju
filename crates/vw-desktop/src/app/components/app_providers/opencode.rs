use super::super::system_settings_common::*;
use crate::app::{
    Message,
    message::app_providers::{
        edit::{EditMessage, message},
        opencode::Field,
    },
    state::app_providers::{
        edit::{EditForm, Mode},
        opencode::Draft,
    },
};
use iced::{
    Element, Length,
    widget::{checkbox, column, text, text_editor, text_input},
};
use vw_gateway_client::vw_api_types::app_providers::opencode::OpenCodeFormat;

pub(super) fn fields<'a>(form: &'a EditForm, draft: &'a Draft) -> Element<'a, Message> {
    let enabled = !form.saving;
    let create = matches!(form.mode, Mode::Create);
    let event = |field| message(EditMessage::OpenCode(field));
    let hint = |value| text(value).size(12).style(settings_muted_text_style);
    let v2 = draft.format == OpenCodeFormat::V2;
    let mut fields = column![
        settings_divider(),
        text("OpenCode 原生供应商节点").size(16),
        hint("仅编辑单个节点的 JSON，可用注释。保存时由执行端校验。"),
        hint("官方授权和模型发现暂不可用。"),
    ]
    .spacing(9)
    .width(Length::Fill);
    if create {
        fields = fields.push(
            checkbox(v2)
                .label("使用 V2 原生节点（providers）")
                .style(settings_checkbox_style)
                .on_toggle_maybe(enabled.then_some(move |value| event(Field::V2(value)))),
        );
    } else {
        fields = fields.push(hint(if v2 { "V2 · providers" } else { "V1 · provider" }));
        fields = fields.push(hint("保留原位的「[保留执行端值]」可沿用受保护的配置。"));
    }
    if create {
        fields = fields.push(hint(if v2 {
            "V2 节点使用 package、settings 与 models。"
        } else {
            "V1 节点使用 npm、options 与 models。"
        }));
    }
    let mut editor = text_editor(&draft.declaration)
        .height(Length::Fixed(250.))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_editor_style);
    if enabled {
        editor = editor.on_action(move |action| event(Field::Declaration(action)));
    }
    fields = fields.push(editor);
    if !v2 {
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
                .push(hint("更改 API 地址时，请替换或移除已保存的密钥。"));
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
    }
    fields
        .push(
            checkbox(draft.apply)
                .label(if create {
                    "保存后添加到 OpenCode"
                } else {
                    "已添加时同步更新 OpenCode"
                })
                .style(settings_checkbox_style)
                .on_toggle_maybe(enabled.then_some(move |value| event(Field::Apply(value)))),
        )
        .into()
}

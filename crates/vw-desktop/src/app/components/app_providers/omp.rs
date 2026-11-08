use super::super::system_settings_common::*;
use super::list::Help;
use crate::app::{
    Message,
    message::app_providers::{
        edit::{EditMessage, message},
        omp::Field,
    },
    state::app_providers::{
        edit::{EditForm, Mode},
        omp::Draft,
    },
};
use iced::{
    Element, Length,
    widget::{checkbox, column, text, text_editor, text_input},
};

pub(super) fn fields<'a>(
    form: &'a EditForm,
    draft: &'a Draft,
    help: Help<'_>,
) -> Element<'a, Message> {
    let enabled = !form.saving;
    let create = matches!(form.mode, Mode::Create);
    let event = |field| message(EditMessage::Omp(field));
    let hint = |value| text(value).size(12).style(settings_muted_text_style);
    let mut fields = column![
        help.field(
            "供应商声明",
            "可编辑 baseUrl、api、auth、models 和 discovery。API 密钥请在下方填写。"
        ),
        hint("官方授权和模型发现暂不可用。"),
    ]
    .spacing(8)
    .push(hint("执行端保留请求头和兼容选项。"));
    if create {
        fields = fields.push(hint("模型节点需要 id；没有 API 密钥时可填写 auth: none。"));
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
            .label(if create { "保存后添加到 OMP" } else { "已添加时同步更新 OMP" })
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_checkbox_style)
            .on_toggle_maybe(enabled.then_some(move |value| event(Field::Apply(value)))),
    );
    column![
        help.group(
            "OMP 原生供应商节点",
            "编辑 models.yml 中单个 providers 节点的 YAML，保存时由执行端校验。"
        ),
        settings_panel(fields)
    ]
    .spacing(12)
    .width(Length::Fill)
    .into()
}

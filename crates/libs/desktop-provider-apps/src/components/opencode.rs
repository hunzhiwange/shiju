use desktop_ui_controls::system_settings_common::*;
use super::help::Help;
use crate::{
    Message,
    message::{
        edit::{EditMessage, message},
        opencode::Field,
    },
    state::{
        edit::{EditForm, Mode},
        opencode::Draft,
    },
};
use iced::{
    Element, Length,
    widget::{checkbox, column, text, text_editor, text_input},
};
use vw_api_app_provider_extended::opencode::OpenCodeFormat;

pub fn fields<'a>(
    form: &'a EditForm,
    draft: &'a Draft,
    help: Help<'_>,
) -> Element<'a, Message> {
    let i18n = help.i18n;
    let enabled = !form.saving;
    let create = matches!(form.mode, Mode::Create);
    let event = |field| message(EditMessage::OpenCode(field));
    let hint = |value| text(value).size(12).style(settings_muted_text_style);
    let v2 = draft.format == OpenCodeFormat::V2;
    let heading = help
        .group(i18n.__("OpenCode 原生供应商节点"), i18n.__("仅编辑单个节点的 JSON，可用注释。保存时由执行端校验。"));
    let mut fields = column![].spacing(10);
    if create {
        fields = fields
            .push(help.field(
                i18n.__("节点格式"),
                if v2 {
                    i18n.__("V2 节点使用 package、settings 与 models。")
                } else {
                    i18n.__("V1 节点使用 npm、options 与 models。")
                },
            ))
            .push(
                checkbox(v2)
                    .label(i18n.__("使用 V2 原生节点（providers）"))
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(enabled.then_some(move |value| event(Field::V2(value)))),
            );
    } else {
        fields = fields.push(settings_control_with_hint(
            settings_value_badge(if v2 { "V2 · providers" } else { "V1 · provider" }),
            i18n.__("保留原位的「[保留执行端值]」可沿用受保护的配置。"),
        ));
    }
    let mut editor = text_editor(&draft.declaration)
        .height(Length::Fixed(250.))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_editor_style_for(enabled));
    if enabled {
        editor = editor.on_action(move |action| event(Field::Declaration(action)));
    }
    fields = fields.push(editor);
    if !v2 {
        if draft.has_saved_key {
            fields = fields
                .push(
                    checkbox(form.replace_key)
                        .label(i18n.__("替换已保存的 API 密钥"))
                        .style(settings_checkbox_style)
                        .on_toggle_maybe(enabled.then_some(|v| message(EditMessage::Replace(v)))),
                )
                .push(
                    checkbox(form.clear_key)
                        .label(i18n.__("移除已保存的 API 密钥"))
                        .style(settings_checkbox_style)
                        .on_toggle_maybe(enabled.then_some(|v| message(EditMessage::Clear(v)))),
                )
                .push(hint(i18n.__("更改 API 地址时，请替换或移除已保存的密钥。")));
        }
        if !draft.has_saved_key || form.replace_key {
            fields = fields.push(settings_field_label(i18n.__("API 密钥"))).push(
                text_input(i18n.__("可选"), &form.key)
                    .secure(true)
                    .padding(SETTINGS_CONTROL_PADDING)
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_text_input_style)
                    .on_input_maybe(enabled.then_some(|v| message(EditMessage::Key(v)))),
            );
        }
    }
    let fields = fields.push(settings_divider()).push(
        checkbox(draft.apply)
            .label(if create {
                i18n.__("保存后添加到 OpenCode")
            } else {
                i18n.__("已添加时同步更新 OpenCode")
            })
            .style(settings_checkbox_style)
            .on_toggle_maybe(enabled.then_some(move |value| event(Field::Apply(value)))),
    );
    column![
        column![heading, hint(i18n.__("官方授权和模型发现暂不可用。"))].spacing(6),
        settings_section_content(fields),
    ]
    .spacing(12)
    .width(Length::Fill)
    .into()
}

use desktop_ui_controls::system_settings_common::*;
use super::help::Help;
use crate::{
    Message,
    message::{
        agy::Field,
        edit::{EditMessage, message},
    },
    state::{agy::Draft, edit::EditForm},
};
use iced::{
    Element, Length,
    widget::{checkbox, column, text, text_input},
};
pub fn fields<'a>(
    form: &'a EditForm,
    draft: &'a Draft,
    help: Help<'_>,
) -> Element<'a, Message> {
    let i18n = help.i18n;
    let enabled = !form.saving;
    let input = |hint, value, event: fn(String) -> Field| {
        text_input(hint, value)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe(enabled.then_some(move |s| message(EditMessage::Agy(event(s)))))
    };
    let mut fields = column![
        text(i18n.__("全局切换、live 成员操作和模型发现暂不可用。"))
            .size(12)
            .style(settings_muted_text_style),
        column![
            settings_field_label(i18n.__("模型显示名（可选）")),
            input(i18n.__("留空使用原生默认"), &draft.model, Field::Model),
        ]
        .spacing(6),
        column![
            settings_field_label(i18n.__("API 地址（可选）")),
            input(i18n.__("留空使用默认端点"), &draft.base_url, Field::BaseUrl),
        ]
        .spacing(6),
        settings_divider(),
    ]
    .spacing(8);
    if draft.has_saved_key {
        fields = fields
            .push(
                checkbox(form.replace_key)
                    .label(i18n.__("替换已保存的 API 密钥"))
                    .text_size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(enabled.then_some(|v| message(EditMessage::Replace(v)))),
            )
            .push(
                checkbox(form.clear_key)
                    .label(i18n.__("移除密钥，使用已有原生登录"))
                    .text_size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(enabled.then_some(|v| message(EditMessage::Clear(v)))),
            );
    }
    if !draft.has_saved_key || form.replace_key {
        fields = fields.push(
            column![
                settings_field_label(i18n.__("API 密钥（可选）")),
                text_input(i18n.__("留空使用已有原生登录"), &form.key)
                    .secure(true)
                    .padding(SETTINGS_CONTROL_PADDING)
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_text_input_style)
                    .on_input_maybe(enabled.then_some(|v| message(EditMessage::Key(v)))),
            ]
            .spacing(6),
        );
    }
    let fields = fields.push(
        text(i18n.__("自定义地址需要有效密钥；改变地址须替换密钥。移除密钥时请同时清空地址。"))
            .size(12)
            .style(settings_muted_text_style),
    );
    column![
        help.group(i18n.__("Agy 终端档案"), i18n.__("保存到目录，下次授权终端启动时生效；现有会话保持不变。")),
        settings_section_content(fields),
    ]
    .spacing(12)
    .width(Length::Fill)
    .into()
}

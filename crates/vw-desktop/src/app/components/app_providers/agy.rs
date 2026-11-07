use super::super::system_settings_common::*;
use crate::app::{
    Message,
    message::app_providers::{
        agy::Field,
        edit::{EditMessage, message},
    },
    state::app_providers::{agy::Draft, edit::EditForm},
};
use iced::{
    Element, Length,
    widget::{checkbox, column, text, text_input},
};
pub(super) fn fields<'a>(form: &'a EditForm, draft: &'a Draft) -> Element<'a, Message> {
    let enabled = !form.saving;
    let input = |hint, value, event: fn(String) -> Field| {
        text_input(hint, value)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe(enabled.then_some(move |s| message(EditMessage::Agy(event(s)))))
    };
    let mut fields = column![
        text("全局切换、live 成员操作、官方授权和模型发现暂不可用。")
            .size(12)
            .style(settings_muted_text_style),
        column![
            settings_field_label("模型显示名（可选）"),
            input("留空使用原生默认", &draft.model, Field::Model),
        ]
        .spacing(6),
        column![
            settings_field_label("API 地址（可选）"),
            input("留空使用默认端点", &draft.base_url, Field::BaseUrl),
        ]
        .spacing(6),
        settings_divider(),
    ]
    .spacing(8);
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
                    .label("移除密钥，使用已有原生登录")
                    .text_size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(enabled.then_some(|v| message(EditMessage::Clear(v)))),
            );
    }
    if !draft.has_saved_key || form.replace_key {
        fields = fields.push(
            column![
                settings_field_label("API 密钥（可选）"),
                text_input("留空使用已有原生登录", &form.key)
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
        text("自定义地址需要有效密钥；改变地址须替换密钥。移除密钥时请同时清空地址。")
            .size(12)
            .style(settings_muted_text_style),
    );
    column![
        settings_group_heading(
            "Agy 终端档案",
            "保存到目录，下次授权终端启动时生效；现有会话保持不变。"
        ),
        settings_panel(fields),
    ]
    .spacing(12)
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
#[path = "agy_ui_refresh_14_tests.rs"]
mod ui_refresh_14_tests;

use super::super::system_settings_common::*;
use crate::app::{
    Message,
    message::app_providers::{
        claude::Field,
        edit::{EditMessage, message},
    },
    state::app_providers::{
        claude::{Draft, Kind},
        edit::{EditForm, Mode},
    },
};
use iced::{
    Element, Length,
    widget::{button, checkbox, column, row, text, text_input},
};

pub(super) fn fields<'a>(form: &'a EditForm, draft: &'a Draft) -> Element<'a, Message> {
    if draft.kind == Kind::Unsupported {
        return settings_panel(
            text("此档案使用其它配置类型。此表单仅可编辑基础资料，不会更改应用配置。")
                .size(13)
                .style(settings_muted_text_style),
        )
        .into();
    }
    let active = !form.saving;
    let create = matches!(form.mode, Mode::Create);
    let mode = |label, third_party| {
        button(text(label).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 14])
            .style(move |theme, status| {
                settings_segment_button_style(
                    theme,
                    status,
                    (draft.kind == Kind::ThirdParty) == third_party,
                )
            })
            .on_press_maybe(
                active.then_some(message(EditMessage::Claude(Field::ThirdParty(third_party)))),
            )
    };
    let input = |label, value, field: fn(String) -> Field| {
        text_input(label, value)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe(active.then_some(move |v| message(EditMessage::Claude(field(v)))))
    };
    let hint = |label: &'static str| text(label).size(12).style(settings_muted_text_style);
    let mut fields =
        column![row![mode("官方", false), mode("第三方", true)].spacing(8),].spacing(10);
    if draft.kind == Kind::Official {
        fields = fields.push(hint("使用 Claude Code 已有的登录。账号授权与绑定暂不可用。"));
        if draft.has_saved_key {
            fields = fields.push(hint("切换到官方配置后，原第三方密钥将从此档案移除。"));
        }
    } else {
        fields = fields
            .push(
                column![
                    settings_field_label("API 地址 · ANTHROPIC_BASE_URL"),
                    input("https://…", &draft.base_url, Field::BaseUrl),
                ]
                .spacing(6),
            )
            .push(
                checkbox(draft.api_key_header)
                    .label("使用 ANTHROPIC_API_KEY（否则使用 ANTHROPIC_AUTH_TOKEN）")
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(
                        active.then_some(|v| message(EditMessage::Claude(Field::ApiKeyHeader(v)))),
                    ),
            );
        if draft.has_saved_key {
            fields = fields.push(settings_control_with_hint(
                checkbox(form.replace_key)
                    .label("替换已保存的密钥")
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(active.then_some(|v| message(EditMessage::Replace(v)))),
                "默认保留原密钥。更改 API 地址时请提供新密钥。",
            ));
        }
        if !draft.has_saved_key || form.replace_key {
            fields = fields.push(
                column![
                    settings_field_label("新的完整密钥"),
                    text_input("输入密钥", &form.key)
                        .secure(true)
                        .padding(SETTINGS_CONTROL_PADDING)
                        .size(SETTINGS_CONTROL_TEXT_SIZE)
                        .style(settings_text_input_style)
                        .on_input_maybe(active.then_some(|v| message(EditMessage::Key(v)))),
                ]
                .spacing(6),
            );
        }
    }
    let fields = fields
        .push(
            column![
                settings_field_label("主模型 · ANTHROPIC_MODEL"),
                settings_control_with_hint(
                    input("可留空", &draft.model, Field::Model),
                    "模型发现与推荐暂不可用，可手动填写模型标识。",
                ),
            ]
            .spacing(6),
        )
        .push(settings_divider())
        .push(
            checkbox(draft.apply)
                .label(if create {
                    "仅在尚无直连供应商时激活"
                } else {
                    "如果此档案正在直连，同时应用配置"
                })
                .style(settings_checkbox_style)
                .on_toggle_maybe(
                    active.then_some(|v| message(EditMessage::Claude(Field::Apply(v)))),
                ),
        );
    column![text("Claude Code 配置").size(17), settings_panel(fields)]
        .spacing(12)
        .width(Length::Fill)
        .into()
}

#[cfg(test)]
#[path = "claude_ui_refresh_13_tests.rs"]
mod ui_refresh_13_tests;

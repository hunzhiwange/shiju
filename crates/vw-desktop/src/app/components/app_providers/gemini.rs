use super::super::system_settings_common::*;
use crate::app::{
    Message,
    message::app_providers::{
        edit::{EditMessage, message},
        gemini::Field,
    },
    state::app_providers::{
        edit::{EditForm, Mode},
        gemini::{Draft, Kind},
    },
};
use iced::{
    Element, Length,
    widget::{button, checkbox, column, row, text, text_input},
};

pub(super) fn fields<'a>(form: &'a EditForm, draft: &'a Draft) -> Element<'a, Message> {
    if draft.kind == Kind::Unsupported {
        return text("此档案使用其它配置类型。此表单仅可编辑基础资料，不会更改应用配置。")
            .size(13)
            .style(settings_muted_text_style)
            .into();
    }
    let active = !form.saving;
    let create = matches!(form.mode, Mode::Create);
    let mode = |label, third_party| {
        button(text(label).size(13))
            .padding([8, 14])
            .style(move |theme, status| {
                settings_segment_button_style(
                    theme,
                    status,
                    (draft.kind == Kind::ThirdParty) == third_party,
                )
            })
            .on_press_maybe(
                active.then_some(message(EditMessage::Gemini(Field::ThirdParty(third_party)))),
            )
    };
    let input = |label, value, field: fn(String) -> Field| {
        text_input(label, value)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe(active.then_some(move |v| message(EditMessage::Gemini(field(v)))))
    };
    let hint = |label: &'static str| text(label).size(12).style(settings_muted_text_style);
    let mut fields = column![
        settings_divider(),
        text("Gemini CLI 配置").size(16),
        row![mode("官方", false), mode("第三方", true)].spacing(8),
    ]
    .spacing(9)
    .width(Length::Fill);
    if draft.kind == Kind::Official {
        fields = fields.push(hint("使用 Gemini CLI 已有的 Google 登录。账号授权与绑定暂不可用。"));
        if draft.has_saved_key {
            fields = fields.push(hint("切换到官方配置后，原第三方密钥将从此档案移除。"));
        }
    } else {
        fields = fields.push(text("API 地址 · GOOGLE_GEMINI_BASE_URL").size(13)).push(input(
            "https://…",
            &draft.base_url,
            Field::BaseUrl,
        ));
        if draft.has_saved_key {
            fields = fields
                .push(
                    checkbox(form.replace_key)
                        .label("替换已保存的密钥")
                        .style(settings_checkbox_style)
                        .on_toggle_maybe(active.then_some(|v| message(EditMessage::Replace(v)))),
                )
                .push(hint("默认保留原密钥。更改 API 地址时请提供新密钥。"));
        }
        if !draft.has_saved_key || form.replace_key {
            fields = fields.push(text("新的完整密钥 · GEMINI_API_KEY").size(13)).push(
                text_input("输入密钥", &form.key)
                    .secure(true)
                    .padding(SETTINGS_CONTROL_PADDING)
                    .size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_text_input_style)
                    .on_input_maybe(active.then_some(|v| message(EditMessage::Key(v)))),
            );
        }
    }
    fields
        .push(text("主模型 · settings.json / model.name").size(13))
        .push(input("可留空", &draft.model, Field::Model))
        .push(hint("模型发现与推荐暂不可用，可手动填写模型标识。"))
        .push(
            checkbox(draft.apply)
                .label(if create {
                    "仅在尚无直连供应商时激活"
                } else {
                    "如果此档案正在直连，同时应用配置"
                })
                .style(settings_checkbox_style)
                .on_toggle_maybe(
                    active.then_some(|v| message(EditMessage::Gemini(Field::Apply(v)))),
                ),
        )
        .into()
}

use super::super::system_settings_common::*;
use crate::app::{
    Message,
    message::app_providers::{
        codex::Field,
        edit::{EditMessage, message},
    },
    state::app_providers::{
        codex::Draft,
        edit::{EditForm, Mode},
    },
};
use iced::{
    Element, Length,
    widget::{button, checkbox, column, row, text, text_editor, text_input},
};

pub(super) fn fields<'a>(form: &'a EditForm, draft: &'a Draft) -> Element<'a, Message> {
    let enabled = !form.saving;
    let event = |field| message(EditMessage::Codex(field));
    let hint = |value| text(value).size(12).style(settings_muted_text_style);
    let mut editor = text_editor(&draft.toml)
        .height(Length::Fixed(190.))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_editor_style);
    if enabled {
        editor = editor.on_action(move |action| event(Field::Toml(action)));
    }
    let mut fields = column![
        settings_divider(),
        text("Codex 配置 · config.toml").size(16),
        hint("编辑供应商选路与模型字段。密钥请在下方单独输入。官方账号授权暂不可用。"),
        editor,
    ]
    .spacing(9)
    .width(Length::Fill);

    if draft.has_saved_key {
        fields = fields
            .push(
                checkbox(form.replace_key)
                    .label("替换已保存的密钥")
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(enabled.then_some(|v| message(EditMessage::Replace(v)))),
            )
            .push(
                checkbox(form.clear_key)
                    .label("清除已保存的密钥")
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(enabled.then_some(|v| message(EditMessage::Clear(v)))),
            )
            .push(hint("默认保留原密钥。更改 API 地址时请替换或清除密钥。"));
    }
    if !draft.has_saved_key || form.replace_key {
        fields = fields.push(text("API 密钥 · OPENAI_API_KEY").size(13)).push(
            text_input("输入新密钥", &form.key)
                .secure(true)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_text_input_style)
                .on_input_maybe(enabled.then_some(|v| message(EditMessage::Key(v)))),
        );
    }
    fields = fields.push(text("模型映射").size(16));
    if let Some(models) = &draft.models {
        fields = fields.push(hint("填写 Codex 菜单显示名和实际请求模型。模型发现与推荐暂不可用。"));
        for (i, model) in models.iter().enumerate() {
            fields = fields.push(
                row![
                    text_input("菜单显示名", &model.display_name)
                        .padding(SETTINGS_CONTROL_PADDING)
                        .size(SETTINGS_CONTROL_TEXT_SIZE)
                        .style(settings_text_input_style)
                        .on_input_maybe(enabled.then_some(move |v| event(Field::ModelName(i, v)))),
                    text_input("实际请求模型", &model.slug)
                        .padding(SETTINGS_CONTROL_PADDING)
                        .size(SETTINGS_CONTROL_TEXT_SIZE)
                        .style(settings_text_input_style)
                        .on_input_maybe(enabled.then_some(move |v| event(Field::ModelId(i, v)))),
                    button(text("移除"))
                        .padding([7, 12])
                        .style(rounded_action_btn_style)
                        .on_press_maybe(enabled.then_some(event(Field::RemoveModel(i)))),
                ]
                .spacing(8),
            );
        }
        fields = fields.push(
            button(text("添加模型"))
                .padding([7, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe(enabled.then_some(event(Field::AddModel))),
        );
    } else {
        fields = fields.push(hint("此档案使用外部模型目录，目录内容暂不可编辑。"));
    }

    if !draft.inactive.is_empty() {
        fields = fields.push(text("不随供应商切换生效的字段").size(14));
        for path in &draft.inactive {
            let path = path.strip_prefix(&["config".into()]).unwrap_or(path).join(".");
            fields = fields.push(text(path).size(13).style(settings_muted_text_style));
        }
    }
    fields
        .push(
            checkbox(draft.apply)
                .label(if matches!(form.mode, Mode::Create) {
                    "仅在尚无直连供应商时激活"
                } else {
                    "如果此档案正在直连，同时应用配置"
                })
                .style(settings_checkbox_style)
                .on_toggle_maybe(enabled.then_some(move |v| event(Field::Apply(v)))),
        )
        .into()
}

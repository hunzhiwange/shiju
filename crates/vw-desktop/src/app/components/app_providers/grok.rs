use super::super::system_settings_common::*;
use super::list::Help;
use crate::app::{
    Message,
    message::app_providers::{
        edit::{EditMessage, message},
        grok::Field,
    },
    state::app_providers::{
        edit::{EditForm, Mode},
        grok::Draft,
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
    let event = |field| message(EditMessage::Grok(field));
    let hint = |value| text(value).size(12).style(settings_muted_text_style);
    let mut editor = text_editor(&draft.toml)
        .height(Length::Fixed(230.))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_editor_style_for(enabled));
    if enabled {
        editor = editor.on_action(move |action| event(Field::Toml(action)));
    }
    let heading = help.group(
        "Grok Build 配置 · config.toml",
        "编辑 models.default 和它指向的模型表。保存时由执行端校验。",
    );
    let mut fields = column![editor].spacing(10);
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
                    .label("移除已保存的密钥")
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(enabled.then_some(|v| message(EditMessage::Clear(v)))),
            )
            .push(hint("默认保留原密钥。更改 API 地址时请替换或移除密钥。"));
    }
    if !draft.has_saved_key || form.replace_key {
        fields = fields.push(settings_field_label("API 密钥")).push(
            text_input("如使用 env_key，可留空", &form.key)
                .secure(true)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_text_input_style)
                .on_input_maybe(enabled.then_some(|v| message(EditMessage::Key(v)))),
        );
    }
    if !draft.inactive.is_empty() {
        fields =
            fields.push(settings_divider()).push(settings_field_label("不随供应商切换生效的字段"));
        for path in &draft.inactive {
            let path = path.strip_prefix(&["config".into()]).unwrap_or(path).join(".");
            fields = fields.push(
                text(path)
                    .size(13)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                    .style(settings_muted_text_style),
            );
        }
    }
    let fields = fields.push(settings_divider()).push(
        checkbox(draft.apply)
            .label(if matches!(form.mode, Mode::Create) {
                "仅在尚无直连供应商时激活"
            } else {
                "如果此档案正在直连，同时应用配置"
            })
            .style(settings_checkbox_style)
            .on_toggle_maybe(enabled.then_some(move |v| event(Field::Apply(v)))),
    );
    column![
        column![heading, hint("官方授权和模型发现暂不可用。")].spacing(6),
        settings_panel(fields),
    ]
    .spacing(12)
    .width(Length::Fill)
    .into()
}

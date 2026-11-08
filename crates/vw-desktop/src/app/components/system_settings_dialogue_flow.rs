//! 系统设置中的对话时间线显示选项。

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_TEXT_SIZE, settings_checkbox_style, settings_divider, settings_error_banner,
    settings_flexible_row_with_help, settings_group_card, settings_page_heading_with_help,
    settings_success_banner,
};
use crate::app::{App, Message, i18n::I18n, message};
use iced::widget::{checkbox, column};
use iced::{Element, Length};

fn field_row<'a>(
    app: &App,
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_flexible_row_with_help(
        I18n::new(app.desktop_locale),
        &app.app_theme,
        label,
        description,
        112.0,
        control,
    )
    .into()
}

fn show_reasoning_summary_toggled(value: bool) -> Message {
    Message::Settings(message::SettingsMessage::DialogueFlowShowReasoningSummaryToggled(value))
}

fn expand_shell_tool_section_toggled(value: bool) -> Message {
    Message::Settings(message::SettingsMessage::DialogueFlowExpandShellToolSectionToggled(value))
}

fn expand_edit_tool_section_toggled(value: bool) -> Message {
    Message::Settings(message::SettingsMessage::DialogueFlowExpandEditToolSectionToggled(value))
}

fn is_success_save_message(message: &str) -> bool {
    message.starts_with("已保存")
}

/// 渲染对话流选项与现有的保存状态。
pub fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let reasoning_row = field_row(
        app,
        i18n.__("显示推理摘要"),
        i18n.__("在时间线中显示模型推理摘要。"),
        checkbox(app.dialogue_flow_show_reasoning_summary)
            .label(i18n.__("启用"))
            .on_toggle(show_reasoning_summary_toggled)
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_checkbox_style),
    );

    let expand_shell_row = field_row(
        app,
        i18n.__("展开 shell 工具"),
        i18n.__("默认在时间线中展开 shell 工具部分。"),
        checkbox(app.dialogue_flow_expand_shell_tool_section)
            .label(i18n.__("启用"))
            .on_toggle(expand_shell_tool_section_toggled)
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_checkbox_style),
    );

    let expand_edit_row = field_row(
        app,
        i18n.__("展开编辑工具"),
        i18n.__("默认在时间线中展开 edit、write、patch 工具部分。"),
        checkbox(app.dialogue_flow_expand_edit_tool_section)
            .label(i18n.__("启用"))
            .on_toggle(expand_edit_tool_section_toggled)
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_checkbox_style),
    );

    let mut col = column![
        settings_page_heading_with_help(
            i18n,
            &app.app_theme,
            i18n.__("对话流"),
            i18n.__("配置时间线中的推理摘要、工具展开和跟进行为。")
        ),
        settings_group_card(
            column![
                reasoning_row,
                settings_divider(),
                expand_shell_row,
                settings_divider(),
                expand_edit_row,
            ]
            .spacing(0),
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    let translated = app
        .dialogue_flow_settings_translation
        .as_ref()
        .map(|message| message.render(i18n).expect("valid dialogue status template"));
    if let Some(original) = &app.dialogue_flow_settings_save_message {
        let msg = translated.unwrap_or_else(|| original.clone());
        if is_success_save_message(original) {
            col = col.push(settings_success_banner(msg));
        } else {
            col = col.push(settings_error_banner(msg));
        }
    }

    col.into()
}
#[cfg(test)]
#[path = "system_settings_dialogue_flow_tests.rs"]
mod system_settings_dialogue_flow_tests;

#[cfg(test)]
#[path = "system_settings_dialogue_flow_ui_refresh_29_tests.rs"]
mod system_settings_dialogue_flow_ui_refresh_29_tests;

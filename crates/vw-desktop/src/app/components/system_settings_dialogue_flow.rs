//! 对话流设置视图组件
//!
//! 本模块提供系统设置中"对话流"配置页面的 UI 视图。
//! 主要用于配置对话流时间线中的显示与交互行为。
//!
//! # 功能概述
//!
//! - 配置是否显示推理摘要
//! - 配置是否默认展开 shell 工具部分
//! - 配置是否默认展开编辑工具部分
//! - 显示保存状态反馈消息

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_divider, settings_error_banner,
    settings_muted_text_style, settings_page_intro, settings_section_content,
};
use crate::app::{App, Message, message};
use iced::widget::{checkbox, column, container, row, text};
use iced::{Alignment, Element, Length};

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
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

fn success_banner_style(theme: &iced::Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        text_color: Some(palette.success.base.color),
        background: Some(iced::Background::Color(palette.success.weak.color.scale_alpha(0.18))),
        border: iced::Border {
            width: 1.0,
            color: palette.success.base.color.scale_alpha(0.45),
            radius: 14.0.into(),
        },
        ..Default::default()
    }
}

/// 渲染对话流设置页面视图
///
/// 构建并返回对话流配置页面的完整 UI 元素，包括标题、说明、
/// 行为开关、跟进行为选择和保存状态提示。
///
/// # 参数
///
/// * `app` - 应用状态引用，包含当前配置输入值和保存消息
///
/// # 返回值
///
/// 返回一个 Iced `Element`，包含完整的对话流设置界面
///
/// # UI 结构
///
/// ```text
/// ┌─────────────────────────────────────────┐
/// │ 对话流                                    │
/// │ 配置对话流权限与时间线显示行为              │
/// │                                          │
/// │ 显示推理摘要      [开关]                  │
/// │ 展开 shell 工具   [开关]                  │
/// │ 展开编辑工具      [开关]                  │
/// │ [保存状态消息]                            │
/// └─────────────────────────────────────────┘
/// ```
///
/// # 示例
///
/// ```ignore
/// use crate::app::{App, Message};
/// use iced::Element;
///
/// fn render_settings(app: &App) -> Element<'_, Message> {
///     system_settings_dialogue_flow::view(app)
/// }
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    let reasoning_row = field_row(
        "显示推理摘要",
        "在时间线中显示模型推理摘要。",
        checkbox(app.dialogue_flow_show_reasoning_summary)
            .label("启用")
            .on_toggle(show_reasoning_summary_toggled)
            .style(settings_checkbox_style),
    );

    let expand_shell_row = field_row(
        "展开 shell 工具",
        "默认在时间线中展开 shell 工具部分。",
        checkbox(app.dialogue_flow_expand_shell_tool_section)
            .label("启用")
            .on_toggle(expand_shell_tool_section_toggled)
            .style(settings_checkbox_style),
    );

    let expand_edit_row = field_row(
        "展开编辑工具",
        "默认在时间线中展开 edit、write、patch 工具部分。",
        checkbox(app.dialogue_flow_expand_edit_tool_section)
            .label("启用")
            .on_toggle(expand_edit_tool_section_toggled)
            .style(settings_checkbox_style),
    );

    let mut col = column![
        settings_page_intro("对话流", "配置时间线中的推理摘要、工具展开和跟进行为。"),
        settings_section_content(
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

    if let Some(msg) = &app.dialogue_flow_settings_save_message {
        if is_success_save_message(msg) {
            let success_banner: Element<'_, Message> = container(text(msg).size(13))
                .padding([10, 12])
                .width(Length::Fill)
                .style(success_banner_style)
                .into();
            col = col.push(success_banner);
        } else {
            col = col.push(settings_error_banner(msg));
        }
    }

    col.into()
}
#[cfg(test)]
#[path = "system_settings_dialogue_flow_tests.rs"]
mod system_settings_dialogue_flow_tests;

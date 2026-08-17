//! Bash 工具视图渲染模块
//!
//! 本模块提供 Bash 命令执行工具的 UI 视图渲染功能，用于在聊天面板中展示
//! Bash 命令的执行结果。主要功能包括：
//!
//! - 解析 Bash 工具的输入/输出数据
//! - 渲染可折叠的命令展示区块
//! - 支持鼠标悬停和展开/折叠交互
//! - 自动截断过长的命令和输出内容

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, container, mouse_area, row, text};
use iced::{Alignment, Color, Element, Length, Theme};

use crate::app::assets::Icon;
use crate::app::{App, Message, message};

use super::canonical_tool_name;
use crate::app::components::chat_panel::tool_text_support::content_safe_font;
use crate::app::components::chat_panel::utils::{
    chat_secondary_muted_text_color, chat_secondary_text_color, eye_icon_button_style,
    eye_icon_svg_style, icon_svg, simplified_block_style, single_line_preview,
};

fn bash_block_style(theme: &Theme) -> iced::widget::container::Style {
    let mut style = simplified_block_style(theme);
    style.border.width = 0.0;
    style.border.color = Color::TRANSPARENT;
    style
}

const BASH_COMMAND_PREVIEW_MAX_CHARS: usize = 72;

pub(super) fn bash_view_padding(tool_name: &str) -> iced::Padding {
    iced::Padding {
        top: 2.0,
        right: 6.0,
        bottom: 2.0,
        left: if tool_name == "bash" { 0.0 } else { 6.0 },
    }
}

pub(super) fn bash_command_preview(command: &str) -> String {
    let preview = single_line_preview(command, BASH_COMMAND_PREVIEW_MAX_CHARS);
    if preview.is_empty() { "运行命令".to_string() } else { preview }
}

/// 渲染 Bash 工具的视图组件
///
/// 该函数解析工具调用的可见数据，构建一个包含命令标题和可展开输出的 UI 组件。
/// 组件支持悬停高亮和点击展开/折叠交互。
///
/// # 参数
///
/// * `app` - 应用状态引用，包含工具展开状态和悬停索引等信息
/// * `msg_idx` - 消息索引，用于标识工具调用所属的消息
/// * `tool_idx` - 工具索引，用于标识同一消息中的具体工具调用
/// * `visible` - 工具调用的原始可见数据字符串，格式为 "tool bash\n{json}"
///
/// # 返回值
///
/// * `Some(Element)` - 如果数据格式正确且工具类型为 "bash"，返回渲染好的 UI 元素
/// * `None` - 如果数据格式不正确或工具类型不是 "bash"
///
/// # 示例
///
/// ```ignore
/// let visible = r#"tool bash
/// {"input": "ls -la", "output": "file1.txt\nfile2.txt"}"#;
/// if let Some(element) = tool_bash_view(&app, 0, 0, visible) {
///     // 使用 element 渲染 UI
/// }
/// ```
pub fn tool_bash_view<'a>(
    app: &'a App,
    msg_idx: usize,
    tool_idx: usize,
    visible: &str,
) -> Option<Element<'a, Message>> {
    // 解析第一行获取工具名称
    let (first, rest) = visible.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());

    if tool_name != "bash" {
        return None;
    }

    // 解析 JSON 格式的工具输入输出数据
    let v = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
    let input = v.get("input").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();

    // 生成工具的唯一标识键（高位存储消息索引，低位存储工具索引）
    let key = ((msg_idx as u64) << 32) | (tool_idx as u64);
    let is_hovered = app.chat_tool_hovered_idx == Some(key);

    // 解析命令行内容
    // 如果输入是 JSON 格式（以 '{' 开头），尝试提取 "command" 字段
    let cmd_line = if input.trim_start().starts_with('{') {
        serde_json::from_str::<serde_json::Value>(&input)
            .ok()
            .and_then(|vv| vv.get("command").and_then(|x| x.as_str()).map(|s| s.to_string()))
            .unwrap_or_default()
    } else {
        // 否则直接使用输入内容作为命令
        input.clone()
    };

    // 生成命令标题，空命令显示"运行命令"，否则截断显示
    let cmd_title = bash_command_preview(&cmd_line);
    let detail_btn = button(
        icon_svg(Icon::ChevronRight)
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0))
            .style(eye_icon_svg_style),
    )
    .padding([2, 4])
    .style(|theme: &Theme, status| eye_icon_button_style(theme, status))
    .on_press(Message::Chat(message::ChatMessage::OpenToolDetail(
        msg_idx,
        tool_idx,
        visible.to_string(),
    )));
    let detail_slot: Element<'a, Message> =
        if is_hovered { detail_btn.into() } else { Space::new().width(Length::Fixed(22.0)).into() };
    // 构建头部行：仅显示摘要，详细输出统一进弹窗。
    let cmd_font = content_safe_font(&cmd_title, "JetBrains Mono");
    let cmd_title_view = text(cmd_title)
        .size(14)
        .font(cmd_font)
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .style(|theme: &Theme| iced::widget::text::Style {
            color: Some(chat_secondary_muted_text_color(theme)),
        });

    let head_row = row![
        text("运行").size(14).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(chat_secondary_text_color(theme)) }
        }),
        cmd_title_view,
        detail_slot,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Length::Fill);
    let head = mouse_area(container(head_row).width(Length::Fill))
        .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
        .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave));

    // 组装最终视图：仅保留头部，运行结果不在 chat 卡片中展示
    Some(
        container(head)
            .padding(bash_view_padding(tool_name))
            .width(Length::Fill)
            .style(bash_block_style)
            .into(),
    )
}

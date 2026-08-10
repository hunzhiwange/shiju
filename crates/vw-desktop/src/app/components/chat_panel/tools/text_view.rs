//! 工具文本视图组件
//!
//! 本模块提供了用于渲染工具调用结果的文本视图组件。主要用于在聊天面板中
//! 以可交互的方式展示工具的输入、输出以及错误信息。
//!
//! # 主要功能
//!
//! - 解析工具调用的 JSON 格式输出
//! - 根据工具执行状态（成功/错误/拒绝）显示不同的视觉样式
//! - 支持展开/折叠显示详细内容
//! - 智能截断过长的文本输出
//! - 提供悬停交互效果

use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, text};
use iced::{Alignment, Background, Border, Element, Length, Theme};
use serde_json::Value;
use std::hash::{Hash, Hasher};

use crate::app::assets::Icon;
use crate::app::components::widgets::RightClickArea;
use crate::app::{App, Message, message};

use super::tool_meta::{tool_header_label, tool_header_title, tool_inline_summary};
use super::tool_parse::{
    tool_error_text, tool_input, tool_output_text, tool_status, tool_summary_text,
};
use super::{
    ToolTextTarget, canonical_tool_name, selected_chat_text_for_target, tool_permission_error_text,
    tool_permission_state, tool_permission_summary, tool_permission_title, tool_text_editor,
};
use crate::app::components::chat_panel::utils::{
    chat_context_target_key, chat_scroll_direction, chat_secondary_muted_text_color,
    copy_tooltip_content, eye_icon_button_style, eye_icon_svg_style, icon_svg, is_recent_copy,
    simplified_block_style, simplified_code_block_style, truncate_chars, truncate_lines_middle,
};

pub(super) fn copy_content_hash(text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn error_copy_button<'a>(app: &'a App, text: &str) -> Element<'a, Message> {
    let content_hash = copy_content_hash(text);
    let recently_copied = is_recent_copy(app, content_hash);
    let icon = if recently_copied { Icon::Check } else { Icon::Copy };
    let label = if recently_copied { "已复制" } else { "复制失败信息" };
    let button = button(
        icon_svg(icon)
            .width(Length::Fixed(11.0))
            .height(Length::Fixed(11.0))
            .style(eye_icon_svg_style),
    )
    .padding([3, 5])
    .style(|theme: &Theme, status| eye_icon_button_style(theme, status))
    .on_press(Message::CopyCode(text.trim().to_string()));

    Tooltip::new(button, copy_tooltip_content(label), TooltipPosition::Top).gap(6).into()
}

pub(super) fn workflow_preview_message(tool_name: &str, value: &Value) -> Option<Message> {
    if canonical_tool_name(tool_name) != "workflow_node" {
        return None;
    }
    let metadata = value.get("metadata")?.as_object()?;
    let workflow_yaml = metadata.get("workflow_yaml")?.as_str()?.trim().to_string();
    if workflow_yaml.is_empty() {
        return None;
    }
    let focus_node_id = metadata
        .get("node_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    Some(Message::WorkflowTool(crate::apps::workflow::WorkflowMessage::OpenInlineYaml {
        workflow_yaml,
        focus_node_id,
    }))
}

fn workflow_preview_button<'a>(message: Message) -> Element<'a, Message> {
    let button = button(
        icon_svg(Icon::Grid1x2)
            .width(Length::Fixed(11.0))
            .height(Length::Fixed(11.0))
            .style(eye_icon_svg_style),
    )
    .padding([2, 4])
    .style(|theme: &Theme, status| eye_icon_button_style(theme, status))
    .on_press(message);

    Tooltip::new(button, copy_tooltip_content("预览工作流"), TooltipPosition::Top).gap(6).into()
}

/// 创建工具文本视图组件
///
/// 该函数解析工具调用的文本输出，并根据工具类型、执行状态等信息
/// 生成相应的 UI 元素。支持展开/折叠、悬停高亮等交互功能。
///
/// # 参数
///
/// * `app` - 应用状态引用，包含工具展开/悬停状态等信息
/// * `msg_idx` - 消息索引，用于生成唯一标识符
/// * `tool_idx` - 工具索引，用于生成唯一标识符
/// * `visible` - 工具调用的原始文本输出，格式为 "tool <name>\n{json}"
///
/// # 返回值
///
/// * `Some(Element)` - 成功解析并生成 UI 元素时返回
/// * `None` - 以下情况返回 None：
///   - 文本格式不正确（缺少换行符或 "tool " 前缀）
///   - 工具名称为空或在排除列表中（bash、todoread、todowrite、write、edit 等）
///   - JSON 解析失败
///   - 显示内容为空
///
/// # 示例
///
/// ```ignore
/// let visible = "tool search\n{\"status\": \"success\", \"input\": \"query\", \"output\": \"results\"}";
/// if let Some(element) = tool_text_view(&app, 0, 0, visible) {
///     // 将 element 添加到 UI 中
/// }
/// ```
///
/// # 实现细节
///
/// ## 文本格式解析
///
/// 输入文本的预期格式为：
/// ```text
/// tool <tool_name>
/// {"status": "success|error|denied", "input": "...", "output": "...", "error": "..."}
/// ```
///
/// ## 排除的工具列表
///
/// 以下工具类型会被排除，不显示在此视图中：
/// - bash：Bash 命令执行
/// - todoread/todowrite：待办事项读写
/// - write/apply_patch：文件编辑操作
/// - read/file_read/pdf_read：文件读取操作
///
/// ## 视觉样式
///
/// - **成功状态**：使用次要颜色的文本，代码块使用 JetBrains Mono 字体
/// - **错误/拒绝状态**：使用危险颜色，带有红色边框和浅红背景
/// - **展开状态**：显示最多 100 行（截断中间部分，保留前后各 500 字符）
/// - **折叠状态**：显示最多 120 字符的预览（错误状态）或隐藏（成功状态）
pub fn tool_text_view<'a>(
    app: &'a App,
    msg_idx: usize,
    tool_idx: usize,
    visible: &str,
) -> Option<Element<'a, Message>> {
    let (first, rest) = visible.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());
    let v = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
    let status = tool_status(&v);
    let is_error = matches!(status, "error" | "denied");
    if tool_name.is_empty()
        || matches!(
            tool_name,
            "bash"
                | "todoread"
                | "todowrite"
                | "write"
                | "file_write"
                | "apply_patch"
                | "read"
                | "file_read"
                | "pdf_read"
        )
    {
        if !is_error {
            return None;
        }
    }

    let input = tool_input(&v).trim();
    let output_text = tool_output_text(&v).unwrap_or_default();
    let output = output_text.trim();
    let err_owned = tool_permission_error_text(tool_name, &v)
        .or_else(|| tool_error_text(&v))
        .unwrap_or_default();
    let err_text = err_owned.trim();
    let is_question_tool = tool_name == "question";

    let display_text = if is_error && !err_text.is_empty() { err_text } else { output };
    let permission_state = tool_permission_state(tool_name, &v);
    let mut summary =
        tool_permission_summary(tool_name, &v).map(ToOwned::to_owned).unwrap_or_default();
    if summary.is_empty() {
        summary = tool_summary_text(&v).unwrap_or_default();
    }
    if summary.is_empty() {
        summary = tool_inline_summary(tool_name, input).unwrap_or_default();
    }
    if summary.is_empty() && !is_error && !display_text.is_empty() {
        summary = truncate_chars(display_text, 96).to_string();
    }

    let show_body = is_error || (!is_question_tool && !display_text.is_empty());
    if !show_body && summary.is_empty() {
        return None;
    }

    let key = ((msg_idx as u64) << 32) | (tool_idx as u64);
    let expanded = show_body;
    let is_hovered = app.chat_tool_hovered_idx == Some(key);
    let context_key = chat_context_target_key(msg_idx, Some(tool_idx));

    let label = tool_header_label(tool_name);
    let title = if let Some(permission_state) = permission_state {
        tool_permission_title(&label, permission_state)
    } else if is_error {
        format!("{}失败", label)
    } else {
        label
    };
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
    let detail_button: Element<'a, Message> =
        if is_hovered { detail_btn.into() } else { Space::new().width(Length::Fixed(22.0)).into() };
    let detail_slot: Element<'a, Message> =
        if let Some(message) = workflow_preview_message(tool_name, &v) {
            row![workflow_preview_button(message), detail_button]
                .spacing(4)
                .align_y(Alignment::Center)
                .into()
        } else {
            detail_button
        };

    let head_row: Element<'a, Message> = row![
        row![
            tool_header_title(tool_name, title.clone(), is_error),
            text(summary.clone()).size(13).style(|theme: &Theme| iced::widget::text::Style {
                color: Some(chat_secondary_muted_text_color(theme))
            })
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        container(Space::new()).width(Length::Fill),
        detail_slot
    ]
    .align_y(Alignment::Center)
    .into();

    let head = mouse_area(container(head_row).width(Length::Fill))
        .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
        .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave));

    let context_text = selected_chat_text_for_target(app, context_key).unwrap_or_else(|| {
        let trimmed = display_text.trim();
        if trimmed.is_empty() { summary.clone() } else { trimmed.to_string() }
    });

    let body: Element<'a, Message> = if expanded {
        let out = truncate_lines_middle(display_text, 100, 500);
        if is_error {
            let copy_button = error_copy_button(app, display_text);
            let err_body = tool_text_editor(
                app,
                ToolTextTarget::ToolCardText { msg_idx, tool_idx, text_idx: 0 },
                "Noto Sans CJK SC",
                14.0,
                false,
                true,
            )
            .unwrap_or_else(|| {
                container(text(out).size(14).style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().danger.base.color),
                }))
                .width(Length::Fill)
                .into()
            });
            let err_content = row![container(err_body).width(Length::Fill), copy_button]
                .spacing(8)
                .align_y(Alignment::Start);
            RightClickArea::new(
                container(err_content)
                    .padding([10, 12])
                    .width(Length::Fill)
                    .style(|theme: &Theme| {
                        let ext = theme.extended_palette();
                        iced::widget::container::Style {
                            background: Some(Background::Color(
                                ext.danger.base.color.scale_alpha(0.07),
                            )),
                            border: Border {
                                width: 1.0,
                                color: ext.danger.base.color.scale_alpha(0.30),
                                radius: 14.0.into(),
                            },
                            ..Default::default()
                        }
                    })
                    .into(),
                Box::new({
                    let text = context_text.clone();
                    move |point| {
                        Message::Chat(message::ChatMessage::OpenMessageContextMenu {
                            target: context_key,
                            x: point.x,
                            y: point.y,
                            text: text.clone(),
                        })
                    }
                }),
            )
            .preserve_on_right_click()
            .into()
        } else {
            let code = tool_text_editor(
                app,
                ToolTextTarget::ToolCardText { msg_idx, tool_idx, text_idx: 0 },
                "JetBrains Mono",
                14.0,
                false,
                false,
            )
            .unwrap_or_else(|| {
                text(out)
                    .size(14)
                    .font(iced::Font::with_name("JetBrains Mono"))
                    .style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(theme.palette().text),
                    })
                    .into()
            });
            let scroll: Element<'a, Message> = scrollable(
                container(code)
                    .width(Length::Fill)
                    .padding([10, 12])
                    .style(simplified_code_block_style),
            )
            .direction(chat_scroll_direction())
            .height(Length::Fixed(180.0))
            .into();

            RightClickArea::new(
                scroll,
                Box::new({
                    let text = context_text.clone();
                    move |point| {
                        Message::Chat(message::ChatMessage::OpenMessageContextMenu {
                            target: context_key,
                            x: point.x,
                            y: point.y,
                            text: text.clone(),
                        })
                    }
                }),
            )
            .preserve_on_right_click()
            .into()
        }
    } else if is_error {
        let copy_button = error_copy_button(app, display_text);
        let err_body = tool_text_editor(
            app,
            ToolTextTarget::ToolCardText { msg_idx, tool_idx, text_idx: 0 },
            "Noto Sans CJK SC",
            14.0,
            false,
            true,
        )
        .unwrap_or_else(|| {
            container(text(truncate_chars(display_text, 120)).size(14).style(|theme: &Theme| {
                iced::widget::text::Style {
                    color: Some(theme.extended_palette().danger.base.color.scale_alpha(0.95)),
                }
            }))
            .width(Length::Fill)
            .into()
        });
        let err_content = row![container(err_body).width(Length::Fill), copy_button]
            .spacing(8)
            .align_y(Alignment::Start);
        RightClickArea::new(
            container(err_content)
                .padding([10, 12])
                .width(Length::Fill)
                .style(|theme: &Theme| {
                    let ext = theme.extended_palette();
                    iced::widget::container::Style {
                        background: Some(Background::Color(
                            ext.danger.base.color.scale_alpha(0.07),
                        )),
                        border: Border {
                            width: 1.0,
                            color: ext.danger.base.color.scale_alpha(0.30),
                            radius: 14.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into(),
            Box::new({
                let text = context_text;
                move |point| {
                    Message::Chat(message::ChatMessage::OpenMessageContextMenu {
                        target: context_key,
                        x: point.x,
                        y: point.y,
                        text: text.clone(),
                    })
                }
            }),
        )
        .preserve_on_right_click()
        .into()
    } else {
        container(text("")).height(Length::Fixed(0.0)).into()
    };

    Some(
        container(column![head, body].spacing(8))
            .width(Length::Fill)
            .style(simplified_block_style)
            .into(),
    )
}

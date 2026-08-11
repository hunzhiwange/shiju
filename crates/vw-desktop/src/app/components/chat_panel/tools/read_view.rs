//! 文件读取工具视图组件
//!
//! 本模块提供用于渲染文件读取工具调用结果的 UI 组件。支持三种工具类型：
//! - `read`：标准文件读取
//! - `file_read`：文件读取别名
//! - `pdf_read`：PDF 文件读取
//!
//! # 功能特性
//!
//! - 解析工具调用的 JSON 输入参数（文件路径、偏移量、读取限制）
//! - 渲染紧凑型和详细型两种视图样式
//! - 支持相对路径和绝对路径的解析
//! - 显示文件名、读取范围等关键信息
//! - 点击按钮可触发文件预览功能

use iced::widget::{Space, button, container, mouse_area, row, text};
use iced::{Alignment, Element, Length, Theme};
use std::path::Path;

use crate::app::components::widgets::RightClickArea;
use crate::app::{App, Message, message};

use super::{ToolTextTarget, canonical_tool_name, tool_inline_text_editor};
use crate::app::assets::Icon;
use crate::app::components::chat_panel::utils::{
    bold_font, chat_context_target_key, chat_secondary_muted_text_color,
    chat_secondary_subtle_text_color, chat_secondary_text_color, eye_icon_button_style,
    eye_icon_svg_style, icon_svg, normalize_file_reference_to_path, resolve_path,
    weak_file_button_style,
};

pub(super) fn parse_read_input(input: &str) -> Option<(String, usize, usize)> {
    if input.trim_start().starts_with('{') {
        let vv = serde_json::from_str::<serde_json::Value>(input.trim()).ok()?;
        let file_path = vv
            .get("filePath")
            .or_else(|| vv.get("file_path"))
            .or_else(|| vv.get("path"))
            .and_then(|x| x.as_str())
            .and_then(normalize_file_reference_to_path)?;
        let offset = vv.get("offset").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
        let limit = vv.get("limit").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
        Some((file_path, offset, limit))
    } else {
        normalize_file_reference_to_path(input).map(|file_path| (file_path, 0usize, 0usize))
    }
}

pub(super) fn read_range_text(offset: usize, limit: usize) -> Option<String> {
    if offset > 0 && limit > 0 {
        let start_line = offset + 1;
        let end_line = offset + limit;
        Some(format!("offset={} limit={} (line {}-{})", offset, limit, start_line, end_line))
    } else if offset > 0 {
        Some(format!("offset={} (from line {})", offset, offset + 1))
    } else if limit > 0 {
        Some(format!("limit={} (line 1-{})", limit, limit))
    } else {
        None
    }
}

pub fn tool_read_compact_view<'a>(app: &'a App, visible: &str) -> Option<Element<'a, Message>> {
    let (first, rest) = visible.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());
    if !matches!(tool_name, "read" | "file_read" | "pdf_read") {
        return None;
    }
    let v = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
    let input = v.get("input").and_then(|v| v.as_str()).unwrap_or("");

    let (file_path, offset, limit) = parse_read_input(input)?;

    if file_path.trim().is_empty() {
        return None;
    }

    let abs = resolve_path(app, &file_path).unwrap_or_else(|| file_path.clone());
    let file_name = Path::new(&file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(file_path.as_str())
        .to_string();

    let range_text = read_range_text(offset, limit);
    let range_view: Element<'a, Message> = if let Some(s) = range_text.clone() {
        text(s)
            .size(14)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(chat_secondary_subtle_text_color(theme)),
            })
            .into()
    } else {
        Space::new().into()
    };

    let verb = match tool_name {
        "pdf_read" => "读取PDF",
        _ => "读取",
    };
    let title = verb.to_string();

    Some(
        button(
            row![
                text(title).size(14).font(bold_font()).style(|theme: &Theme| {
                    iced::widget::text::Style { color: Some(chat_secondary_text_color(theme)) }
                }),
                text(file_name).size(14).style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(chat_secondary_muted_text_color(theme)),
                }),
                range_view
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .padding([2, 6])
        .style(weak_file_button_style)
        .on_press(Message::Preview(message::PreviewMessage::Open(abs)))
        .into(),
    )
}

pub fn tool_read_view<'a>(
    app: &'a App,
    msg_idx: usize,
    tool_idx: usize,
    visible: &str,
) -> Option<Element<'a, Message>> {
    let (first, rest) = visible.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());
    if !matches!(tool_name, "read" | "file_read" | "pdf_read") {
        return None;
    }
    let v = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
    let input = v.get("input").and_then(|v| v.as_str()).unwrap_or("");

    let (file_path, offset, limit) = parse_read_input(input)?;

    if file_path.trim().is_empty() {
        return None;
    }

    let abs = resolve_path(app, &file_path).unwrap_or_else(|| file_path.clone());
    let file_name = Path::new(&file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(file_path.as_str())
        .to_string();

    let range_text = read_range_text(offset, limit);
    let has_range_text = range_text.is_some();
    let range_text_value = range_text.clone().unwrap_or_default();

    let verb = match tool_name {
        "pdf_read" => "读取PDF",
        _ => "读取",
    };
    let context_key = chat_context_target_key(msg_idx, Some(tool_idx));
    let context_text = abs.clone();
    let key = ((msg_idx as u64) << 32) | (tool_idx as u64);
    let is_hovered = app.chat_tool_hovered_idx == Some(key);
    let file_name_view: Element<'a, Message> = tool_inline_text_editor(
        app,
        ToolTextTarget::ToolCardText { msg_idx, tool_idx, text_idx: 0 },
        crate::app::components::chat_panel::tool_text_support::chat_text_font_name(),
        14.0,
        chat_secondary_muted_text_color,
    )
    .unwrap_or_else(|| {
        text(file_name.clone())
            .size(14)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(chat_secondary_muted_text_color(theme)),
            })
            .into()
    });
    let range_slot: Element<'a, Message> = if has_range_text {
        tool_inline_text_editor(
            app,
            ToolTextTarget::ToolCardText { msg_idx, tool_idx, text_idx: 1 },
            crate::app::components::chat_panel::tool_text_support::chat_text_font_name(),
            14.0,
            chat_secondary_subtle_text_color,
        )
        .unwrap_or_else(|| {
            text(range_text_value.clone())
                .size(14)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(chat_secondary_subtle_text_color(theme)),
                })
                .into()
        })
    } else {
        Space::new().into()
    };
    let open_btn = button(text("打开").size(13))
        .padding([2, 6])
        .style(weak_file_button_style)
        .on_press(Message::Preview(message::PreviewMessage::Open(abs.clone())));

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

    Some(
        RightClickArea::new(
            mouse_area(
                container(
                    row![
                        row![
                            text(verb).size(14).font(bold_font()).style(|theme: &Theme| {
                                iced::widget::text::Style {
                                    color: Some(chat_secondary_text_color(theme)),
                                }
                            }),
                            file_name_view,
                            range_slot
                        ]
                        .spacing(8)
                        .align_y(iced::Alignment::Center),
                        container(Space::new()).width(Length::Fill),
                        open_btn,
                        detail_slot
                    ]
                    .align_y(Alignment::Center)
                    .spacing(4),
                )
                .width(Length::Fill),
            )
            .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
            .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave))
            .into(),
            Box::new(move |point| {
                Message::Chat(message::ChatMessage::OpenMessageContextMenu {
                    target: context_key,
                    x: point.x,
                    y: point.y,
                    text: context_text.clone(),
                })
            }),
        )
        .use_global_position()
        .preserve_on_right_click()
        .into(),
    )
}

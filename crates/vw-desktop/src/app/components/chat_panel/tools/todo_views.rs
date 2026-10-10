//! Todo 工具视图组件
//!
//! 本模块提供聊天面板中 Todo 相关工具的 UI 视图渲染功能。

use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::assets::Icon;
use crate::app::components::chat_panel::tool_text_support::chat_text_font;
use crate::app::components::chat_panel::utils::{
    chat_context_target_key, chat_scroll_direction, chat_secondary_subtle_text_color,
    chat_secondary_text_color, icon_svg, simplified_block_style, truncate_chars,
};
use crate::app::components::input_panel::todo_panel::read_todos_for_panel;
use crate::app::components::widgets::RightClickArea;
use crate::app::{App, Message, message};
use vw_shared::todo::Todo;

use super::tool_meta::{ToolVisual, tool_activity_header, tool_visual_state};
use super::tool_parse::tool_status;
use super::{ToolTextTarget, canonical_tool_name, selected_chat_text_for_target, tool_text_editor};

pub(super) fn tool_expanded(app: &App, key: u64, _is_running: bool) -> bool {
    todo_tool_expanded(app.chat_tool_expanded.contains(&key), _is_running)
}

pub(super) fn todo_tool_expanded(manual_expanded: bool, _is_running: bool) -> bool {
    manual_expanded
}

pub(super) fn todo_tool_card_padding() -> iced::Padding {
    iced::Padding::ZERO
}

fn todo_tool_header<'a>(
    app: &'a App,
    msg_idx: usize,
    tool_idx: usize,
    tool_name: &str,
    detail_raw: String,
    label: String,
    meta: Element<'a, Message>,
    visual: ToolVisual,
    expanded: bool,
) -> Element<'a, Message> {
    let key = ((msg_idx as u64) << 32) | (tool_idx as u64);
    let is_hovered = app.chat_tool_hovered_idx == Some(key);
    let detail_btn = button(
        icon_svg(Icon::ChevronRight).width(Length::Fixed(10.0)).height(Length::Fixed(10.0)).style(
            |theme: &Theme, _status| iced::widget::svg::Style {
                color: Some(chat_secondary_subtle_text_color(theme)),
            },
        ),
    )
    .padding([0, 1])
    .style(todo_header_button_style)
    .on_press(Message::Chat(message::ChatMessage::OpenToolDetail(msg_idx, tool_idx, detail_raw)));
    let detail_slot: Element<'a, Message> =
        if is_hovered { detail_btn.into() } else { Space::new().width(Length::Fixed(14.0)).into() };
    let toggle_btn = button(
        icon_svg(if expanded { Icon::ChevronDown } else { Icon::ChevronRight })
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0))
            .style(|theme: &Theme, _status| iced::widget::svg::Style {
                color: Some(chat_secondary_subtle_text_color(theme)),
            }),
    )
    .padding([0, 1])
    .style(todo_header_button_style)
    .on_press(Message::Chat(message::ChatMessage::ToggleTool(msg_idx, tool_idx)));
    let actions = row![meta, toggle_btn, detail_slot].spacing(7).align_y(Alignment::Center);
    mouse_area(
        container(tool_activity_header(tool_name, label, "", visual, actions.into()))
            .width(Length::Fill),
    )
    .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
    .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave))
    .into()
}

fn todo_header_button_style(
    theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        border: iced::Border { width: 0.0, color: iced::Color::TRANSPARENT, radius: 0.0.into() },
        text_color: chat_secondary_subtle_text_color(theme),
        shadow: iced::Shadow::default(),
        ..Default::default()
    }
}

fn todo_meta_pill<'a>(label: String) -> Element<'a, Message> {
    container(text(label).size(12).font(chat_text_font()))
        .padding([1, 8])
        .style(|theme: &Theme| {
            let ext = theme.extended_palette();
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            let bg = if is_dark {
                ext.background.strong.color.scale_alpha(0.26)
            } else {
                ext.background.weak.color.scale_alpha(0.7)
            };
            iced::widget::container::Style {
                background: Some(Background::Color(bg)),
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
                text_color: Some(chat_secondary_subtle_text_color(theme)),
                ..Default::default()
            }
        })
        .into()
}

fn todo_summary_pill<'a>(summary_text: String) -> Element<'a, Message> {
    container(text(summary_text).size(12).font(chat_text_font()))
        .padding([1, 8])
        .style(|theme: &Theme| {
            let ext = theme.extended_palette();
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            let bg = if is_dark {
                ext.background.strong.color.scale_alpha(0.26)
            } else {
                ext.background.weak.color.scale_alpha(0.7)
            };
            iced::widget::container::Style {
                background: Some(Background::Color(bg)),
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
                text_color: Some(chat_secondary_subtle_text_color(theme)),
                ..Default::default()
            }
        })
        .into()
}

pub fn tool_todowrite_compact_view<'a>(
    app: &'a App,
    msg_idx: usize,
    tool_idx: usize,
    visible: &str,
) -> Option<Element<'a, Message>> {
    let (first, rest) = visible.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());
    if tool_name != "todowrite" {
        return None;
    }

    let v = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
    let tool_status = v.get("status").and_then(|s| s.as_str()).unwrap_or("");
    let visual = tool_visual_state(tool_status);
    let is_error = matches!(tool_status, "error" | "failed" | "denied");
    let is_completed = matches!(tool_status, "completed" | "success");
    let is_running = matches!(tool_status, "running" | "waiting");

    let merge = v
        .get("input")
        .and_then(|i| i.as_str())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s.trim()).ok())
        .and_then(|vv| vv.get("merge").and_then(|m| m.as_bool()))
        .unwrap_or(false);

    let todos = v
        .get("input")
        .and_then(|i| i.as_str())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s.trim()).ok())
        .and_then(|vv| vv.get("todos").cloned())
        .and_then(|vv| vv.as_array().cloned())
        .unwrap_or_default();

    let todos_list: Vec<(String, String)> = todos
        .iter()
        .map(|t| {
            let status = t.get("status").and_then(|s| s.as_str()).unwrap_or("").trim().to_string();
            let content = t
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("（无内容）")
                .trim()
                .to_string();
            (status, content)
        })
        .collect();

    let label = if is_error {
        "更新任务失败".to_string()
    } else if is_completed {
        if merge { "任务已更新".to_string() } else { "任务已写入".to_string() }
    } else if tool_status == "cancelled" {
        "任务已取消".to_string()
    } else if tool_status == "waiting" {
        "等待更新任务".to_string()
    } else if is_running {
        "写任务中".to_string()
    } else {
        "更新任务".to_string()
    };

    let key = ((msg_idx as u64) << 32) | (tool_idx as u64);
    let expanded = tool_expanded(app, key, is_running);
    let context_key = chat_context_target_key(msg_idx, Some(tool_idx));
    let selected_context_text = selected_chat_text_for_target(app, context_key);

    let head_with_hover = todo_tool_header(
        app,
        msg_idx,
        tool_idx,
        tool_name,
        visible.to_string(),
        label,
        todo_meta_pill(if todos_list.is_empty() {
            "-".to_string()
        } else {
            format!("{} 项", todos_list.len())
        }),
        visual,
        expanded,
    );

    if !expanded {
        return Some(
            container(head_with_hover)
                .padding(todo_tool_card_padding())
                .width(Length::Fill)
                .style(simplified_block_style)
                .into(),
        );
    }

    let mut content = column![head_with_hover].spacing(8);

    if is_error {
        let err_full = v
            .get("error")
            .and_then(|e| e.as_str())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if !err_full.trim().is_empty() {
            let err_view = tool_text_editor(
                app,
                ToolTextTarget::ToolCardText { msg_idx, tool_idx, text_idx: 0 },
                "Noto Sans CJK SC",
                14.0,
                false,
                true,
            )
            .unwrap_or_else(|| {
                let short = truncate_chars(err_full.trim(), 160);
                container(text(short).size(14).style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().danger.base.color.scale_alpha(0.9)),
                }))
                .width(Length::Fill)
                .into()
            });
            content = content.push(err_view);
        }
    }

    if !todos_list.is_empty() {
        let todo_count = todos_list.len();
        let mut list = column![].spacing(6);
        let mut context_lines = Vec::new();

        for (status, content_text) in todos_list {
            let symbol = match status.as_str() {
                "completed" => "✓",
                "in_progress" => "·",
                _ => "○",
            };
            context_lines.push(format!("{} {}", symbol, content_text));
            list = list.push(
                row![
                    text(symbol).size(14).style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(chat_secondary_text_color(theme)),
                    }),
                    text(content_text).size(14).style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(chat_secondary_text_color(theme)),
                    })
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }

        let body_content: Element<'a, Message> = container(list).width(Length::Fill).into();

        const MAX_TODO_HEIGHT: f32 = 180.0;
        const TODO_SCROLL_THRESHOLD: usize = 6;
        const TODO_SCROLLBAR_GUTTER: f32 = 12.0;

        let body_content = container(body_content).width(Length::Fill).padding(iced::Padding {
            top: 6.0,
            right: TODO_SCROLLBAR_GUTTER,
            bottom: 8.0,
            left: 8.0,
        });

        let body: Element<'a, Message> = if todo_count > TODO_SCROLL_THRESHOLD {
            scrollable(body_content)
                .direction(chat_scroll_direction())
                .height(Length::Fixed(MAX_TODO_HEIGHT))
                .into()
        } else {
            scrollable(body_content)
                .direction(chat_scroll_direction())
                .height(Length::Shrink)
                .into()
        };
        let body: Element<'a, Message> = RightClickArea::new(
            body,
            Box::new({
                let text = selected_context_text.unwrap_or_else(|| context_lines.join("\n"));
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
        .use_global_position()
        .preserve_on_right_click()
        .into();
        content = content.push(container(body).width(Length::Fill));
    }

    Some(
        container(content)
            .padding(todo_tool_card_padding())
            .width(Length::Fill)
            .style(simplified_block_style)
            .into(),
    )
}

pub fn tool_todos_view<'a>(
    app: &'a App,
    msg_idx: usize,
    tool_idx: usize,
    visible: &str,
) -> Option<Element<'a, Message>> {
    let (first, rest) = visible.split_once('\n')?;
    let tool_name = canonical_tool_name(first.trim().strip_prefix("tool ")?.trim());
    if tool_name != "todoread" {
        return None;
    }

    let v = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
    let visual = tool_visual_state(tool_status(&v));
    let output = v.get("output")?.as_str()?;
    let parsed: Option<Vec<Todo>> = serde_json::from_str::<Vec<Todo>>(output.trim()).ok();
    let (mut todos, load_error) = match parsed {
        Some(todos) => (todos, None),
        None => read_todos_for_panel(app)?,
    };

    let parse_id = |id: &str| id.parse::<u64>().ok();
    todos.sort_by(|a, b| match (parse_id(&a.id), parse_id(&b.id)) {
        (Some(ai), Some(bi)) => ai.cmp(&bi),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.id.cmp(&b.id),
    });

    let total = todos.len();
    let done = todos.iter().filter(|t| t.status == "completed").count();
    let is_running = todos.iter().any(|t| t.status == "in_progress");
    let key = ((msg_idx as u64) << 32) | (tool_idx as u64);
    let expanded = tool_expanded(app, key, is_running);

    let header = todo_tool_header(
        app,
        msg_idx,
        tool_idx,
        tool_name,
        visible.to_string(),
        "读取当前任务".to_string(),
        todo_summary_pill(if total == 0 {
            "任务未开始".to_string()
        } else {
            format!("{}/{} 任务完成", done, total)
        }),
        visual,
        expanded,
    );

    if !expanded {
        return Some(
            container(header)
                .padding(todo_tool_card_padding())
                .width(Length::Fill)
                .style(simplified_block_style)
                .into(),
        );
    }

    let mut items = column![].spacing(8);

    if let Some(msg) = load_error {
        let err_view = text(msg).size(14).style(|theme: &Theme| iced::widget::text::Style {
            color: Some(theme.extended_palette().danger.base.color),
        });
        items = items.push(err_view);
    } else if todos.is_empty() {
    } else {
        let mut list = column![].spacing(6);

        for todo in &todos {
            let symbol = match todo.status.as_str() {
                "completed" => "✓",
                "in_progress" => "·",
                _ => "○",
            };
            list = list.push(
                row![
                    text(symbol).size(14).style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(chat_secondary_text_color(theme)),
                    }),
                    text(todo.content.clone()).size(14).style(|theme: &Theme| {
                        iced::widget::text::Style { color: Some(chat_secondary_text_color(theme)) }
                    })
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }

        let list_view: Element<'a, Message> = container(list).width(Length::Fill).into();
        items = items.push(list_view);
    }

    const MAX_TODO_HEIGHT: f32 = 180.0;
    const TODO_SCROLL_THRESHOLD: usize = 6;
    const TODO_SCROLLBAR_GUTTER: f32 = 12.0;

    let list_content = container(items).width(Length::Fill).padding(iced::Padding {
        top: 6.0,
        right: TODO_SCROLLBAR_GUTTER,
        bottom: 8.0,
        left: 8.0,
    });

    let list = if total > TODO_SCROLL_THRESHOLD {
        scrollable(list_content)
            .direction(chat_scroll_direction())
            .height(Length::Fixed(MAX_TODO_HEIGHT))
    } else {
        scrollable(list_content).direction(chat_scroll_direction()).height(Length::Shrink)
    };

    Some(
        container(column![header, list].spacing(8))
            .padding(todo_tool_card_padding())
            .width(Length::Fill)
            .style(simplified_block_style)
            .into(),
    )
}

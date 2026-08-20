//! 消息文本主体相关工具。
//!
//! 该模块负责纯文本消息、只读编辑器和思考文本内容的呈现，
//! 并集中维护分段显示与粗略高度估算逻辑。

use iced::widget::{column, container, row, text, text_editor};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::components::text_editor_scroll_panel::pass_wheel_to_parent;
use crate::app::{App, Message, message};

use super::super::selectable_text::{SelectableTextPolicy, selectable_text_policy};
use super::super::tool_text_support::{
    READ_ONLY_EDITOR_MAX_HEIGHT, chat_text_font, chat_text_line_height, read_only_text_style,
};
use super::super::utils::normalize_display_text;
use super::styles::{
    MESSAGE_TEXT_SIZE, message_body_text_color, message_text_line_height, think_block_text_color,
};

const SEGMENT_MAX_LINES: usize = 96;
const SEGMENT_MAX_CHARS: usize = 4_000;
const SESSION_CONTROL_OPEN: &str = "<session_control_selection>";
const SESSION_CONTROL_CLOSE: &str = "</session_control_selection>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionControlSelection {
    pub(super) tools: Vec<String>,
    pub(super) skills: Vec<String>,
    pub(super) shiju_skills: Vec<String>,
}

pub(super) fn estimate_text_height(text: &str) -> f32 {
    const CHARS_PER_LINE: usize = 72;
    const LINE_HEIGHT: f32 = 24.0;

    let explicit_lines = text.lines().count().max(1);
    let wrapped_lines = text.len().div_ceil(CHARS_PER_LINE).max(1);
    explicit_lines.max(wrapped_lines) as f32 * LINE_HEIGHT
}

pub(super) fn split_text_segments(text: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut current_lines = 0usize;

    for line in text.lines() {
        let line_len = line.len() + usize::from(!current.is_empty());
        let next_lines = current_lines.saturating_add(1);
        let next_len = current.len().saturating_add(line_len);
        if !current.is_empty() && (next_lines > SEGMENT_MAX_LINES || next_len > SEGMENT_MAX_CHARS) {
            segments.push(current);
            current = String::new();
            current_lines = 0;
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
        current_lines += 1;
    }

    if current.is_empty() {
        if !text.is_empty() {
            segments.push(text.to_string());
        }
    } else {
        segments.push(current);
    }

    segments
}

pub(super) fn should_segment_text_block(text: &str) -> bool {
    text.len() > SEGMENT_MAX_CHARS || text.lines().count() > SEGMENT_MAX_LINES
}

fn parse_session_control_items(line: &str, prefix: &str) -> Option<Vec<String>> {
    let rest = line.strip_prefix(prefix)?;
    Some(
        rest.split([',', '，'])
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
    )
}

pub(super) fn split_session_control_selection(
    raw: &str,
) -> (String, Option<SessionControlSelection>) {
    let Some(open_start) = raw.find(SESSION_CONTROL_OPEN) else {
        return (raw.to_string(), None);
    };
    let content_start = open_start + SESSION_CONTROL_OPEN.len();
    let Some(close_offset) = raw[content_start..].find(SESSION_CONTROL_CLOSE) else {
        return (raw.to_string(), None);
    };
    let close_start = content_start + close_offset;
    let close_end = close_start + SESSION_CONTROL_CLOSE.len();

    let block = &raw[content_start..close_start];
    let mut selection =
        SessionControlSelection { tools: Vec::new(), skills: Vec::new(), shiju_skills: Vec::new() };
    for line in block.lines().map(str::trim) {
        if let Some(items) = parse_session_control_items(line, "工具：")
            .or_else(|| parse_session_control_items(line, "工具:"))
        {
            selection.tools = items;
        } else if let Some(items) = parse_session_control_items(line, "技能：")
            .or_else(|| parse_session_control_items(line, "技能:"))
        {
            selection.skills = items;
        } else if let Some(items) = parse_session_control_items(line, "识矩功能：")
            .or_else(|| parse_session_control_items(line, "识矩功能:"))
        {
            selection.shiju_skills = items;
        }
    }

    let mut cleaned = String::with_capacity(raw.len().saturating_sub(close_end - open_start));
    cleaned.push_str(&raw[..open_start]);
    cleaned.push_str(&raw[close_end..]);
    let cleaned = cleaned.trim().to_string();

    if selection.tools.is_empty()
        && selection.skills.is_empty()
        && selection.shiju_skills.is_empty()
    {
        (cleaned, None)
    } else {
        (cleaned, Some(selection))
    }
}

fn session_control_pill(label: String) -> Element<'static, Message> {
    container(text(label).size(11).font(chat_text_font()))
        .padding([3, 8])
        .style(|theme: &Theme| {
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            iced::widget::container::Style {
                text_color: Some(theme.palette().text.scale_alpha(0.86)),
                background: Some(Background::Color(if is_dark {
                    Color::from_rgba8(255, 255, 255, 0.08)
                } else {
                    Color::from_rgba8(15, 23, 42, 0.06)
                })),
                border: Border {
                    width: 1.0,
                    color: if is_dark {
                        Color::from_rgba8(255, 255, 255, 0.10)
                    } else {
                        Color::from_rgba8(15, 23, 42, 0.10)
                    },
                    radius: 999.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

fn session_control_row(title: &'static str, items: &[String]) -> Option<Element<'static, Message>> {
    if items.is_empty() {
        return None;
    }

    let mut chips = row![text(title).size(11).font(chat_text_font()).style(|theme: &Theme| {
        iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.62)) }
    })]
    .spacing(6)
    .align_y(Alignment::Center);

    for item in items {
        chips = chips.push(session_control_pill(item.clone()));
    }

    Some(chips.into())
}

pub(super) fn session_control_selection_card<'a>(
    selection: SessionControlSelection,
) -> Element<'a, Message> {
    let mut rows =
        column![text("会话上下文").size(12).font(chat_text_font()).style(|theme: &Theme| {
            iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.78)) }
        })]
        .spacing(8);

    if let Some(row) = session_control_row("工具", &selection.tools) {
        rows = rows.push(row);
    }
    if let Some(row) = session_control_row("技能", &selection.skills) {
        rows = rows.push(row);
    }
    if let Some(row) = session_control_row("识矩功能", &selection.shiju_skills) {
        rows = rows.push(row);
    }

    container(rows)
        .padding([10, 12])
        .width(Length::Fill)
        .style(|theme: &Theme| {
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            iced::widget::container::Style {
                background: Some(Background::Color(if is_dark {
                    Color::from_rgba8(18, 20, 24, 0.72)
                } else {
                    Color::from_rgba8(248, 250, 252, 0.94)
                })),
                border: Border {
                    width: 1.0,
                    color: if is_dark {
                        Color::from_rgba8(255, 255, 255, 0.10)
                    } else {
                        Color::from_rgba8(15, 23, 42, 0.10)
                    },
                    radius: 12.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

pub(super) fn message_text_body<'a>(content: String, is_user: bool) -> Element<'a, Message> {
    if should_segment_text_block(&content) {
        let mut blocks = column![].spacing(10);
        for segment in split_text_segments(&content) {
            blocks = blocks.push(
                container(
                    text(segment)
                        .size(MESSAGE_TEXT_SIZE)
                        .font(chat_text_font())
                        .line_height(message_text_line_height())
                        .style(move |theme: &Theme| iced::widget::text::Style {
                            color: Some(message_body_text_color(theme, is_user)),
                        }),
                )
                .width(Length::Fill),
            );
        }
        blocks.into()
    } else {
        container(
            text(content)
                .size(MESSAGE_TEXT_SIZE)
                .font(chat_text_font())
                .line_height(message_text_line_height())
                .style(move |theme: &Theme| iced::widget::text::Style {
                    color: Some(message_body_text_color(theme, is_user)),
                }),
        )
        .width(Length::Fill)
        .into()
    }
}

pub(super) fn message_editor_body<'a>(
    app: &'a App,
    idx: usize,
    value: Color,
) -> Option<Element<'a, Message>> {
    message_editor_body_with_width(app, idx, value, None)
}

pub(super) fn user_message_editor_body<'a>(
    app: &'a App,
    idx: usize,
    value: Color,
) -> Option<Element<'a, Message>> {
    let editor_content = app.chat_message_editors.get(idx)?;
    let width = estimated_user_message_editor_width(&editor_content.text());
    message_editor_body_with_width(app, idx, value, Some(width))
}

fn message_editor_body_with_width<'a>(
    app: &'a App,
    idx: usize,
    value: Color,
    fixed_width: Option<f32>,
) -> Option<Element<'a, Message>> {
    let editor_content = app.chat_message_editors.get(idx)?;
    let on_action =
        move |action| Message::Chat(message::ChatMessage::MessageEditorAction(idx, action));
    let editor = text_editor(editor_content)
        .on_action(on_action)
        .size(MESSAGE_TEXT_SIZE)
        .line_height(message_text_line_height())
        .padding(0)
        .height(Length::Shrink)
        .max_height(READ_ONLY_EDITOR_MAX_HEIGHT)
        .font(chat_text_font())
        .style(move |theme: &Theme, _status: text_editor::Status| {
            read_only_text_style(theme, value)
        });
    let editor = if let Some(width) = fixed_width { editor.width(width) } else { editor };
    let container_width =
        if fixed_width.is_some() { Length::Shrink } else { Length::Fill };

    Some(container(pass_wheel_to_parent(editor)).width(container_width).into())
}

pub(super) fn estimated_user_message_editor_width(text: &str) -> f32 {
    const MIN_WIDTH: f32 = 32.0;
    const MAX_WIDTH: f32 = 612.0;

    text.lines()
        .map(|line| {
            line.chars()
                .map(|character| match character {
                    '\t' => 32.0,
                    character if character.is_ascii() => 8.25,
                    _ => MESSAGE_TEXT_SIZE,
                })
                .sum::<f32>()
        })
        .fold(MIN_WIDTH, f32::max)
        .clamp(MIN_WIDTH, MAX_WIDTH)
}

pub(super) fn think_text_body<'a>(
    app: &'a App,
    msg_idx: usize,
    think_idx: usize,
    content: String,
    prefer_plain_text: bool,
) -> Element<'a, Message> {
    let scroll_key = ((msg_idx as u64) << 32) | (think_idx as u64);
    let policy = selectable_text_policy(&content, should_segment_text_block(&content));
    if !prefer_plain_text
        && policy != SelectableTextPolicy::PreviewWithCopyFallback
        && let Some(editor_content) = app.chat_think_editors.get(&scroll_key)
    {
        let on_action = move |action| {
            Message::Chat(message::ChatMessage::ThinkEditorAction(msg_idx, think_idx, action))
        };
        let editor = text_editor(editor_content)
            .on_action(on_action)
            .size(14.0)
            .line_height(chat_text_line_height())
            .padding(0)
            .height(Length::Shrink)
            .max_height(READ_ONLY_EDITOR_MAX_HEIGHT)
            .font(chat_text_font())
            .style(move |theme: &Theme, _status: text_editor::Status| {
                let value = think_block_text_color(theme);
                read_only_text_style(theme, value)
            });
        return container(pass_wheel_to_parent(editor))
            .width(Length::Fill)
            .style(|_theme: &Theme| iced::widget::container::Style {
                background: None,
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 10.0.into() },
                ..Default::default()
            })
            .into();
    }

    if should_segment_text_block(&content) {
        let mut blocks = column![].spacing(10);
        for segment in split_text_segments(&content) {
            blocks = blocks.push(
                container(
                    text(segment)
                        .size(14)
                        .font(chat_text_font())
                        .line_height(chat_text_line_height())
                        .style(|theme: &Theme| iced::widget::text::Style {
                            color: Some(think_block_text_color(theme)),
                        }),
                )
                .width(Length::Fill)
                .style(|_theme: &Theme| iced::widget::container::Style {
                    background: None,
                    border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 10.0.into() },
                    ..Default::default()
                }),
            );
        }
        blocks.into()
    } else {
        container(
            text(content)
                .size(14)
                .font(chat_text_font())
                .line_height(chat_text_line_height())
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(think_block_text_color(theme)),
                }),
        )
        .width(Length::Fill)
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: None,
            border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 10.0.into() },
            ..Default::default()
        })
        .into()
    }
}

pub(crate) fn should_prefer_plain_think_body(is_streaming_msg: bool, is_thinking: bool) -> bool {
    is_streaming_msg || is_thinking
}

pub(crate) fn estimate_message_height_rough(raw: &str) -> f32 {
    let normalized = normalize_display_text(raw.trim());
    let normalized = normalized.trim();
    let text_len = normalized.len();
    let explicit_lines = normalized.lines().count().max(1) as f32;
    let wrapped_lines = text_len.div_ceil(96).max(1) as f32;
    let line_count = explicit_lines.max(wrapped_lines);
    let think_bonus = normalized.matches("<think").count() as f32 * 28.0;
    let tool_bonus = normalized.matches("tool ").count() as f32 * 36.0;
    (line_count * 22.0 + think_bonus + tool_bonus + 56.0).clamp(64.0, 720.0)
}

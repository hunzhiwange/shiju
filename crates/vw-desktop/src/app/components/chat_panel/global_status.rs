//! 聊天面板底部全局状态栏。

use std::cell::RefCell;

use iced::widget::svg;
use iced::widget::{Space, container, row, text};
use iced::{Alignment, Color, Element, Length, Theme};
use serde_json::Value;

use crate::app::components::chat_panel::tools::{
    ToolVisual, ToolVisualState, tool_inline_summary_unbounded, tool_input, tool_name_from_raw,
    tool_status, tool_verb, tool_visual_state,
};
use crate::app::components::status_animation::spinner_frame;
use crate::app::models::{ChatMessage, ChatRole};
use crate::app::{App, Message};

use super::utils::{bold_font, chat_secondary_subtle_text_color, icon_svg, truncate_chars};

/// 全局状态和思考头部共用的固定活动槽位。
pub(crate) const COMPACT_ACTIVITY_SLOT_SIZE: f32 = 18.0;
pub(crate) const COMPACT_ACTIVITY_ICON_SIZE: f32 = 14.0;
pub(crate) const COMPACT_ACTIVITY_TEXT_SIZE: f32 = 12.0;

/// 底部思考状态的强调色：主题蓝。
const THINK_PULSE_ACCENT: Color = Color::from_rgb(0.235, 0.510, 0.961);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChatGlobalStatus {
    pub action: String,
    pub detail: String,
    visual: ToolVisual,
    is_thinking: bool,
}

thread_local! {
    /// 缓存上一次解析出的状态及其指纹，避免高频动画帧重复全量解析末条消息造成卡顿。
    static STATUS_MEMO: RefCell<Option<(StatusFingerprint, Option<ChatGlobalStatus>)>> =
        const { RefCell::new(None) };
}

/// 末条助手消息的轻量指纹：长度变化即代表流式内容更新，远比重新解析便宜。
#[derive(Clone, Copy, PartialEq, Eq)]
struct StatusFingerprint {
    chat_len: usize,
    content_len: usize,
    tail_hash: u64,
    is_requesting: bool,
}

fn status_fingerprint(chat: &[ChatMessage], is_requesting: bool) -> StatusFingerprint {
    let last_assistant = chat.iter().rev().find(|message| message.role == ChatRole::Assistant);
    let content = last_assistant.map(|m| m.content.as_str()).unwrap_or("");
    // 取末尾一小段做哈希，兜住“长度不变但内容被改写”的少见情况；按字符边界回退避免切断多字节字符。
    let mut tail_start = content.len().saturating_sub(64);
    while tail_start < content.len() && !content.is_char_boundary(tail_start) {
        tail_start += 1;
    }
    let tail = &content[tail_start..];
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(tail, &mut hasher);
    StatusFingerprint {
        chat_len: chat.len(),
        content_len: content.len(),
        tail_hash: std::hash::Hasher::finish(&hasher),
        is_requesting,
    }
}

/// 带记忆的状态计算：指纹未变时直接复用上次结果，仅在消息真正变化时才全量解析。
fn chat_global_status_memoized(
    chat: &[ChatMessage],
    is_requesting: bool,
) -> Option<ChatGlobalStatus> {
    let fingerprint = status_fingerprint(chat, is_requesting);
    STATUS_MEMO.with(|memo| {
        if let Some((cached_fp, cached_status)) = memo.borrow().as_ref()
            && *cached_fp == fingerprint
        {
            return cached_status.clone();
        }
        let status = chat_global_status(chat, is_requesting);
        *memo.borrow_mut() = Some((fingerprint, status.clone()));
        status
    })
}

pub(crate) fn chat_global_status(
    chat: &[ChatMessage],
    is_requesting: bool,
) -> Option<ChatGlobalStatus> {
    if !is_requesting {
        return None;
    }

    let Some(last_assistant) =
        chat.iter().rev().find(|message| message.role == ChatRole::Assistant)
    else {
        return None;
    };

    if let Some(raw_tool) = last_tool_block(&last_assistant.content)
        && let Some(status) = status_from_tool_raw(&raw_tool, is_requesting)
    {
        return Some(status);
    }

    let (thinks, _, _thinking_open) = crate::app::ui::chat::split_think(&last_assistant.content);
    if is_requesting && !thinks.is_empty() {
        return Some(ChatGlobalStatus {
            action: "思考中".to_string(),
            detail: latest_think_detail(&thinks),
            visual: tool_visual_state("running"),
            is_thinking: true,
        });
    }

    None
}

pub(crate) fn chat_global_status_view(app: &App) -> Element<'_, Message> {
    let Some(status) =
        chat_global_status_memoized(&app.chat, app.current_session_runtime().is_requesting)
    else {
        return Space::new().height(Length::Fixed(0.0)).into();
    };
    let is_thinking = status.is_thinking;
    let action = status.action;

    // 思考中恢复原有星形呼吸动画；工具状态继续使用紧凑状态图标。
    let (activity_icon, line_height) = if is_thinking {
        (think_pulse_indicator(app.status_animation_frame), THINK_PULSE_MAX_SIZE)
    } else {
        (
            compact_activity_indicator(status.visual, app.status_animation_frame),
            COMPACT_ACTIVITY_SLOT_SIZE,
        )
    };

    // 进行内容保留完整单行，由剩余布局宽度动态裁切，不再按固定字符数提前截短。
    let detail_line = single_line(&status.detail);
    let content = if detail_line.is_empty() { action } else { format!("{action} {detail_line}") };

    // 时间、token 紧跟图标，进行内容放到最后。
    let mut meta_parts = Vec::new();
    let elapsed_ms = app
        .current_session_runtime_ref()
        .map(|runtime| runtime.llm_timer.elapsed_ms_at(web_time::Instant::now()))
        .unwrap_or(0);
    meta_parts.push(format_elapsed(elapsed_ms / 1_000));
    if let Some(tokens) = tokens_label(&app.usage) {
        meta_parts.push(tokens);
    }
    meta_parts.push(content);
    let meta_text = meta_parts.join(" · ");

    let meta_view = container(
        text(meta_text)
            .size(COMPACT_ACTIVITY_TEXT_SIZE)
            .width(Length::Fill)
            .wrapping(iced::widget::text::Wrapping::None)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(chat_secondary_subtle_text_color(theme)),
            }),
    )
    .width(Length::Fill)
    .clip(true);

    let line = row![activity_icon, meta_view]
        .spacing(5)
        .width(Length::Fill)
        .height(Length::Fixed(line_height))
        .align_y(Alignment::Center)
        .clip(true);

    container(line)
        .width(Length::Fill)
        .height(Length::Shrink)
        .padding(iced::Padding { top: 4.0, right: 28.0, bottom: 6.0, left: 12.0 })
        .into()
}

/// 脉冲图标循环的四种星型字形，靠近 Claude Code 的跳动效果。
const THINK_PULSE_GLYPHS: [&str; 4] = ["✳", "✶", "✷", "❋"];

/// 动画帧每 90ms 自增；每约半秒切换一种字形。
const THINK_PULSE_FRAMES_PER_GLYPH: usize = 6;

pub(crate) fn think_pulse_glyph(frame: usize) -> &'static str {
    THINK_PULSE_GLYPHS[(frame / THINK_PULSE_FRAMES_PER_GLYPH) % THINK_PULSE_GLYPHS.len()]
}

const THINK_PULSE_MIN_SIZE: f32 = 14.0;
const THINK_PULSE_MAX_SIZE: f32 = 44.0;
const THINK_PULSE_BREATH_PERIOD: usize = THINK_PULSE_FRAMES_PER_GLYPH * 2;
const THINK_PULSE_VERTICAL_OFFSET: f32 = 3.0;

pub(crate) fn think_pulse_size(frame: usize) -> f32 {
    let phase = (frame % THINK_PULSE_BREATH_PERIOD) as f32 / THINK_PULSE_BREATH_PERIOD as f32
        * std::f32::consts::TAU;
    let wave = (1.0 - phase.cos()) / 2.0;
    THINK_PULSE_MIN_SIZE + wave * (THINK_PULSE_MAX_SIZE - THINK_PULSE_MIN_SIZE)
}

fn think_pulse_indicator<'a>(frame: usize) -> Element<'a, Message> {
    container(
        text(think_pulse_glyph(frame))
            .size(think_pulse_size(frame))
            .line_height(iced::widget::text::LineHeight::Relative(1.0))
            .font(bold_font())
            .style(|_theme: &Theme| iced::widget::text::Style { color: Some(THINK_PULSE_ACCENT) }),
    )
    .width(Length::Fixed(THINK_PULSE_MAX_SIZE))
    .height(Length::Fixed(THINK_PULSE_MAX_SIZE))
    // 星形字形的视觉中心偏低，增加底部留白将其向上补偿半个留白高度。
    .padding(iced::Padding {
        top: 0.0,
        right: 0.0,
        bottom: THINK_PULSE_VERTICAL_OFFSET * 2.0,
        left: 0.0,
    })
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .into()
}

/// 活动帧只替换图形，几何尺寸始终保持不变。
pub(crate) fn compact_activity_geometry(_frame: usize) -> (f32, f32) {
    (COMPACT_ACTIVITY_SLOT_SIZE, COMPACT_ACTIVITY_ICON_SIZE)
}

pub(crate) fn compact_activity_glyph(frame: usize) -> &'static str {
    spinner_frame(frame)
}

pub(crate) fn compact_running_activity_indicator<'a>(frame: usize) -> Element<'a, Message> {
    compact_activity_indicator(tool_visual_state("running"), frame)
}

fn compact_activity_indicator<'a>(visual: ToolVisual, frame: usize) -> Element<'a, Message> {
    let (slot_size, icon_size) = compact_activity_geometry(frame);
    let indicator: Element<'a, Message> = if visual.is_animated {
        text(compact_activity_glyph(frame))
            .size(icon_size)
            .line_height(iced::widget::text::LineHeight::Relative(1.0))
            .style(move |theme: &Theme| iced::widget::text::Style {
                color: Some(visual.color(theme)),
            })
            .into()
    } else {
        icon_svg(visual.icon)
            .width(Length::Fixed(icon_size))
            .height(Length::Fixed(icon_size))
            .style(move |theme: &Theme, _status| svg::Style { color: Some(visual.color(theme)) })
            .into()
    };

    container(indicator)
        .width(Length::Fixed(slot_size))
        .height(Length::Fixed(slot_size))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into()
}

/// 把可能含换行的进行内容压成单行（空白折叠为单个空格）。
fn single_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 纯函数：把秒数格式化为 `X分 Y秒`（不足一分钟则 `Y秒`）。
pub(crate) fn format_elapsed(secs: u64) -> String {
    if secs >= 60 { format!("{}分 {}秒", secs / 60, secs % 60) } else { format!("{}秒", secs) }
}

/// 把累计 token 用量格式化为 `N tokens` / `N.Nk tokens`；无消耗返回 None。
pub(crate) fn tokens_label(usage: &crate::app::models::TokenUsage) -> Option<String> {
    let total =
        usage.input_tokens.max(0) + usage.output_tokens.max(0) + usage.reasoning_tokens.max(0);
    format_token_count(total, "tokens")
}

fn format_token_count(tokens: i64, suffix: &str) -> Option<String> {
    let tokens = tokens.max(0);
    if tokens <= 0 {
        return None;
    }
    if tokens >= 1000 {
        Some(format!("{:.1}k {}", tokens as f64 / 1000.0, suffix))
    } else {
        Some(format!("{} {}", tokens, suffix))
    }
}

fn status_from_tool_raw(raw: &str, is_requesting: bool) -> Option<ChatGlobalStatus> {
    let tool_name = tool_name_from_raw(raw)?;
    let (_, rest) = raw.split_once('\n')?;
    let value = serde_json::from_str::<Value>(rest.trim()).ok()?;
    let status = tool_status(&value);
    let input = tool_input(&value);
    let summary = tool_inline_summary_unbounded(&tool_name, input).unwrap_or_default();
    let verb = tool_verb(&tool_name);
    let detail = if summary.trim().is_empty() { tool_name.clone() } else { summary };

    let visual = if is_requesting && status.is_empty() {
        tool_visual_state("running")
    } else {
        tool_visual_state(status)
    };
    let action = match visual.state {
        ToolVisualState::Waiting => format!("等待{}", verb),
        ToolVisualState::Running => format!("正在{}", verb),
        ToolVisualState::Failed => format!("{}失败", verb),
        ToolVisualState::Cancelled => format!("{}已取消", verb),
        ToolVisualState::Completed | ToolVisualState::Unknown => return None,
    };

    Some(ChatGlobalStatus { action, detail, visual, is_thinking: false })
}

fn latest_think_detail(thinks: &[String]) -> String {
    let text = thinks.last().map(String::as_str).unwrap_or_default();
    let mut lines = text
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();

    if lines.is_empty() {
        return String::new();
    }

    if lines.len() > 2 {
        lines = lines.split_off(lines.len() - 2);
    }

    lines.into_iter().map(|line| truncate_chars(line.trim(), 92)).collect::<Vec<_>>().join("\n")
}

fn last_tool_block(raw: &str) -> Option<String> {
    let mut rest = raw;
    let mut last = None;
    while let Some(pos) = find_tool_start(rest) {
        let tool = &rest[pos..];
        let Some(consumed) = parse_tool_block(tool) else {
            break;
        };
        last = Some(tool[..consumed].to_string());
        rest = &tool[consumed..];
    }
    last
}

fn find_tool_start(s: &str) -> Option<usize> {
    if s.starts_with("tool ") && parse_tool_block(s).is_some() {
        return Some(0);
    }
    let mut search_from = 0usize;
    while let Some(pos_rel) = s[search_from..].find("\ntool ") {
        let pos = search_from + pos_rel + 1;
        if parse_tool_block(&s[pos..]).is_some() {
            return Some(pos);
        }
        search_from = pos + 1;
    }
    None
}

fn parse_tool_block(s: &str) -> Option<usize> {
    if !s.starts_with("tool ") {
        return None;
    }

    let line_end = s.find('\n')?;
    let mut idx = line_end + 1;
    let mut buf = String::new();

    for _ in 0..64 {
        if idx >= s.len() {
            break;
        }

        let next_end = s[idx..].find('\n').map(|offset| idx + offset).unwrap_or(s.len());
        let line = &s[idx..next_end];
        if !buf.is_empty() {
            buf.push('\n');
        }
        buf.push_str(line);

        if serde_json::from_str::<Value>(buf.trim()).is_ok() {
            return Some(if next_end < s.len() { next_end + 1 } else { next_end });
        }
        if next_end >= s.len() {
            break;
        }
        idx = next_end + 1;
    }

    None
}

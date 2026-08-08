//! 聊天面板底部全局状态栏。

use std::cell::RefCell;

use iced::widget::{Space, container, row, text};
use iced::{Alignment, Color, Element, Length, Theme};
use serde_json::Value;

use crate::app::components::chat_panel::tools::{
    tool_inline_summary, tool_input, tool_name_from_raw, tool_status, tool_verb,
};
use crate::app::models::{ChatMessage, ChatRole};
use crate::app::{App, Message};

use super::utils::{bold_font, chat_secondary_subtle_text_color, truncate_chars};

/// 运行状态强调色：主题蓝。
const PULSE_ACCENT: Color = Color::from_rgb(0.235, 0.510, 0.961);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChatGlobalStatus {
    pub action: String,
    pub detail: String,
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
            action: "正在思考".to_string(),
            detail: latest_think_detail(&thinks),
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
    let action = status.action;

    // 单行：呼吸放大缩小的脉冲图标 + 运行时长 · token 消耗 · 当前进行内容。
    let glyph = pulse_glyph(app.status_animation_frame);
    let glyph_size = pulse_size(app.status_animation_frame);
    // 固定尺寸容器内居中承载图标：字号变化时整行不抖动，且呼吸从中心向四周对称扩张。
    let pulse_icon = container(
        text(glyph)
            .size(glyph_size)
            .line_height(iced::widget::text::LineHeight::Relative(1.0))
            .font(bold_font())
            .style(move |_theme: &Theme| iced::widget::text::Style { color: Some(PULSE_ACCENT) }),
    )
    .width(Length::Fixed(PULSE_MAX_SIZE))
    .height(Length::Fixed(PULSE_MAX_SIZE))
    .align_x(Alignment::Center)
    .align_y(Alignment::Center);

    // 进行内容：状态文案（思考中/运行等）打头，后接最新 detail 单行、最多 40 字符、超出以 … 截断。
    let detail_line = single_line(&status.detail);
    let content = if detail_line.is_empty() {
        action
    } else {
        format!("{} {}", action, truncate_chars(&detail_line, 40))
    };

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

    // 文字顶部加内边距使其整体下移；整行居中后文字中心与图标中心对齐。
    let meta_view =
        container(text(meta_text).size(12).wrapping(iced::widget::text::Wrapping::None).style(
            |theme: &Theme| iced::widget::text::Style {
                color: Some(chat_secondary_subtle_text_color(theme)),
            },
        ))
        .padding(iced::Padding { top: 6.0, right: 0.0, bottom: 0.0, left: 0.0 });

    let line = row![pulse_icon, meta_view].spacing(5).align_y(Alignment::Center);

    container(line)
        .width(Length::Fill)
        .height(Length::Shrink)
        .padding(iced::Padding { top: 4.0, right: 28.0, bottom: 6.0, left: 12.0 })
        .into()
}

/// 脉冲图标循环的四种星型字形，靠近 Claude Code 的跳动效果。
const PULSE_GLYPHS: [&str; 4] = ["✳", "✶", "✷", "❋"];

/// 动画帧每 90ms 自增；每约半秒（6 帧）切换一种字形，循环四种形状。
const PULSE_FRAMES_PER_GLYPH: usize = 6;

/// 依据动画帧返回当前应显示的脉冲字形。
pub(crate) fn pulse_glyph(frame: usize) -> &'static str {
    PULSE_GLYPHS[(frame / PULSE_FRAMES_PER_GLYPH) % PULSE_GLYPHS.len()]
}

/// 呼吸动画的最小、最大字号；幅度更大以显华丽。
const PULSE_MIN_SIZE: f32 = 14.0;
const PULSE_MAX_SIZE: f32 = 34.0;

/// 呼吸一次完整放大缩小所用帧数；比字形切换慢一倍，呼吸更从容。
const PULSE_BREATH_PERIOD: usize = PULSE_FRAMES_PER_GLYPH * 2;

/// 依据动画帧在 `PULSE_MIN_SIZE`~`PULSE_MAX_SIZE` 间做正弦呼吸。
pub(crate) fn pulse_size(frame: usize) -> f32 {
    let phase =
        (frame % PULSE_BREATH_PERIOD) as f32 / PULSE_BREATH_PERIOD as f32 * std::f32::consts::TAU;
    let wave = (1.0 - phase.cos()) / 2.0; // 0 → 1 → 0
    PULSE_MIN_SIZE + wave * (PULSE_MAX_SIZE - PULSE_MIN_SIZE)
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
    let summary = tool_inline_summary(&tool_name, input).unwrap_or_default();
    let verb = tool_verb(&tool_name);
    let detail = if summary.trim().is_empty() { tool_name.clone() } else { summary };

    let action = match status {
        "completed" => return None,
        "error" => format!("{}失败", verb),
        "running" => format!("正在{}", verb),
        _ if is_requesting && status.is_empty() => format!("正在{}", verb),
        _ => return None,
    };

    Some(ChatGlobalStatus { action, detail })
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

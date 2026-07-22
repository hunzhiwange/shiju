//! 解析助手消息中的分段内容。
//! 该模块把文本、工具块和结构化片段拆分为终端可渲染的稳定单元。

use super::super::theme::{
    ACCENT_CYAN, PENDING_BADGE_BG, PENDING_BADGE_FG, TEXT_MUTED, TEXT_SUBTLE, WARNING,
};
use super::todocards::{
    TodoCardData, parse_todoread_card_data, parse_todowrite_card_data, todo_status_symbol,
};
use super::{
    ThinkBlockMeta, render_tool_card, think_block_expanded, tool_summary_cli, truncate_chars_cli,
};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

/// AssistantSegment 描述该模块对外暴露的离散状态。
pub(crate) enum AssistantSegment {
    Text(String),
    Think { content: String, open: bool },
    Tool { name: String, summary: String, input_raw: String },
}

/// 执行 parse_assistant_segments 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) fn parse_assistant_segments(content: &str) -> Vec<AssistantSegment> {
    let lines: Vec<&str> = content.lines().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.starts_with("<think") {
            let mut j = i;
            let mut found_close = trimmed.contains("</think>");
            while !found_close && j + 1 < lines.len() {
                j += 1;
                found_close = lines[j].contains("</think>");
            }
            let content = lines[i..=j].join("\n");
            out.push(AssistantSegment::Think { content, open: !found_close });
            i = j.saturating_add(1);
            continue;
        }

        if let Some(tool_name) = line.strip_prefix("tool ").map(str::trim).filter(|s| !s.is_empty())
        {
            let mut buf = String::new();
            let mut consumed = 1usize;
            let mut j = i + 1;
            let mut parsed_ok = false;
            while j < lines.len() && consumed <= 64 {
                if !buf.is_empty() {
                    buf.push('\n');
                }
                buf.push_str(lines[j]);
                consumed = j.saturating_sub(i).saturating_add(1);
                if serde_json::from_str::<serde_json::Value>(buf.trim()).is_ok() {
                    parsed_ok = true;
                    break;
                }
                j = j.saturating_add(1);
            }

            if parsed_ok {
                let summary = tool_summary_cli(tool_name, &buf);
                out.push(AssistantSegment::Tool {
                    name: tool_name.to_string(),
                    summary,
                    input_raw: buf,
                });
                i = i.saturating_add(consumed);
                continue;
            }
        }

        out.push(AssistantSegment::Text(line.to_string()));
        i = i.saturating_add(1);
    }
    out
}

/// 执行 assistant_segments_to_lines 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) fn assistant_segments_to_lines(
    segments: Vec<AssistantSegment>,
    expand_tool_details: bool,
    expand_think_content: bool,
) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    for seg in segments {
        match seg {
            AssistantSegment::Text(text) => out.push(Line::from(Span::raw(text))),
            AssistantSegment::Think { content, open } => {
                out.extend(render_think_card(&content, open, expand_think_content, None));
            }
            AssistantSegment::Tool { name, summary, input_raw } => {
                if name == "todowrite" || name == "todoread" {
                    let data = if name == "todowrite" {
                        parse_todowrite_card_data(&input_raw)
                    } else {
                        parse_todoread_card_data(&input_raw)
                    }
                    .unwrap_or_default();

                    out.extend(render_todo_card(&name, &summary, data, expand_tool_details));
                    continue;
                }

                out.extend(render_tool_card(&name, &summary, &input_raw, expand_tool_details));
            }
        }
    }
    out
}

/// 执行 assistant_segments_to_lines_with_meta 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) fn assistant_segments_to_lines_with_meta(
    segments: Vec<AssistantSegment>,
    expand_tool_details: bool,
    expand_think_all: bool,
    think_detail_overrides: &BTreeSet<u64>,
    think_duration_secs: Option<u64>,
    think_id_salt: u64,
) -> (Vec<Line<'static>>, Vec<Option<ThinkBlockMeta>>) {
    let mut out = Vec::new();
    let mut meta = Vec::new();
    let mut think_idx = 0_u64;
    for seg in segments {
        match seg {
            AssistantSegment::Text(text) => {
                out.push(Line::from(Span::raw(text)));
                meta.push(None);
            }
            AssistantSegment::Think { content, open } => {
                let think_id = think_block_id(think_id_salt, think_idx);
                think_idx = think_idx.saturating_add(1);
                let think_meta = ThinkBlockMeta { id: think_id, open };
                let expanded =
                    think_block_expanded(think_meta, expand_think_all, think_detail_overrides);
                let rendered = render_think_card(&content, open, expanded, think_duration_secs);
                let rendered_len = rendered.len();
                out.extend(rendered);
                meta.extend(std::iter::repeat_n(Some(think_meta), rendered_len));
            }
            AssistantSegment::Tool { name, summary, input_raw } => {
                if name == "todowrite" || name == "todoread" {
                    let data = if name == "todowrite" {
                        parse_todowrite_card_data(&input_raw)
                    } else {
                        parse_todoread_card_data(&input_raw)
                    }
                    .unwrap_or_default();

                    let rendered = render_todo_card(&name, &summary, data, expand_tool_details);
                    out.extend(rendered);
                    meta.resize(out.len(), None);
                    continue;
                }

                let rendered = render_tool_card(&name, &summary, &input_raw, expand_tool_details);
                let rendered_len = rendered.len();
                out.extend(rendered);
                meta.extend(std::iter::repeat_n(None, rendered_len));
            }
        }
    }
    (out, meta)
}

fn render_todo_card(
    name: &str,
    summary: &str,
    data: TodoCardData,
    expand_tool_details: bool,
) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let title = if name == "todowrite" { "任务更新" } else { "任务列表" };
    out.push(Line::from(vec![
        Span::styled("[T] ", Style::default().fg(ACCENT_CYAN)),
        Span::styled(title.to_string(), Style::default().fg(ACCENT_CYAN)),
        Span::styled(format!(" · {} 项", data.total), Style::default().fg(TEXT_SUBTLE)),
        if summary.is_empty() {
            Span::raw("")
        } else {
            Span::styled(
                format!(" · {}", truncate_chars_cli(summary, 72)),
                Style::default().fg(TEXT_SUBTLE),
            )
        },
    ]));
    out.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!(" ✓{} ", data.done),
            Style::default().fg(Color::Black).bg(Color::Green),
        ),
        Span::raw(" "),
        Span::styled(
            format!(" ·{} ", data.running),
            Style::default().fg(Color::Black).bg(Color::Yellow),
        ),
        Span::raw(" "),
        Span::styled(
            format!(" ○{} ", data.pending),
            Style::default().fg(PENDING_BADGE_FG).bg(PENDING_BADGE_BG),
        ),
    ]));

    let preview_count =
        if expand_tool_details { data.items.len() } else { data.items.len().min(3) };
    for (status, content) in data.items.into_iter().take(preview_count) {
        out.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(todo_status_symbol(&status), Style::default().fg(ACCENT_CYAN)),
            Span::raw(" "),
            Span::raw(truncate_chars_cli(&content, 110)),
        ]));
    }
    if !expand_tool_details && data.total > preview_count {
        out.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("... +{} more", data.total.saturating_sub(preview_count)),
                Style::default().fg(TEXT_SUBTLE),
            ),
        ]));
    }
    out.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            if expand_tool_details { "Ctrl+T 收起详情" } else { "Ctrl+T 展开详情" }.to_string(),
            Style::default().fg(TEXT_SUBTLE),
        ),
    ]));
    out
}

fn render_think_card(
    content: &str,
    open: bool,
    expanded: bool,
    think_duration_secs: Option<u64>,
) -> Vec<Line<'static>> {
    let lines = content.lines().count().max(1);
    let status = if open { "思考中" } else { "思考" };
    let preview = think_preview_line(content);
    let mut out = Vec::new();

    out.push(Line::from(vec![
        Span::styled(status.to_string(), Style::default().fg(WARNING).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" · {lines} 行"), Style::default().fg(TEXT_SUBTLE)),
        Span::styled(
            think_duration_secs.map(|secs| format!(" · {secs} 秒")).unwrap_or_default(),
            Style::default().fg(TEXT_SUBTLE),
        ),
    ]));

    if !preview.is_empty() {
        out.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(truncate_chars_cli(&preview, 100), Style::default().fg(TEXT_MUTED)),
        ]));
    }

    if expanded {
        for raw in content.lines() {
            let cleaned = strip_think_tags_inline(raw);
            if cleaned.trim().is_empty() {
                continue;
            }
            out.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(cleaned, Style::default().fg(TEXT_MUTED)),
            ]));
        }
    }

    out
}

fn think_preview_line(content: &str) -> String {
    for raw in content.lines() {
        let cleaned = strip_think_tags_inline(raw);
        let trimmed = cleaned.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    String::new()
}

fn strip_think_tags_inline(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut i = 0usize;
    while i < line.len() {
        let rest = &line[i..];
        let open_pos = rest.find("<think");
        let close_pos = rest.find("</think");
        let next = match (open_pos, close_pos) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        let Some(rel) = next else {
            out.push_str(rest);
            break;
        };

        out.push_str(&rest[..rel]);
        let tag_start = i + rel;
        let after_start = &line[tag_start..];
        if let Some(tag_end_rel) = after_start.find('>') {
            i = tag_start + tag_end_rel + 1;
        } else {
            break;
        }
    }
    out
}

fn think_block_id(salt: u64, think_idx: u64) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    salt.hash(&mut h);
    think_idx.hash(&mut h);
    h.finish()
}

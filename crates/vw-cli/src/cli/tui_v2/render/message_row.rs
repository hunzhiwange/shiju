//! message row renderer 集合。
//!
//! 本模块只负责把已经稳定派生好的 `UiMessage` / `TuiTranscriptItem` 渲染成文本行，
//! 不参与 scroll window、sticky prompt、未读边界或 footer pill 的状态判断。
//! 这样 S4-3 可以把“状态层派生”和“单行渲染细节”拆开，避免继续把两层揉在一起。

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::cli::tui_v2::model::{
    UiAssistantMessage, UiMessage, UiMessageBase, UiSystemMessageLevel, UiThinkingBlock,
    UiThinkingPhase, UiTurnTerminal,
};
use crate::cli::tui_v2::state::selectors::{
    TuiToolCallGroup, select_tool_row_view,
};
use crate::cli::tui_v2::state::{TuiAssistantTurnEntry, TuiTranscriptItem};
use crate::cli::tui_v2::terminal_capabilities::sanitize_terminal_text;

use super::TuiTheme;
use super::markdown::render_markdown;

pub(super) fn render_transcript_item_lines(
    item: &TuiTranscriptItem<'_>,
    theme: TuiTheme,
) -> Vec<Line<'static>> {
    render_transcript_item_lines_at_width(item, theme, 80)
}

pub(super) fn render_transcript_item_lines_at_width(
    item: &TuiTranscriptItem<'_>,
    theme: TuiTheme,
    width: u16,
) -> Vec<Line<'static>> {
    render_transcript_item_lines_with_options(item, theme, width, false)
}

pub(super) fn render_transcript_item_lines_with_options(
    item: &TuiTranscriptItem<'_>,
    theme: TuiTheme,
    width: u16,
    verbose: bool,
) -> Vec<Line<'static>> {
    render_transcript_item_lines_with_context(
        item,
        theme,
        width,
        verbose,
        crate::cli::tui_v2::terminal_capabilities::TerminalCapabilities::default(),
        None,
    )
}

pub(super) fn render_transcript_item_lines_with_context(
    item: &TuiTranscriptItem<'_>,
    theme: TuiTheme,
    width: u16,
    verbose: bool,
    capabilities: crate::cli::tui_v2::terminal_capabilities::TerminalCapabilities,
    workspace_root: Option<&str>,
) -> Vec<Line<'static>> {
    match item {
        TuiTranscriptItem::Standalone(message) => {
            render_message_lines(
                message,
                theme,
                width,
                verbose,
                capabilities,
                workspace_root,
            )
        }
        TuiTranscriptItem::AssistantTurn(turn) => {
            let mut lines = Vec::new();

            for (index, entry) in turn.preface.iter().enumerate() {
                lines.extend(render_assistant_turn_entry_lines(entry, theme, true, verbose));
                if index + 1 < turn.preface.len() || !turn.children.is_empty() {
                    lines.push(Line::raw(""));
                }
            }

            for (index, entry) in turn.children.iter().enumerate() {
                lines.extend(render_assistant_turn_entry_lines(entry, theme, false, verbose));
                if index + 1 < turn.children.len() {
                    lines.push(Line::raw(""));
                }
            }

            let assistant_lines = render_assistant_lines(turn.assistant, theme, width, verbose);
            if !lines.is_empty() && !assistant_lines.is_empty() {
                lines.push(Line::raw(""));
            }
            lines.extend(assistant_lines);
            lines
        }
    }
}

fn render_message_lines(
    message: &UiMessage,
    theme: TuiTheme,
    width: u16,
    verbose: bool,
    capabilities: crate::cli::tui_v2::terminal_capabilities::TerminalCapabilities,
    workspace_root: Option<&str>,
) -> Vec<Line<'static>> {
    match message {
        UiMessage::User(message) => {
            if message.text.is_empty() {
                return Vec::new();
            }
            let mut lines =
                render_symbol_text("❯", theme.accent, message.text.as_str(), theme);
            append_verbose_base(&mut lines, &message.base, None, theme, verbose);
            lines
        }
        UiMessage::Assistant(message) => render_assistant_lines(message, theme, width, verbose),
        UiMessage::Step(step) => vec![Line::from(vec![
            label_span("步骤", theme.warning),
            Span::raw(" "),
            Span::raw(format!(
                "#{} {:?} · {}",
                step.step_index,
                step.state,
                step.finish_reason.as_deref().unwrap_or("进行中")
            )),
        ])],
        UiMessage::System(message) if message.level == UiSystemMessageLevel::Info && !verbose => {
            Vec::new()
        }
        UiMessage::System(message) => render_symbol_text(
            if message.level == UiSystemMessageLevel::Success { "✓" } else { "●" },
            system_level_color(message.level, theme),
            message.text.as_str(),
            theme,
        ),
        UiMessage::ToolCall(message) => {
            let group = TuiToolCallGroup {
                call: message,
                results: Vec::new(),
                progress: Vec::new(),
                notices: Vec::new(),
            };
            let view = select_tool_row_view(&group);
            super::tool_row::render_tool_row(&view, theme, verbose)
        }
        UiMessage::ToolResult(message) => render_text_block(
            if message.is_error { "失败" } else { "结果" },
            if message.is_error { theme.danger } else { theme.success },
            message.content.as_str(),
            theme,
        ),
        UiMessage::Thinking(message) => render_thinking_lines(message, theme, verbose),
        UiMessage::Error(message) => {
            render_symbol_text("●", theme.danger, message.message.as_str(), theme)
        }
        UiMessage::Attachment(message) => super::attachment::render_attachment(
            message,
            1,
            capabilities,
            workspace_root,
            theme,
            width,
        )
        .lines,
        UiMessage::Progress(message) => render_text_block(
            "进度",
            theme.warning,
            format!(
                "{} · {}/{}",
                message.label,
                message.current.unwrap_or_default(),
                message.total.unwrap_or_default()
            )
            .as_str(),
            theme,
        ),
        UiMessage::CompactBoundary(message) => {
            let mut lines = vec![Line::styled(
                "✻ Conversation compacted (Ctrl+O for history)",
                theme.muted_style(),
            )];
            if verbose {
                lines.push(Line::styled(
                    format!(
                        "  {} · {} messages",
                        message.summary, message.compacted_messages
                    ),
                    theme.muted_style(),
                ));
            }
            lines
        }
        UiMessage::Unknown(message) => {
            render_symbol_text("●", theme.danger, message.summary.as_str(), theme)
        }
    }
}

fn render_assistant_lines(
    message: &UiAssistantMessage,
    theme: TuiTheme,
    width: u16,
    verbose: bool,
) -> Vec<Line<'static>> {
    if message.text.trim().is_empty() {
        return Vec::new();
    }
    let mut lines =
        render_markdown_with_prefix("●", theme.text, message.text.as_str(), width, theme);
    if !verbose {
        return lines;
    }
    let mut detail = vec![
        Span::styled("  ", theme.muted_style()),
        Span::styled(
            terminal_label(&message.terminal),
            theme.fg_style(terminal_color(&message.terminal, theme)),
        ),
    ];

    if message.step_count > 0 {
        detail.push(Span::styled(format!(" · {} 步", message.step_count), theme.muted_style()));
    }
    if let Some(model) = message.model.as_deref() {
        detail.push(Span::styled(format!(" · {model}"), theme.muted_style()));
    }
    if let Some(created_ms) = message.base.created_ms {
        detail.push(Span::styled(format!(" · @{created_ms}ms"), theme.muted_style()));
    }

    lines.push(Line::from(detail));
    lines
}

fn render_markdown_with_prefix(
    symbol: &str,
    color: ratatui::style::Color,
    text: &str,
    width: u16,
    theme: TuiTheme,
) -> Vec<Line<'static>> {
    if text.is_empty() {
        return Vec::new();
    }
    let sanitized = sanitize_terminal_text(text);
    let markdown = render_markdown(sanitized.as_str(), width.saturating_sub(2), theme);
    markdown
        .lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            let mut spans = if index == 0 {
                vec![label_span(symbol, color), Span::raw(" ")]
            } else {
                vec![Span::styled("  ", theme.muted_style())]
            };
            spans.extend(line.spans);
            Line::from(spans)
        })
        .collect()
}

fn render_symbol_text(
    symbol: &str,
    color: ratatui::style::Color,
    text: &str,
    theme: TuiTheme,
) -> Vec<Line<'static>> {
    let sanitized = sanitize_terminal_text(text);
    let mut lines = sanitized.lines();
    let first = lines.next().unwrap_or_default();
    let mut rendered =
        vec![Line::from(vec![label_span(symbol, color), Span::raw(" "), Span::raw(first.to_string())])];
    rendered.extend(lines.map(|line| {
        Line::from(vec![
            Span::styled("  ", theme.muted_style()),
            Span::styled(line.to_string(), theme.body_style()),
        ])
    }));
    rendered
}

fn append_verbose_base(
    lines: &mut Vec<Line<'static>>,
    base: &UiMessageBase,
    detail: Option<&str>,
    theme: TuiTheme,
    verbose: bool,
) {
    if !verbose {
        return;
    }
    let mut text = String::from("  ");
    if let Some(created_ms) = base.created_ms {
        text.push_str(format!("@{created_ms}ms").as_str());
    }
    if let Some(detail) = detail {
        if text.len() > 2 {
            text.push_str(" · ");
        }
        text.push_str(detail);
    }
    if text.len() > 2 {
        lines.push(Line::styled(text, theme.muted_style()));
    }
}

fn render_assistant_turn_entry_lines(
    entry: &TuiAssistantTurnEntry<'_>,
    theme: TuiTheme,
    is_preface: bool,
    verbose: bool,
) -> Vec<Line<'static>> {
    let phase = if is_preface { "前置" } else { "子项" };

    match entry {
        TuiAssistantTurnEntry::Thinking(message) => render_thinking_lines(message, theme, verbose),
        TuiAssistantTurnEntry::Step(step) => vec![Line::from(vec![
            label_span(format!("{phase}步骤"), theme.warning),
            Span::raw(" "),
            Span::raw(format!(
                "#{} {:?} · {}",
                step.step_index,
                step.state,
                step.finish_reason.as_deref().unwrap_or("进行中")
            )),
        ])],
        TuiAssistantTurnEntry::Tool(tool_group) => {
            let view = select_tool_row_view(tool_group);
            super::tool_row::render_tool_row(&view, theme, verbose)
        }
        TuiAssistantTurnEntry::ToolResult(message) => render_text_block(
            format!("{phase}结果"),
            if message.is_error { theme.danger } else { theme.success },
            message.content.as_str(),
            theme,
        ),
        TuiAssistantTurnEntry::CollapsedTools(batch) if verbose => {
            let mut lines = Vec::new();
            for group in &batch.calls {
                if !lines.is_empty() {
                    lines.push(Line::raw(""));
                }
                let view = select_tool_row_view(group);
                lines.extend(super::tool_row::render_tool_row(&view, theme, true));
            }
            lines
        }
        TuiAssistantTurnEntry::CollapsedTools(batch) => {
            let mut detail = format!("{} · {} 条结果", batch.summary, batch.total_results);
            if batch.total_progress > 0 {
                detail.push_str(format!(" · {} 条进度", batch.total_progress).as_str());
            }
            if batch.notice_count > 0 {
                detail.push_str(format!(" · {} 条通知", batch.notice_count).as_str());
            }
            vec![Line::from(vec![
                label_span(format!("{phase}工具组"), theme.accent),
                Span::raw(" "),
                Span::raw(detail),
            ])]
        }
    }
}

fn render_thinking_lines(
    message: &UiThinkingBlock,
    theme: TuiTheme,
    verbose: bool,
) -> Vec<Line<'static>> {
    match message.phase {
        UiThinkingPhase::HiddenHistory => Vec::new(),
        UiThinkingPhase::Redacted => {
            vec![Line::styled("∴ Thinking (redacted)", theme.muted_style())]
        }
        UiThinkingPhase::Streaming if !verbose => {
            vec![Line::styled("∴ Thinking…", theme.muted_style())]
        }
        UiThinkingPhase::Closed if !verbose => vec![Line::styled(
            "∴ Thinking (Ctrl+O to expand)",
            theme.muted_style(),
        )],
        UiThinkingPhase::Streaming | UiThinkingPhase::Closed => {
            let mut lines = vec![Line::styled("∴ Thinking…", theme.muted_style())];
            lines.extend(
                render_markdown(sanitize_terminal_text(message.content.as_str()).as_str(), 78, theme)
                    .lines
                    .into_iter()
                    .map(|line| {
                        let mut spans = vec![Span::styled("  ", theme.muted_style())];
                        spans.extend(line.spans);
                        Line::from(spans)
                    }),
            );
            lines
        }
    }
}

fn render_text_block(
    label: impl Into<String>,
    color: ratatui::style::Color,
    text: &str,
    theme: TuiTheme,
) -> Vec<Line<'static>> {
    let label = label.into();
    let sanitized = sanitize_terminal_text(text);
    let mut raw_lines = sanitized.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
    if raw_lines.is_empty() {
        raw_lines.push(String::new());
    }

    let mut lines = Vec::with_capacity(raw_lines.len());
    for (index, line) in raw_lines.into_iter().enumerate() {
        if index == 0 {
            lines.push(Line::from(vec![
                label_span(label.clone(), color),
                Span::raw(" "),
                Span::raw(line),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("   ", theme.muted_style()),
                Span::styled(line, theme.body_style()),
            ]));
        }
    }

    lines
}

fn label_span(label: impl Into<String>, color: ratatui::style::Color) -> Span<'static> {
    Span::styled(label.into(), theme_label_style(color))
}

fn theme_label_style(color: ratatui::style::Color) -> Style {
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

fn terminal_label(terminal: &UiTurnTerminal) -> &'static str {
    match terminal {
        UiTurnTerminal::Pending => "待命",
        UiTurnTerminal::Streaming => "输出中",
        UiTurnTerminal::Done { .. } => "完成",
        UiTurnTerminal::Cancelled { .. } => "已取消",
        UiTurnTerminal::TimedOut { .. } => "超时",
        UiTurnTerminal::Error { .. } => "错误",
    }
}

fn terminal_color(terminal: &UiTurnTerminal, theme: TuiTheme) -> ratatui::style::Color {
    match terminal {
        UiTurnTerminal::Pending => theme.border,
        UiTurnTerminal::Streaming => theme.accent,
        UiTurnTerminal::Done { .. } => theme.success,
        UiTurnTerminal::Cancelled { .. } => theme.warning,
        UiTurnTerminal::TimedOut { .. } | UiTurnTerminal::Error { .. } => theme.danger,
    }
}

fn system_level_color(level: UiSystemMessageLevel, theme: TuiTheme) -> ratatui::style::Color {
    match level {
        UiSystemMessageLevel::Info => theme.accent,
        UiSystemMessageLevel::Warning => theme.warning,
        UiSystemMessageLevel::Success => theme.success,
    }
}

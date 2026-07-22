//! message row renderer 集合。
//!
//! 本模块只负责把已经稳定派生好的 `UiMessage` / `TuiTranscriptItem` 渲染成文本行，
//! 不参与 scroll window、sticky prompt、未读边界或 footer pill 的状态判断。
//! 这样 S4-3 可以把“状态层派生”和“单行渲染细节”拆开，避免继续把两层揉在一起。

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::cli::tui_v2::model::{
    UiAssistantMessage, UiMessage, UiSystemMessageLevel, UiTurnTerminal,
};
use crate::cli::tui_v2::state::{TuiAssistantTurnEntry, TuiTranscriptItem};

use super::TuiTheme;

pub(super) fn render_transcript_item_lines(
    item: &TuiTranscriptItem<'_>,
    theme: TuiTheme,
) -> Vec<Line<'static>> {
    match item {
        TuiTranscriptItem::Standalone(message) => render_message_lines(message, theme),
        TuiTranscriptItem::AssistantTurn(turn) => {
            let mut lines = Vec::new();

            for (index, entry) in turn.preface.iter().enumerate() {
                lines.extend(render_assistant_turn_entry_lines(entry, theme, true));
                if index + 1 < turn.preface.len() || !turn.children.is_empty() {
                    lines.push(Line::raw(""));
                }
            }

            for (index, entry) in turn.children.iter().enumerate() {
                lines.extend(render_assistant_turn_entry_lines(entry, theme, false));
                if index + 1 < turn.children.len() {
                    lines.push(Line::raw(""));
                }
            }

            if !lines.is_empty() {
                lines.push(Line::raw(""));
            }
            lines.extend(render_assistant_lines(turn.assistant, theme));
            lines
        }
    }
}

fn render_message_lines(message: &UiMessage, theme: TuiTheme) -> Vec<Line<'static>> {
    match message {
        UiMessage::User(message) => {
            render_text_block("你", theme.accent, message.text.as_str(), theme)
        }
        UiMessage::Assistant(message) => render_assistant_lines(message, theme),
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
        UiMessage::System(message) => render_text_block(
            "系统",
            system_level_color(message.level, theme),
            message.text.as_str(),
            theme,
        ),
        UiMessage::ToolCall(message) => {
            let mut lines = vec![Line::from(vec![
                label_span("工具", theme.warning),
                Span::raw(" "),
                Span::raw(format!("{} · {:?}", message.tool_name, message.state)),
            ])];
            if let Some(summary) =
                message.summary.as_ref().filter(|summary| !summary.trim().is_empty())
            {
                lines.push(Line::from(vec![
                    Span::styled("   ", theme.muted_style()),
                    Span::styled(summary.clone(), theme.muted_style()),
                ]));
            }
            lines
        }
        UiMessage::ToolResult(message) => render_text_block(
            if message.is_error { "失败" } else { "结果" },
            if message.is_error { theme.danger } else { theme.success },
            message.content.as_str(),
            theme,
        ),
        UiMessage::Thinking(message) => render_text_block(
            "思考",
            theme.muted,
            message.summary.as_deref().unwrap_or(message.content.as_str()),
            theme,
        ),
        UiMessage::Error(message) => {
            render_text_block("错误", theme.danger, message.message.as_str(), theme)
        }
    }
}

fn render_assistant_lines(message: &UiAssistantMessage, theme: TuiTheme) -> Vec<Line<'static>> {
    let mut lines = render_text_block("助手", theme.success, message.text.as_str(), theme);
    let mut detail = vec![
        Span::styled("   ", theme.muted_style()),
        Span::styled(
            terminal_label(&message.terminal),
            theme.fg_style(terminal_color(&message.terminal, theme)),
        ),
    ];

    if message.step_count > 0 {
        detail.push(Span::styled(format!(" · {} 步", message.step_count), theme.muted_style()));
    }

    lines.push(Line::from(detail));
    lines
}

fn render_assistant_turn_entry_lines(
    entry: &TuiAssistantTurnEntry<'_>,
    theme: TuiTheme,
    is_preface: bool,
) -> Vec<Line<'static>> {
    let phase = if is_preface { "前置" } else { "子项" };

    match entry {
        TuiAssistantTurnEntry::Thinking(message) => render_text_block(
            format!("{phase}思考"),
            theme.muted,
            message.summary.as_deref().unwrap_or(message.content.as_str()),
            theme,
        ),
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
        TuiAssistantTurnEntry::Tool(tool_group) => vec![Line::from(vec![
            label_span(format!("{phase}工具"), theme.warning),
            Span::raw(" "),
            Span::raw(format!(
                "{} · {:?} · {} 条结果",
                tool_group.call.tool_name,
                tool_group.call.state,
                tool_group.results.len(),
            )),
        ])],
        TuiAssistantTurnEntry::ToolResult(message) => render_text_block(
            format!("{phase}结果"),
            if message.is_error { theme.danger } else { theme.success },
            message.content.as_str(),
            theme,
        ),
        TuiAssistantTurnEntry::CollapsedTools(batch) => vec![Line::from(vec![
            label_span(format!("{phase}工具组"), theme.accent),
            Span::raw(" "),
            Span::raw(format!("{} · {} 条结果", batch.summary, batch.total_results)),
        ])],
    }
}

fn render_text_block(
    label: impl Into<String>,
    color: ratatui::style::Color,
    text: &str,
    theme: TuiTheme,
) -> Vec<Line<'static>> {
    let label = label.into();
    let mut raw_lines = text.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
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

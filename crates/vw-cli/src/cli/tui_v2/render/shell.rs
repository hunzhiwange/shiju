//! Shell 类工具输出渲染。

use ratatui::text::{Line, Span};

use super::TuiTheme;

const DEFAULT_OUTPUT_LINES: usize = 10;
const DEFAULT_OUTPUT_BYTES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ShellCardInput {
    pub(super) command: String,
    pub(super) cwd: Option<String>,
    pub(super) stdout: String,
    pub(super) stderr: String,
    pub(super) exit_code: Option<i32>,
    pub(super) duration_ms: Option<u64>,
    pub(super) background: bool,
    pub(super) running: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TruncatedShellOutput {
    pub(super) visible_lines: Vec<String>,
    pub(super) omitted_lines: usize,
    pub(super) omitted_bytes: usize,
}

pub(super) fn truncate_shell_output(
    output: &str,
    max_lines: usize,
    max_bytes: usize,
) -> TruncatedShellOutput {
    let mut byte_end = output.len().min(max_bytes);
    while !output.is_char_boundary(byte_end) {
        byte_end = byte_end.saturating_sub(1);
    }
    let byte_bounded = &output[..byte_end];
    let all_lines = output.lines().count();
    let bounded_lines = byte_bounded.lines().collect::<Vec<_>>();
    let visible_lines =
        bounded_lines.iter().take(max_lines).map(|line| (*line).to_string()).collect::<Vec<_>>();
    TruncatedShellOutput {
        omitted_lines: all_lines.saturating_sub(visible_lines.len()),
        omitted_bytes: output.len().saturating_sub(byte_end),
        visible_lines,
    }
}

pub(super) fn render_shell_card(
    input: &ShellCardInput,
    theme: TuiTheme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = vec![Line::styled(
        truncate_line(format!("$ {}", input.command).as_str(), usize::from(width)),
        theme.body_style(),
    )];
    if let Some(cwd) = input.cwd.as_deref() {
        lines.push(Line::styled(
            truncate_line(format!("  cwd {cwd}").as_str(), usize::from(width)),
            theme.muted_style(),
        ));
    }

    let stdout =
        truncate_shell_output(input.stdout.as_str(), DEFAULT_OUTPUT_LINES, DEFAULT_OUTPUT_BYTES);
    for output in &stdout.visible_lines {
        lines.push(Line::from(vec![
            Span::styled("│ ", theme.muted_style()),
            Span::styled(
                truncate_line(output, usize::from(width).saturating_sub(2)),
                theme.body_style(),
            ),
        ]));
    }
    if !input.stderr.trim().is_empty() {
        let stderr = truncate_shell_output(input.stderr.as_str(), 1, DEFAULT_OUTPUT_BYTES);
        if let Some(error) = stderr.visible_lines.first() {
            lines.push(Line::from(vec![
                Span::styled("! ", ratatui::style::Style::default().fg(theme.danger)),
                Span::styled(
                    truncate_line(error, usize::from(width).saturating_sub(2)),
                    ratatui::style::Style::default().fg(theme.danger),
                ),
            ]));
        }
    }
    if stdout.omitted_lines > 0 {
        lines.push(Line::styled(
            format!("… {} lines truncated (Ctrl+O to expand)", stdout.omitted_lines),
            theme.muted_style(),
        ));
    } else if stdout.omitted_bytes > 0 {
        lines.push(Line::styled(
            format!("… {} bytes truncated (Ctrl+O to expand)", stdout.omitted_bytes),
            theme.muted_style(),
        ));
    }

    let status = shell_status(input);
    lines.push(Line::styled(
        status,
        if input.running { theme.accent_style() } else { theme.muted_style() },
    ));
    lines
}

fn shell_status(input: &ShellCardInput) -> String {
    let mut parts = vec![if input.running {
        "running".to_string()
    } else {
        format!("exit {}", input.exit_code.unwrap_or_default())
    }];
    if let Some(duration_ms) = input.duration_ms {
        parts.push(format!("{:.1}s", duration_ms as f64 / 1_000.0));
    }
    if input.background {
        parts.push("background".to_string());
    }
    parts.join(" · ")
}

fn truncate_line(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let mut output = value.chars().take(width.saturating_sub(1)).collect::<String>();
    output.push('…');
    output
}

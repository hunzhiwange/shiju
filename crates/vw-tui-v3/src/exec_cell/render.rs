//! Ratatui layout for the copied command cell.

use std::time::Duration;

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::model::ExecCell;
use crate::motion::{MotionMode, ReducedMotionIndicator, activity_indicator};
use crate::width::display_width;
use crate::wrapping::{RtOptions, adaptive_wrap_line};

pub const TOOL_CALL_MAX_LINES: usize = 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutputLinesParams {
    pub max_lines: usize,
    pub width: usize,
}

pub fn output_lines(output: &str, params: OutputLinesParams) -> Vec<Line<'static>> {
    output
        .lines()
        .take(params.max_lines)
        .flat_map(|line| {
            let source = Line::from(line.to_string());
            adaptive_wrap_line(&source, RtOptions::new(params.width.max(1)))
                .into_iter()
                .map(|line| crate::render::line_utils::line_to_static(&line))
                .map(|line| line.style(Style::default().fg(Color::DarkGray)))
                .collect::<Vec<_>>()
        })
        .collect()
}

impl ExecCell {
    pub fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let width = usize::from(width).max(1);
        let mut lines = Vec::new();
        for call in &self.calls {
            let prefix =
                if call.source == super::model::ExecCommandSource::User { "$ " } else { "› " };
            let command = format!("{prefix}{}", call.command);
            let source = Line::from(command);
            let mut command_lines = adaptive_wrap_line(
                &source,
                RtOptions::new(width).subsequent_indent(Line::from("  ")),
            )
            .into_iter()
            .map(|line| crate::render::line_utils::line_to_static(&line))
            .collect::<Vec<_>>();
            lines.append(&mut command_lines);
            if call.output.is_none()
                && let Some(indicator) = activity_indicator(
                    call.start_time,
                    MotionMode::Reduced,
                    ReducedMotionIndicator::StaticBullet,
                )
            {
                lines.push(Line::from(vec![Span::raw("  "), indicator]));
            }
            if let Some(output) = &call.output {
                lines.extend(output_lines(
                    &output.output,
                    OutputLinesParams {
                        max_lines: TOOL_CALL_MAX_LINES,
                        width: width.saturating_sub(2).max(1),
                    },
                ));
                let status = if output.exit_code == 0 { "✓" } else { "✗" };
                let style = if output.exit_code == 0 {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                };
                let elapsed = call.duration.map(format_duration).unwrap_or_default();
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(format!("{status} {elapsed}"), style),
                ]));
            }
        }
        lines
    }
}

fn format_duration(duration: Duration) -> String {
    if duration.as_secs() > 0 {
        format!("{}s", duration.as_secs())
    } else {
        format!("{}ms", duration.as_millis())
    }
}

pub fn new_active_exec_command(call_id: String, tool: String, command: String) -> ExecCell {
    ExecCell::pending(call_id, tool, command)
}

pub fn rendered_width(lines: &[Line<'_>]) -> usize {
    lines.iter().map(|line| display_width(&line.to_string())).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exec_cell::CommandOutput;

    #[test]
    fn completed_exec_cell_keeps_command_output_and_status() {
        let cell = ExecCell::completed(
            "call-1".into(),
            "shell".into(),
            "printf ok".into(),
            CommandOutput::new(0, "ok".into()),
            Duration::from_millis(8),
        );
        let lines = cell.display_lines(40);
        assert!(lines.iter().any(|line| line.to_string().contains("printf ok")));
        assert!(lines.iter().any(|line| line.to_string().contains('✓')));
    }
}

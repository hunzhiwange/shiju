//! 复制而来的命令单元 Ratatui 布局。
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
    /// 单次工具输出最多展示的源行数。
    pub max_lines: usize,
    /// 每个源行折行时可用的终端宽度。
    pub width: usize,
}

/// 截取并折行命令输出，统一使用弱化样式以区别于命令本身。
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
    /// 按调用顺序构造命令、活动指示、输出和结束状态行。
    pub fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let width = usize::from(width).max(1);
        let mut lines = Vec::new();
        for call in &self.calls {
            let command = match call.source {
                super::model::ExecCommandSource::User => format!("$ {}", call.command),
                super::model::ExecCommandSource::Tool if call.command.trim().is_empty() => {
                    format!("› {}", call.tool)
                }
                super::model::ExecCommandSource::Tool => {
                    format!("› {} {}", call.tool, call.command)
                }
            };
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
    // 秒级耗时避免显示多余小数，短任务则保留毫秒粒度。
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
    // 使用终端显示宽度而非字节长度，兼容宽字符内容。
    lines.iter().map(|line| display_width(&line.to_string())).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exec_cell::CommandOutput;

    #[test]
    // 验证已完成单元同时保留命令、输出和成功状态，不只覆盖布局构造。
    fn completed_exec_cell_keeps_command_output_and_status() {
        let cell = ExecCell::completed(
            "call-1".into(),
            "shell".into(),
            "printf ok".into(),
            CommandOutput::new(0, "ok".into()),
            Duration::from_millis(8),
        );
        let lines = cell.display_lines(40);
        assert!(lines.iter().any(|line| line.to_string().contains("shell printf ok")));
        assert!(lines.iter().any(|line| line.to_string().contains("printf ok")));
        assert!(lines.iter().any(|line| line.to_string().contains('✓')));
    }
}

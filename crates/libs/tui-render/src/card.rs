//! 会话与状态卡共享的边框几何。

use ratatui::prelude::*;
use tui_text::line_truncation::line_width;

pub fn card_inner_width(width: u16, max_inner_width: usize) -> Option<usize> {
    if width < 4 {
        return None;
    }
    Some(std::cmp::min(usize::from(width.saturating_sub(4)), max_inner_width))
}

pub fn with_border_with_inner_width(
    lines: Vec<Line<'static>>,
    inner_width: usize,
) -> Vec<Line<'static>> {
    let max_line_width = lines.iter().map(line_width).max().unwrap_or(0);
    let content_width = inner_width.max(max_line_width);
    let border_inner_width = content_width + 2;
    let mut out = Vec::with_capacity(lines.len() + 2);
    out.push(Line::from(format!("╭{}╮", "─".repeat(border_inner_width))).dim());
    for line in lines {
        let used_width = line_width(&line);
        let mut spans = Vec::with_capacity(line.spans.len() + 3);
        spans.push("│ ".dim());
        spans.extend(line);
        if used_width < content_width {
            spans.push(Span::from(" ".repeat(content_width - used_width)).dim());
        }
        spans.push(" │".dim());
        out.push(Line::from(spans));
    }
    out.push(Line::from(format!("╰{}╯", "─".repeat(border_inner_width))).dim());
    out
}

//! ratatui 文本行的借用、所有权转换、空白判断与前缀处理工具。

use ratatui::text::Line;
use ratatui::text::Span;

/// Create a ratatui `Line` that borrows the contents of another line.
/// 创建借用另一行内容的 ratatui `Line`，并保留原样式与对齐方式。
pub fn line_to_borrowed<'a>(line: &'a Line<'_>) -> Line<'a> {
    Line {
        style: line.style,
        alignment: line.alignment,
        spans: line
            .spans
            .iter()
            .map(|span| Span {
                style: span.style,
                content: std::borrow::Cow::Borrowed(span.content.as_ref()),
            })
            .collect(),
    }
}

pub use tui_text::{line_to_static, push_owned_lines};

/// Prefix each line with `initial_prefix` for the first line and
/// `subsequent_prefix` for following lines. Returns a new Vec of owned lines.
/// 首行与后续行分别添加指定前缀，并返回新的拥有型行集合。
pub fn prefix_lines(
    lines: Vec<Line<'static>>,
    initial_prefix: Span<'static>,
    subsequent_prefix: Span<'static>,
) -> Vec<Line<'static>> {
    lines
        .into_iter()
        .enumerate()
        .map(|(i, l)| {
            let mut spans = Vec::with_capacity(l.spans.len() + 1);
            spans.push(if i == 0 { initial_prefix.clone() } else { subsequent_prefix.clone() });
            spans.extend(l.spans);
            Line::from(spans).style(l.style)
        })
        .collect()
}

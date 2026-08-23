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

/// Clone a borrowed ratatui `Line` into an owned `'static` line.
/// 将借用行克隆为拥有内容的 `'static` 行。
pub fn line_to_static(line: &Line<'_>) -> Line<'static> {
    Line {
        style: line.style,
        alignment: line.alignment,
        spans: line
            .spans
            .iter()
            .map(|s| Span {
                style: s.style,
                content: std::borrow::Cow::Owned(s.content.to_string()),
            })
            .collect(),
    }
}

/// Append owned copies of borrowed lines to `out`.
/// 把一组借用行的所有权副本追加到输出集合。
pub fn push_owned_lines<'a>(src: &[Line<'a>], out: &mut Vec<Line<'static>>) {
    for l in src {
        out.push(line_to_static(l));
    }
}

/// Consider a line blank if it has no spans or only spans whose contents are
/// empty or consist solely of spaces (no tabs/newlines).
/// 没有 span，或所有 span 仅含普通空格时视为空白行；制表符与换行不在此定义内。
#[cfg(test)]
pub fn is_blank_line_spaces_only(line: &Line<'_>) -> bool {
    if line.spans.is_empty() {
        return true;
    }
    line.spans.iter().all(|s| s.content.is_empty() || s.content.chars().all(|c| c == ' '))
}

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

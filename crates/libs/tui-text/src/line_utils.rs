//! 换行输出所需的行所有权转换。

use ratatui::text::Line;
use ratatui::text::Span;

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

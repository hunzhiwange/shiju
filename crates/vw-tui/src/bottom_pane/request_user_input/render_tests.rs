//! 验证用户选择弹窗截断文本时遵守 Unicode 字素边界，并保持 span 渲染稳定。

use super::truncate_line_word_boundary_with_ellipsis;
use pretty_assertions::assert_eq;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

#[test]
// 半角假名及其浊音记号应作为同一字素整体舍弃，省略号不能截断组合序列。
fn halfwidth_sound_marks_are_truncated_at_a_grapheme_boundary() {
    let line = Line::from("abｶﾞ tail");

    assert_eq!(
        truncate_line_word_boundary_with_ellipsis(line, /*max_width*/ 4),
        Line::from(vec![Span::raw("ab"), Span::raw("…")])
    );
}

#[test]
// 带多种样式的半角组合字截断后仍能安全绘制，并由快照固定结果结构。
fn halfwidth_sound_marks_are_truncated_and_rendered_at_a_grapheme_boundary() {
    // 用不同样式和不同半角组合字构造两行，确认截断不会破坏样式分段。
    let lines = [
        Line::from(vec!["ab".bold(), "ｶﾞ".cyan(), " tail".dim()]),
        Line::from(vec!["xy".bold(), "ﾊﾟ".magenta(), " tail".dim()]),
    ]
    .map(|line| truncate_line_word_boundary_with_ellipsis(line, /*max_width*/ 5));
    let area = Rect::new(0, 0, 5, 2);
    let mut buf = Buffer::empty(area);

    Paragraph::new(lines.to_vec()).render(area, &mut buf);

    insta::assert_snapshot!(
        "request_user_input_halfwidth_sound_marks_truncation",
        format!("{buf:?}\n{lines:#?}")
    );
}

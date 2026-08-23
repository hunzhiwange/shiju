//! 验证按终端显示宽度截断时不会拆散 Unicode 字素簇。
//! 该回归用半角假名浊音组合覆盖“多个码点、一个可见字符”的边界。

use super::line_width;
use super::truncate_line_to_width;
use pretty_assertions::assert_eq;
use ratatui::text::Line;

#[test]
// 截断结果必须保留完整组合字符，并把省略号放在合法的字素边界上。
fn halfwidth_sound_marks_stay_with_their_grapheme_when_truncated() {
    let line = Line::from("abｶﾞc");

    assert_eq!(line_width(&line), 5);
    assert_eq!(truncate_line_to_width(line.clone(), /*max_width*/ 3), Line::from("ab"));
    assert_eq!(truncate_line_to_width(line, /*max_width*/ 4), Line::from("abｶﾞ"));
}

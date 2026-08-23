//! TUI 通用坐标与动画辅助函数的测试。
//! 光标位置必须同时考虑换行和宽字符的显示列宽，呼吸颜色则应在固定周期后重复。

use super::tui_utils::{cursor_position, neon_breath_color};

#[test]
fn cursor_position_counts_lines_and_wide_chars() {
    assert_eq!(cursor_position("a你\nb", 2), (0, 3));
    assert_eq!(cursor_position("a你\nb", 4), (1, 1));
    assert_eq!(neon_breath_color(0), neon_breath_color(8));
}

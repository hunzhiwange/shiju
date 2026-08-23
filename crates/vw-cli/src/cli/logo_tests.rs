//! CLI 标识文本的最小可见性测试。
//! 确认紧凑高度下仍保留预期行数和品牌文本，避免布局裁剪导致标识消失。

use super::logo::logo_text_lines;

#[test]
fn logo_text_lines_keep_brand_text_visible() {
    let lines = logo_text_lines(1);
    assert_eq!(lines.len(), 2);
    assert!(format!("{:?}", lines[1]).contains("VibeWindow"));
}

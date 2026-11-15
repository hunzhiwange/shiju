//! CLI 暗色主题基础令牌的对比度约束测试。
//! 确保主次文字与滚动条轨道、滑块使用可区分的颜色，防止关键层级在暗色背景中消失。

use super::theme::{SCROLLBAR_THUMB, SCROLLBAR_TRACK, TEXT_MUTED, TEXT_PRIMARY};

#[test]
fn theme_tokens_keep_dark_surface_contrast_explicit() {
    assert_ne!(TEXT_PRIMARY, TEXT_MUTED);
    assert_ne!(SCROLLBAR_TRACK, SCROLLBAR_THUMB);
}

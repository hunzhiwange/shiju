//! 验证内置主题的明暗模式判定。
//! 明暗主题各保留一个明确样本，作为共享样式测试的基础假设。

use super::theme::is_dark_theme;
use iced::Theme;

#[test]
fn is_dark_theme_distinguishes_builtin_themes() {
    assert!(is_dark_theme(&Theme::Dark));
    assert!(!is_dark_theme(&Theme::Light));
}

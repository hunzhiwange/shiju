//! 验证空会话占位内容、工作区标题及明暗主题。
//! 亮度阈值两侧和恰好位于阈值的主题 fixture 用于锁定严格比较语义。

use super::{empty_session_title, is_dark_theme, login_greeting};
use iced::{Color, Theme, theme::Palette};

#[test]
fn empty_session_title_matches_workspace_mode() {
    assert_eq!(empty_session_title(false), "识矩软件智能体");
    assert_eq!(empty_session_title(true), "您的专属 AI 工作助理");
}

#[test]
fn login_greeting_uses_current_user_at_noon_boundary() {
    for hour in [0, 9, 11] {
        assert_eq!(login_greeting("刘祥敏", hour), "上午好，刘祥敏");
    }
    for hour in [12, 15, 23] {
        assert_eq!(login_greeting("李明", hour), "下午好，李明");
    }
}

#[test]
fn is_dark_theme_detects_dark_mode() {
    assert!(is_dark_theme(&Theme::Dark));
    assert!(!is_dark_theme(&Theme::Light));
}

#[test]
fn workspace_details_use_muted_text_in_both_themes() {
    for theme in [Theme::Light, Theme::Dark] {
        assert_eq!(
            super::context_text(&theme).color,
            Some(theme.palette().text.scale_alpha(0.72)),
        );
    }
}

#[test]
fn dark_theme_threshold_is_strictly_below_half_rgb_sum() {
    let at_threshold = Theme::custom(
        "threshold",
        Palette { background: Color::from_rgb(0.5, 0.5, 0.5), ..Palette::LIGHT },
    );
    let below_threshold = Theme::custom(
        "below-threshold",
        Palette { background: Color::from_rgb(0.49, 0.5, 0.5), ..Palette::LIGHT },
    );

    assert!(!is_dark_theme(&at_threshold));
    assert!(is_dark_theme(&below_threshold));
}

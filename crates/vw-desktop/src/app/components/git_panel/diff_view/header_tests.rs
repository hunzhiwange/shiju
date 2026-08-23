//! 验证差异标题栏的统计文案、主题配色与控制按钮组合。
//! 重点锁定标题清洗、颜色混合边界以及关闭和全屏控件的可选分支。

use iced::{Background, Color, Theme};

use crate::app::Message;
use crate::app::assets::Icon;
use crate::app::message;

fn background_color(background: &Option<Background>) -> Color {
    let Some(Background::Color(color)) = background else {
        panic!("expected color background");
    };
    *color
}

#[test]
fn header_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("header_tests.rs"));
}

#[test]
fn mix_color_clamps_ratio_and_interpolates_alpha() {
    let red = Color::from_rgba(1.0, 0.0, 0.0, 0.25);
    let blue = Color::from_rgba(0.0, 0.0, 1.0, 0.75);

    assert_eq!(super::header::mix_color(red, blue, -1.0), red);
    assert_eq!(super::header::mix_color(red, blue, 2.0), blue);

    let mixed = super::header::mix_color(red, blue, 0.5);
    assert_eq!(mixed, Color::from_rgba(0.5, 0.0, 0.5, 0.5));
}

#[test]
fn clean_diff_header_title_strips_active_stats_and_normalizes_whitespace() {
    assert_eq!(
        super::header::clean_diff_header_title(" src/lib.rs   (+3)  （-2） ", "+3", "-2", 3, 2),
        "src/lib.rs"
    );
    assert_eq!(
        super::header::clean_diff_header_title("src/lib.rs +4 -1", "+4", "-1", 4, 1),
        "src/lib.rs"
    );
    assert_eq!(
        super::header::clean_diff_header_title("src/lib.rs (+0) -1", "+0", "-1", 0, 1),
        "src/lib.rs (+0)"
    );
}

#[test]
fn stat_badge_and_title_styles_follow_theme_palette() {
    let light = Theme::Light;
    let palette = light.extended_palette();

    assert_eq!(
        super::header::diff_stat_badge_text_style(&light, true).color,
        Some(palette.success.base.color)
    );
    assert_eq!(
        super::header::diff_stat_badge_text_style(&light, false).color,
        Some(palette.danger.base.color)
    );
    assert_eq!(
        super::header::diff_stat_badge_container_style(&light, true).text_color,
        Some(palette.success.base.color)
    );
    assert_eq!(
        super::header::diff_stat_badge_container_style(&light, false).text_color,
        Some(palette.danger.base.color)
    );
    assert_eq!(
        super::header::diff_header_title_text_style(&light).color,
        Some(light.palette().text.scale_alpha(0.92))
    );
}

#[test]
fn diff_header_accent_selects_expected_stat_tone() {
    let light = Theme::Light;
    let palette = light.extended_palette();

    assert_eq!(super::header::diff_header_accent(&light, 0, 0), None);
    assert_eq!(super::header::diff_header_accent(&light, 1, 0), Some(palette.success.base.color));
    assert_eq!(super::header::diff_header_accent(&light, 0, 1), Some(palette.danger.base.color));
    assert_eq!(
        super::header::diff_header_accent(&light, 2, 3),
        Some(super::header::mix_color(palette.success.base.color, palette.danger.base.color, 0.5))
    );
}

#[test]
fn diff_header_container_style_covers_light_dark_and_accent_paths() {
    let light = Theme::Light;
    let dark = Theme::Dark;

    assert!(!super::header::is_dark_diff_header_theme(&light));
    assert!(super::header::is_dark_diff_header_theme(&dark));

    let light_palette = light.extended_palette();
    let light_neutral = super::header::diff_header_container_style(&light, 0, 0);
    assert_eq!(
        background_color(&light_neutral.background),
        light_palette.background.base.color.scale_alpha(0.98)
    );
    assert_eq!(light_neutral.border.color, light_palette.background.strong.color.scale_alpha(0.18));
    assert_eq!(light_neutral.border.width, 1.0);

    let light_added = super::header::diff_header_container_style(&light, 1, 0);
    assert_ne!(
        background_color(&light_added.background),
        background_color(&light_neutral.background)
    );
    assert_ne!(light_added.border.color, light_neutral.border.color);

    let dark_palette = dark.extended_palette();
    let dark_neutral = super::header::diff_header_container_style(&dark, 0, 0);
    assert_eq!(
        background_color(&dark_neutral.background),
        dark_palette.background.weak.color.scale_alpha(0.24)
    );
    assert_eq!(dark_neutral.border.color, dark_palette.background.strong.color.scale_alpha(0.30));

    let deleted = super::header::diff_header_container_style(&dark, 0, 1);
    let mixed = super::header::diff_header_container_style(&dark, 1, 1);
    assert_ne!(background_color(&deleted.background), background_color(&dark_neutral.background));
    assert_ne!(background_color(&mixed.background), background_color(&deleted.background));
}

#[test]
fn build_diff_header_covers_stats_close_and_fullscreen_combinations() {
    let close = Message::Git(message::GitMessage::CloseDiffFileMenu);
    let fullscreen = Message::Git(message::GitMessage::ToggleFullscreen);

    let _with_all_controls = super::header::build_diff_header(
        "src/lib.rs (+2 -1)".to_string(),
        2,
        1,
        Some(close.clone()),
        Some(fullscreen.clone()),
        Some("全屏".to_string()),
        Some(Icon::ArrowsFullscreen),
    );

    let _without_stats_or_close =
        super::header::build_diff_header("src/lib.rs".to_string(), 0, 0, None, None, None, None);

    let _partial_fullscreen_args_are_ignored = super::header::build_diff_header(
        "src/lib.rs （+3）".to_string(),
        3,
        0,
        None,
        Some(fullscreen),
        None,
        Some(Icon::ArrowsFullscreen),
    );
}

#[test]
fn build_diff_header_without_close_accepts_complete_fullscreen_control() {
    let fullscreen = Message::Git(message::GitMessage::ToggleFullscreen);

    let _header = super::header::build_diff_header(
        "src/lib.rs".to_string(),
        0,
        0,
        None,
        Some(fullscreen),
        Some("全屏".to_string()),
        Some(Icon::ArrowsFullscreen),
    );
}

#[test]
fn wrap_diff_header_covers_neutral_added_deleted_and_mixed_styles() {
    for (insertions, deletions) in [(0, 0), (1, 0), (0, 1), (2, 3)] {
        let header = super::header::build_diff_header(
            "src/lib.rs".to_string(),
            insertions,
            deletions,
            None,
            None,
            None,
            None,
        );
        let _wrapped = super::header::wrap_diff_header(header, insertions, deletions);
    }
}

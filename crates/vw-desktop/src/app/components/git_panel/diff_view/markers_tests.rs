//! 验证差异行标记、行号色调及明暗主题下的透明度计算。
//! 测试逐一覆盖新增、删除、中性标记和强调状态，避免主题分支退化。

use iced::{Color, Theme};

#[test]
fn marker_alpha_covers_all_marker_kinds_and_theme_modes() {
    assert_eq!(
        super::markers::emphasized_marker_alpha(super::markers::LineMarkerKind::None, false, true),
        0.0
    );
    assert_eq!(
        super::markers::emphasized_marker_alpha(super::markers::LineMarkerKind::None, true, true),
        0.34
    );
    assert_eq!(
        super::markers::emphasized_marker_alpha(super::markers::LineMarkerKind::Add, true, false),
        0.76
    );
    assert_eq!(
        super::markers::emphasized_marker_alpha(
            super::markers::LineMarkerKind::Delete,
            false,
            true
        ),
        0.66
    );
    assert_eq!(
        super::markers::emphasized_marker_alpha(
            super::markers::LineMarkerKind::Delete,
            false,
            false
        ),
        0.58
    );
}

#[test]
fn marker_mix_color_clamps_and_interpolates() {
    let a = Color::from_rgba(0.0, 0.0, 0.0, 0.2);
    let b = Color::from_rgba(1.0, 1.0, 1.0, 0.8);

    assert_eq!(super::markers::mix_color(a, b, -0.5), a);
    assert_eq!(super::markers::mix_color(a, b, 1.5), b);
    assert_eq!(super::markers::mix_color(a, b, 0.5), Color::from_rgba(0.5, 0.5, 0.5, 0.5));
}

#[test]
fn line_number_padding_and_cells_cover_tones() {
    assert_eq!(
        super::markers::line_number_right_padding(super::markers::LineNumberTone::Neutral),
        6.0
    );
    assert_eq!(super::markers::line_number_right_padding(super::markers::LineNumberTone::Add), 7.0);
    assert_eq!(
        super::markers::line_number_right_padding(super::markers::LineNumberTone::Delete),
        7.0
    );

    let _neutral = super::markers::line_number_cell("1".to_string());
    let _add = super::markers::line_number_cell_with_tone(
        "2".to_string(),
        super::markers::LineNumberTone::Add,
    );
    let _delete = super::markers::line_number_cell_with_tone_offset(
        "3".to_string(),
        super::markers::LineNumberTone::Delete,
        3.0,
    );
    let _empty = super::markers::empty_line_number_cell();
}

#[test]
fn line_marker_cells_cover_add_delete_none_and_emphasis() {
    for kind in [
        super::markers::LineMarkerKind::None,
        super::markers::LineMarkerKind::Add,
        super::markers::LineMarkerKind::Delete,
    ] {
        let _plain = super::markers::line_marker_cell_emphasis(kind, false);
        let _emphasized = super::markers::line_marker_cell_emphasis(kind, true);
    }
}

#[test]
fn marker_alpha_remaining_branches() {
    // (None, true, false) → is_dark=false branch of LineMarkerKind::None if emphasized
    assert_eq!(
        super::markers::emphasized_marker_alpha(super::markers::LineMarkerKind::None, true, false),
        0.28
    );
    // (Add, true, true) → is_dark=true branch of _ if emphasized
    assert_eq!(
        super::markers::emphasized_marker_alpha(super::markers::LineMarkerKind::Add, true, true),
        0.82
    );
    // (None, false, false) → LineMarkerKind::None arm (not emphasized, is_dark=false)
    assert_eq!(
        super::markers::emphasized_marker_alpha(super::markers::LineMarkerKind::None, false, false),
        0.0
    );
}

#[test]
fn non_emphasized_add_marker_uses_theme_specific_alpha() {
    assert_eq!(
        super::markers::emphasized_marker_alpha(super::markers::LineMarkerKind::Add, false, true,),
        0.66
    );
    assert_eq!(
        super::markers::emphasized_marker_alpha(super::markers::LineMarkerKind::Add, false, false,),
        0.58
    );
}

#[test]
fn mix_color_boundaries() {
    let a = Color::from_rgba(0.2, 0.4, 0.6, 0.8);
    let b = Color::from_rgba(0.8, 0.6, 0.4, 0.2);

    // t exactly 0.0 returns a unchanged (not clamped)
    assert_eq!(super::markers::mix_color(a, b, 0.0), a);
    // t exactly 1.0 returns b unchanged (not clamped)
    assert_eq!(super::markers::mix_color(a, b, 1.0), b);
}

#[test]
fn is_dark_theme_light_vs_dark() {
    assert!(!super::markers::is_dark_theme(&Theme::Light));
    assert!(super::markers::is_dark_theme(&Theme::Dark));
}

#[test]
fn line_number_text_color_all_tones_and_themes() {
    for tone in [
        super::markers::LineNumberTone::Neutral,
        super::markers::LineNumberTone::Add,
        super::markers::LineNumberTone::Delete,
    ] {
        let light = super::markers::line_number_text_color(&Theme::Light, tone);
        assert!(light.a > 0.0, "light {tone:?} alpha should be > 0");
        let dark = super::markers::line_number_text_color(&Theme::Dark, tone);
        assert!(dark.a > 0.0, "dark {tone:?} alpha should be > 0");
    }
}

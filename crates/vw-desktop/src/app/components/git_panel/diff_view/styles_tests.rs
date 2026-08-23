//! 验证差异窗格、行号区域与交互状态在明暗主题下的样式契约。
//! 对精确背景色、边框和混色边界做断言，元素构建仅用于覆盖布局分支。

use iced::widget::{container, text};
use iced::{Background, Color, Element, Theme};

use crate::app::Message;

#[test]
fn task_715_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("styles_tests.rs"));
}

fn app() -> crate::app::App {
    crate::app::App::new().0
}

fn content(label: &'static str) -> Element<'static, Message> {
    container(text(label)).into()
}

#[test]
fn diff_highlight_is_enabled_for_default_app() {
    let app = app();

    assert!(super::styles::diff_highlight_enabled(&app));
}

#[test]
fn row_and_pane_builders_accept_all_tones_and_emphasis_states() {
    for tone in [
        super::DiffSplitPaneTone::Neutral,
        super::DiffSplitPaneTone::Empty,
        super::DiffSplitPaneTone::Add,
        super::DiffSplitPaneTone::Delete,
    ] {
        let _: Element<'static, Message> =
            super::styles::merge_diff_row(content("row"), tone, false);
        let _: Element<'static, Message> =
            super::styles::merge_diff_row(content("row"), tone, true);
        let _: Element<'static, Message> =
            super::styles::diff_split_pane(content("pane"), tone, false);
        let _: Element<'static, Message> =
            super::styles::diff_split_pane(content("pane"), tone, true);
    }
}

#[test]
fn explicit_background_builders_cover_plain_and_emphasized_paths() {
    let bg = Color::from_rgba8(12, 34, 56, 0.5);

    let _: Element<'static, Message> =
        super::styles::merge_diff_row_with_background(content("row"), bg, false);
    let _: Element<'static, Message> =
        super::styles::merge_diff_row_with_background(content("row"), bg, true);
    let _: Element<'static, Message> =
        super::styles::diff_split_pane_with_background(content("pane"), bg, false);
    let _: Element<'static, Message> =
        super::styles::diff_split_pane_with_background(content("pane"), bg, true);
    let _: Element<'static, Message> =
        super::styles::diff_line_number_with_background(content("line"), bg);
}

#[test]
fn divider_and_line_number_area_build_empty_and_interactive_variants() {
    let _: Element<'static, Message> = super::styles::diff_split_divider();
    let _: Element<'static, Message> = super::styles::split_line_number_area(
        "src/lib.rs",
        None,
        "",
        super::markers::LineNumberTone::Neutral,
    );
    let _: Element<'static, Message> = super::styles::split_line_number_area(
        "src/lib.rs",
        Some((0, false)),
        "added",
        super::markers::LineNumberTone::Add,
    );
    let _: Element<'static, Message> = super::styles::split_line_number_area(
        "src/lib.rs",
        Some((3, true)),
        "deleted",
        super::markers::LineNumberTone::Delete,
    );
}

#[test]
fn theme_detection_and_blending_cover_boundaries() {
    assert!(!super::styles::is_dark_theme(&Theme::Light));
    assert!(super::styles::is_dark_theme(&Theme::Dark));

    let start = Color::from_rgba(0.1, 0.2, 0.3, 0.4);
    let end = Color::from_rgba(0.9, 0.8, 0.7, 0.6);

    assert_color_close(super::styles::blend_color(start, end, -1.0), start);
    assert_color_close(super::styles::blend_color(start, end, 2.0), end);
    assert_color_close(
        super::styles::blend_color(start, end, 0.25),
        Color::from_rgba(0.3, 0.35000002, 0.4, 0.45000002),
    );
}

#[test]
fn pane_backgrounds_cover_all_tones_for_light_and_dark_themes() {
    for theme in [Theme::Light, Theme::Dark] {
        for tone in [
            super::DiffSplitPaneTone::Neutral,
            super::DiffSplitPaneTone::Empty,
            super::DiffSplitPaneTone::Add,
            super::DiffSplitPaneTone::Delete,
        ] {
            let plain = super::styles::diff_pane_background(&theme, tone, false);
            let emphasized = super::styles::diff_pane_background(&theme, tone, true);
            let base = super::styles::diff_pane_base_background(&theme, tone);

            assert_color_close(plain, base);
            assert_ne!(plain, emphasized);
            assert!(plain.a >= 0.0);
            assert!(emphasized.a >= 0.0);
        }
    }
}

#[test]
fn pane_base_backgrounds_use_exact_theme_tones() {
    let light = Theme::Light;
    let light_palette = light.extended_palette();
    assert_color_close(
        super::styles::diff_pane_base_background(&light, super::DiffSplitPaneTone::Neutral),
        light_palette.background.base.color.scale_alpha(0.96),
    );
    assert_color_close(
        super::styles::diff_pane_base_background(&light, super::DiffSplitPaneTone::Empty),
        light_palette.background.weak.color.scale_alpha(0.36),
    );
    assert_color_close(
        super::styles::diff_pane_base_background(&light, super::DiffSplitPaneTone::Add),
        light_palette.success.base.color.scale_alpha(0.24),
    );
    assert_color_close(
        super::styles::diff_pane_base_background(&light, super::DiffSplitPaneTone::Delete),
        light_palette.danger.base.color.scale_alpha(0.14),
    );

    let dark = Theme::Dark;
    let dark_palette = dark.extended_palette();
    assert_color_close(
        super::styles::diff_pane_base_background(&dark, super::DiffSplitPaneTone::Neutral),
        dark_palette.background.base.color.scale_alpha(0.08),
    );
    assert_color_close(
        super::styles::diff_pane_base_background(&dark, super::DiffSplitPaneTone::Empty),
        dark_palette.background.weak.color.scale_alpha(0.08),
    );
    assert_color_close(
        super::styles::diff_pane_base_background(&dark, super::DiffSplitPaneTone::Add),
        dark_palette.success.base.color.scale_alpha(0.34),
    );
    assert_color_close(
        super::styles::diff_pane_base_background(&dark, super::DiffSplitPaneTone::Delete),
        dark_palette.danger.base.color.scale_alpha(0.20),
    );
}

#[test]
fn container_style_helpers_return_expected_backgrounds_and_borders() {
    let custom = Color::from_rgba8(12, 34, 56, 0.5);

    for theme in [Theme::Light, Theme::Dark] {
        for tone in [
            super::DiffSplitPaneTone::Neutral,
            super::DiffSplitPaneTone::Empty,
            super::DiffSplitPaneTone::Add,
            super::DiffSplitPaneTone::Delete,
        ] {
            let row = super::styles::merge_diff_row_style(&theme, tone, false);
            assert_eq!(row.border.width, 0.0);
            assert_color_close(
                background_color(row.background),
                super::styles::diff_pane_background(&theme, tone, false),
            );

            let pane = super::styles::diff_split_pane_style(&theme, tone, true);
            assert_eq!(pane.border.width, 0.0);
            assert_eq!(pane.border.color.a, 0.0);
            assert_color_close(
                background_color(pane.background),
                super::styles::diff_pane_background(&theme, tone, true),
            );
        }

        let plain_row = super::styles::merge_diff_row_background_style(&theme, custom, false);
        let emphasized_row = super::styles::merge_diff_row_background_style(&theme, custom, true);
        assert_color_close(background_color(plain_row.background), custom);
        assert_ne!(
            background_color(plain_row.background),
            background_color(emphasized_row.background)
        );

        let plain_pane = super::styles::diff_split_pane_background_style(&theme, custom, false);
        let emphasized_pane = super::styles::diff_split_pane_background_style(&theme, custom, true);
        assert_color_close(background_color(plain_pane.background), custom);
        assert_ne!(
            background_color(plain_pane.background),
            background_color(emphasized_pane.background)
        );

        let divider = super::styles::diff_split_divider_style(&theme);
        assert!(background_color(divider.background).a > 0.0);
    }
}

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}

fn assert_color_close(actual: Color, expected: Color) {
    assert!((actual.r - expected.r).abs() < 0.0001, "red: {actual:?} != {expected:?}");
    assert!((actual.g - expected.g).abs() < 0.0001, "green: {actual:?} != {expected:?}");
    assert!((actual.b - expected.b).abs() < 0.0001, "blue: {actual:?} != {expected:?}");
    assert!((actual.a - expected.a).abs() < 0.0001, "alpha: {actual:?} != {expected:?}");
}

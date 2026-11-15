use super::*;
use super::super::controls::{fullscreen_icon_style, fullscreen_tooltip_style, overlay_icon_button_style};
use iced::{Background, Color, Theme};
use iced::widget::button;
#[test]
fn overlay_icon_button_style_distinguishes_statuses() {
    let theme = Theme::Light;
    let active = overlay_icon_button_style(&theme, button::Status::Active);
    let hovered = overlay_icon_button_style(&theme, button::Status::Hovered);
    let pressed = overlay_icon_button_style(&theme, button::Status::Pressed);

    assert!(active.background.is_none());
    assert_eq!(active.border.color, Color::TRANSPARENT);
    assert!(matches!(hovered.background, Some(Background::Color(_))));
    assert!(matches!(pressed.background, Some(Background::Color(_))));
    assert_ne!(hovered.border.color, pressed.border.color);
}

#[test]
fn fullscreen_tooltip_style_switches_between_light_and_dark_palettes() {
    let light = fullscreen_tooltip_style(&Theme::Light);
    let dark = fullscreen_tooltip_style(&Theme::Dark);

    assert_eq!(light.text_color, Some(Color::WHITE));
    assert_ne!(light.background, dark.background);
    assert!(light.shadow.blur_radius > dark.shadow.blur_radius);
}

#[test]
fn fullscreen_icon_style_uses_visible_theme_color() {
    let light = fullscreen_icon_style(&Theme::Light);
    let dark = fullscreen_icon_style(&Theme::Dark);

    assert!(light.color.is_some());
    assert!(dark.color.is_some());
    assert_ne!(light.color, dark.color);
}

#[test]
fn preview_breadcrumb_segments_prefers_project_relative_segments_and_falls_back_to_path() {
    let relative = preview_breadcrumb_segments(
        std::path::Path::new("/tmp/project/src/main.rs"),
        Some("/tmp/project"),
    );
    assert_eq!(relative, vec!["src".to_string(), "main.rs".to_string()]);

    let root =
        preview_breadcrumb_segments(std::path::Path::new("/tmp/project"), Some("/tmp/project"));
    assert_eq!(root, vec!["/tmp/project".to_string()]);

    let unrelated = preview_breadcrumb_segments(
        std::path::Path::new("/tmp/other/main.rs"),
        Some("/tmp/project"),
    );
    assert_eq!(unrelated, vec!["/tmp/other/main.rs".to_string()]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn active_preview_lsp_badge_state_and_colors_cover_all_tones() {
    assert_eq!(
        super::active_preview_lsp_badge_state(Some("rust-analyzer"), false, true),
        ("LSP", super::BadgeTone::Ready)
    );
    assert_eq!(
        super::active_preview_lsp_badge_state(Some("rust-analyzer"), true, true),
        ("LSP", super::BadgeTone::Working)
    );
    assert_eq!(
        super::active_preview_lsp_badge_state(None, false, true),
        ("LSP", super::BadgeTone::Error)
    );
    assert_eq!(
        super::active_preview_lsp_badge_state(None, false, false),
        ("Plain Text", super::BadgeTone::Neutral)
    );

    for tone in [
        super::BadgeTone::Ready,
        super::BadgeTone::Working,
        super::BadgeTone::Error,
        super::BadgeTone::Neutral,
    ] {
        let dot = super::active_preview_lsp_badge_dot_color(&Theme::Dark, tone);
        let (background, text, border) = super::active_preview_lsp_badge_colors(&Theme::Dark, tone);
        assert_ne!(dot, Color::TRANSPARENT);
        assert_ne!(background, Color::TRANSPARENT);
        assert_ne!(text, Color::TRANSPARENT);
        assert_ne!(border, Color::TRANSPARENT);
    }
}

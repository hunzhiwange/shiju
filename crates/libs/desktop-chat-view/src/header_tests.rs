use super::header::*;
use iced::{Background, Color, Theme};
use iced::widget::button;
fn background_color(background: Option<Background>) -> Color { match background { Some(Background::Color(color)) => color, _ => panic!("expected color background") } }

#[test]
fn task_475_test_module_is_wired() {
    let path = std::path::Path::new(file!());

    assert!(path.ends_with("header_tests.rs"));
}

#[test]
fn workspace_badge_styles_cover_theme_and_primary_variants() {
    for theme in [Theme::Light, Theme::Dark] {
        for is_primary in [true, false] {
            let style = session_workspace_badge_style(&theme, is_primary);

            assert!(style.background.is_some());
            assert_eq!(style.border.width, 1.0);
            assert_eq!(style.border.radius.top_left, 999.0);
            assert!(style.text_color.is_some());
        }
    }

    let primary = background_color(session_workspace_badge_style(&Theme::Light, true).background);
    let secondary =
        background_color(session_workspace_badge_style(&Theme::Light, false).background);

    assert_ne!(primary, secondary);
}

#[test]
fn return_to_parent_button_style_uses_readable_dark_theme_text() {
    let active = return_to_parent_button_style(&Theme::Dark, button::Status::Active);
    let hovered = return_to_parent_button_style(&Theme::Dark, button::Status::Hovered);
    let pressed = return_to_parent_button_style(&Theme::Dark, button::Status::Pressed);

    assert_eq!(active.text_color, Theme::Dark.palette().text.scale_alpha(0.92));
    assert_eq!(hovered.text_color, Theme::Dark.palette().text.scale_alpha(0.96));
    assert_eq!(pressed.text_color, Theme::Dark.palette().text.scale_alpha(0.96));
}

#[test]
fn status_badge_styles_use_expected_surfaces() {
    let running = running_status_badge_style(Color::from_rgb8(239, 68, 68));
    let queued = queued_status_badge_style();
    let text_style = colored_text_style(Color::from_rgb8(46, 184, 114));

    assert!(running.background.is_some());
    assert!(queued.background.is_some());
    assert_eq!(running.border.radius.top_left, 999.0);
    assert_eq!(queued.text_color, Some(Color::from_rgb8(245, 158, 11)));
    assert_eq!(text_style.color, Some(Color::from_rgb8(46, 184, 114)));
}

#[test]
fn header_surface_style_is_transparent() {
    let style = header_surface_style(&Theme::Dark, None);

    assert!(style.background.is_none());
    assert_eq!(style.border.width, 0.0);
    assert_eq!(style.border.color, Color::TRANSPARENT);
}

#[test]
fn session_actions_button_style_covers_all_status_and_theme_paths() {
    for theme in [Theme::Light, Theme::Dark] {
        for status in [
            button::Status::Active,
            button::Status::Hovered,
            button::Status::Pressed,
            button::Status::Disabled,
        ] {
            let style = session_actions_button_style(&theme, status);

            assert!(style.background.is_some());
            assert_eq!(style.border.width, 1.0);
            assert_eq!(style.border.radius.top_left, 8.0);
            assert_eq!(style.text_color, theme.palette().text.scale_alpha(0.92));
        }
    }

    let active = background_color(
        session_actions_button_style(&Theme::Light, button::Status::Active).background,
    );
    let hovered = background_color(
        session_actions_button_style(&Theme::Light, button::Status::Hovered).background,
    );
    let pressed_dark = background_color(
        session_actions_button_style(&Theme::Dark, button::Status::Pressed).background,
    );

    assert_eq!(active, Color::TRANSPARENT);
    assert_ne!(hovered, Color::TRANSPARENT);
    assert_ne!(pressed_dark, Color::TRANSPARENT);
}

#[test]
fn session_actions_popover_style_covers_light_and_dark_surfaces() {
    let light = session_actions_popover_style(&Theme::Light);
    let dark = session_actions_popover_style(&Theme::Dark);

    assert!(light.background.is_some());
    assert!(dark.background.is_some());
    assert_eq!(light.border.radius.top_left, 12.0);
    assert_eq!(dark.border.radius.top_left, 12.0);
    assert_ne!(background_color(light.background), background_color(dark.background));
    assert!(dark.shadow.blur_radius > 0.0);
}

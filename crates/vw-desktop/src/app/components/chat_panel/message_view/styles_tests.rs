use super::styles::{
    COMPACT_ACTION_BUTTON_RADIUS, MESSAGE_META_TEXT_SIZE, MESSAGE_TEXT_SIZE, THINK_META_TEXT_SIZE,
    THINK_STATUS_TEXT_SIZE, is_dark_theme, message_body_text_color, message_meta_text_color,
    message_text_line_height, subtle_card_shadow, think_block_text_color, thinking_status_text,
};
use iced::{Color, Theme, theme::Palette};

#[test]
fn dark_theme_detection_tracks_palette_background() {
    assert!(is_dark_theme(&Theme::Dark));
    assert!(!is_dark_theme(&Theme::Light));
}

#[test]
fn message_colors_are_distinct_for_user_and_assistant() {
    assert_ne!(
        message_body_text_color(&Theme::Light, true),
        message_body_text_color(&Theme::Light, false)
    );
    assert_ne!(
        message_meta_text_color(&Theme::Dark, true),
        message_meta_text_color(&Theme::Dark, false)
    );
}

#[test]
fn subtle_shadow_is_stronger_in_dark_theme() {
    let light = subtle_card_shadow(&Theme::Light);
    let dark = subtle_card_shadow(&Theme::Dark);

    assert!(dark.color.a > light.color.a);
    assert_eq!(dark.offset, light.offset);
    assert_eq!(dark.blur_radius, light.blur_radius);
}

#[test]
fn thinking_status_text_builds_for_empty_label() {
    let _ = thinking_status_text("", 1234, 2);
    assert_ne!(think_block_text_color(&Theme::Light), think_block_text_color(&Theme::Dark));
}

#[test]
fn message_text_line_height_returns_expected_value() {
    assert_eq!(message_text_line_height(), iced::widget::text::LineHeight::Relative(1.62));
}

#[test]
fn style_constants_have_expected_values() {
    assert_eq!(MESSAGE_TEXT_SIZE, 15.0);
    assert_eq!(MESSAGE_META_TEXT_SIZE, 13.0);
    assert_eq!(THINK_STATUS_TEXT_SIZE, 13.0);
    assert_eq!(THINK_META_TEXT_SIZE, 12.0);
    assert_eq!(COMPACT_ACTION_BUTTON_RADIUS, 5.0);
}

#[test]
fn message_body_text_color_alpha_is_correct_per_branch() {
    // Light theme, user: scale_alpha(0.90)
    assert!((message_body_text_color(&Theme::Light, true).a - 0.90).abs() < 1e-6);
    // Light theme, assistant: scale_alpha(0.88)
    assert!((message_body_text_color(&Theme::Light, false).a - 0.88).abs() < 1e-6);
    // Dark theme, user: scale_alpha(0.96)
    assert!((message_body_text_color(&Theme::Dark, true).a - 0.96).abs() < 1e-6);
    // Dark theme, assistant: scale_alpha(0.91)
    assert!((message_body_text_color(&Theme::Dark, false).a - 0.91).abs() < 1e-6);
}

#[test]
fn message_meta_text_color_alpha_is_correct_per_branch() {
    // Dark theme, user: text.scale_alpha(0.50)
    assert!((message_meta_text_color(&Theme::Dark, true).a - 0.50).abs() < 1e-6);
    // Dark theme, assistant: scale_alpha(0.46)
    assert!((message_meta_text_color(&Theme::Dark, false).a - 0.46).abs() < 1e-6);
    // Light theme, assistant: scale_alpha(0.48)
    assert!((message_meta_text_color(&Theme::Light, false).a - 0.48).abs() < 1e-6);
    // Light theme, user: extended_palette secondary base text.scale_alpha(0.72)
    let light_user = message_meta_text_color(&Theme::Light, true);
    let expected = Theme::Light.extended_palette().secondary.base.text.scale_alpha(0.72);
    assert!((light_user.a - expected.a).abs() < 1e-6);
    assert_eq!(light_user.r, expected.r);
    assert_eq!(light_user.g, expected.g);
    assert_eq!(light_user.b, expected.b);
}

#[test]
fn think_block_text_color_is_valid_in_both_themes() {
    assert!(think_block_text_color(&Theme::Light).a > 0.0);
    assert!(think_block_text_color(&Theme::Dark).a > 0.0);
}

#[test]
fn thinking_status_text_builds_for_non_empty_label() {
    let _ = thinking_status_text("thinking", 9999, 0);
    let _ = thinking_status_text("思考中", 9999, 1);
}

#[test]
fn dark_theme_detection_treats_exact_threshold_as_light() {
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

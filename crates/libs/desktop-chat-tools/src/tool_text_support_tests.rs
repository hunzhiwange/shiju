use super::tool_text_support::*;
use iced::widget::text_editor;
use iced::{Color, Theme};

#[test]
fn tool_text_key_packs_indices_without_collisions_for_adjacent_values() {
    assert_ne!(tool_text_key(1, 2, 3), tool_text_key(1, 2, 4));
    assert_ne!(tool_text_key(1, 2, 3), tool_text_key(1, 3, 3));
    assert_eq!(tool_text_key(0, 0, 0), 0);
}

#[test]
fn text_editor_safety_rejects_oversized_lines() {
    assert!(is_safe_for_text_editor("short\ntext"));
    assert!(!is_safe_for_text_editor(&"x".repeat(2_001)));
    assert!(!is_safe_for_text_editor(&"x".repeat(20_001)));
    assert!(is_safe_for_text_editor(""));
}

#[test]
fn chat_text_font_name_is_configured() {
    assert!(!chat_text_font_name().is_empty());
    let _ = chat_text_font();
    match chat_text_line_height() {
        iced::widget::text::LineHeight::Relative(value) => assert_eq!(value, 1.52),
        _ => panic!("chat text line height should be relative"),
    }
}

#[test]
fn content_safe_font_keeps_ascii_monospace_and_falls_back_for_non_ascii() {
    let preferred = "JetBrains Mono";

    assert_eq!(content_safe_font("fn main() {}", preferred), iced::Font::with_name(preferred));
    assert_eq!(content_safe_font("// 中文注释", preferred), chat_text_font());
}

#[test]
fn text_styles_use_theme_selection_and_danger_color() {
    let theme = Theme::Dark;
    let selection = chat_text_selection_color(&theme);
    assert!(selection.a > 0.0);

    let readonly = read_only_text_style(&theme, Color::from_rgb8(1, 2, 3));
    assert_eq!(readonly.value, Color::from_rgb8(1, 2, 3));
    assert_eq!(readonly.border.width, 0.0);

    let normal = tool_text_style(&theme, text_editor::Status::Active);
    let danger = tool_text_style_with_danger(&theme, text_editor::Status::Active, true);
    assert_ne!(normal.value, danger.value);
}

#[test]
fn message_target_from_context_decodes_bit_packed_target() {
    assert_eq!(message_target_from_context(0), (0, None));
    let t = (7usize as u64) << 32;
    assert_eq!(message_target_from_context(t), (7, None));
    let t = ((7usize as u64) << 32) | 3;
    assert_eq!(message_target_from_context(t), (7, Some(2)));
    let t = ((7usize as u64) << 32) | 1;
    assert_eq!(message_target_from_context(t), (7, Some(0)));
    let t = ((7usize as u64) << 32) | 0xFFFF_FFFF;
    assert_eq!(message_target_from_context(t), (7, Some(0xFFFF_FFFE)));
}

#[test]
fn chat_text_selection_color_differs_between_dark_and_light_themes() {
    let dark_color = chat_text_selection_color(&Theme::Dark);
    let light_color = chat_text_selection_color(&Theme::Light);
    assert_ne!(dark_color, light_color);
    assert!(dark_color.a > 0.0);
    assert!(light_color.a > 0.0);
}

#[test]
fn tool_text_style_without_danger_matches_tool_text_style() {
    let theme = Theme::Dark;
    let default = tool_text_style(&theme, text_editor::Status::Active);
    let no_danger = tool_text_style_with_danger(&theme, text_editor::Status::Active, false);
    assert_eq!(default.value, no_danger.value);
}

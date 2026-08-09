use super::ui::{
    bold_font, capped_scroll_height, chat_context_menu, chat_context_menu_button_style,
    chat_context_menu_style, chat_context_target_key, copy_tooltip_content, copy_tooltip_style,
    wrap_chat_context_menu,
};
use iced::widget::button;
use iced::{Background, Color, Length, Theme};

#[test]
fn chat_context_target_key_packs_optional_tool_index() {
    assert_eq!(chat_context_target_key(0, Some(0)), 1);
    assert_eq!(chat_context_target_key(3, None), 3_u64 << 32);
    assert_eq!(chat_context_target_key(3, Some(4)), (3_u64 << 32) | 5);
}

#[test]
fn capped_scroll_height_keeps_reasonable_bounds() {
    assert_eq!(capped_scroll_height("", 35.0), Length::Shrink);
    assert_eq!(capped_scroll_height("short text", 120.0), Length::Shrink);
    assert_eq!(capped_scroll_height("short text", 34.0), Length::Fixed(34.0));
    assert_eq!(capped_scroll_height("one\ntwo", 52.0), Length::Fixed(52.0));
    assert_eq!(capped_scroll_height("one\ntwo", 53.0), Length::Shrink);
    assert_eq!(capped_scroll_height(&"line\n".repeat(100), 120.0), Length::Fixed(120.0));
}

#[test]
fn bold_font_is_configured() {
    assert_eq!(bold_font().weight, iced::font::Weight::Bold);
}

#[test]
fn menu_and_tooltip_build_only_when_requested() {
    assert!(chat_context_menu(false).is_none());
    assert!(chat_context_menu(true).is_some());
    let _ = copy_tooltip_content("复制");
}

#[test]
fn context_menu_wrapper_keeps_a_stable_content_shell_when_toggled() {
    let _closed = wrap_chat_context_menu(iced::widget::text("chat"), false, (12.0, 26.0), true);
    let _open = wrap_chat_context_menu(iced::widget::text("chat"), true, (12.0, 26.0), true);
}

#[test]
fn tooltip_style_uses_fixed_dark_palette() {
    let style = copy_tooltip_style(&Theme::Light);

    assert_eq!(style.text_color, Some(Color::from_rgb8(0xF3, 0xF4, 0xF6)));
    assert_eq!(background_color(style.background), Color::from_rgb8(0x0B, 0x0B, 0x0B));
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.color, Color::from_rgb8(0x14, 0x14, 0x14));
    assert_eq!(style.shadow.color, Color::BLACK.scale_alpha(0.25));
    assert_eq!(style.shadow.offset, iced::Vector::new(0.0, 2.0));
    assert_eq!(style.shadow.blur_radius, 6.0);
}

#[test]
fn context_menu_button_style_covers_theme_and_state_branches() {
    let light_active = chat_context_menu_button_style(&Theme::Light, button::Status::Active);
    let light_hovered = chat_context_menu_button_style(&Theme::Light, button::Status::Hovered);
    let light_pressed = chat_context_menu_button_style(&Theme::Light, button::Status::Pressed);
    let dark_hovered = chat_context_menu_button_style(&Theme::Dark, button::Status::Hovered);
    let dark_pressed = chat_context_menu_button_style(&Theme::Dark, button::Status::Pressed);

    assert_eq!(light_active.background, None);
    assert_eq!(
        background_color(light_hovered.background),
        Color::from_rgba8(0xEA, 0xED, 0xF1, 0.85)
    );
    assert_eq!(
        background_color(light_pressed.background),
        Color::from_rgba8(0xE2, 0xE6, 0xEB, 0.95)
    );
    assert_eq!(
        background_color(dark_hovered.background),
        Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.05)
    );
    assert_eq!(
        background_color(dark_pressed.background),
        Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.09)
    );
    assert_eq!(light_active.text_color, Theme::Light.palette().text);
    assert_eq!(dark_pressed.text_color, Theme::Dark.palette().text);
}

#[test]
fn context_menu_container_style_covers_light_and_dark_palettes() {
    let light = chat_context_menu_style(&Theme::Light);
    let dark = chat_context_menu_style(&Theme::Dark);

    assert_eq!(background_color(light.background), Color::from_rgba8(0xF3, 0xF4, 0xF6, 0.98));
    assert_eq!(background_color(dark.background), Color::from_rgba8(0x22, 0x22, 0x24, 0.98));
    assert_eq!(light.border.color, Color::from_rgba8(0xDD, 0xE1, 0xE6, 1.0));
    assert_eq!(dark.border.color, Color::from_rgba8(0x3A, 0x3A, 0x3F, 0.95));
    assert_eq!(light.shadow.color, Color::BLACK.scale_alpha(0.08));
    assert_eq!(dark.shadow.color, Color::BLACK.scale_alpha(0.20));
    assert_eq!(dark.shadow.offset, iced::Vector::new(0.0, 4.0));
    assert_eq!(dark.shadow.blur_radius, 10.0);
}

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}

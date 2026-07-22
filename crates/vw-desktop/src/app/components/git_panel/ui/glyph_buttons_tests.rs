use iced::widget::button;
use iced::{Background, Color, Element, Length, Size, Theme};

use crate::app::Message;

use super::glyph_buttons::{
    header_plain_glyph_button, medium_glyph_button, medium_glyph_button_style,
    medium_plain_glyph_button, medium_plain_glyph_button_style, small_glyph_button,
    small_glyph_button_style, small_plain_glyph_button, small_plain_glyph_button_style,
};

fn radius(style: &button::Style) -> f32 {
    style.border.radius.top_left
}

#[test]
fn tooltip_glyph_buttons_wrap_button_and_tip_content() {
    let medium: Element<'static, Message> =
        medium_glyph_button("M", "medium glyph".into(), Message::None);
    let small: Element<'static, Message> =
        small_glyph_button("S", "small glyph".into(), Message::None);
    let header: Element<'static, Message> =
        header_plain_glyph_button("H", "header glyph".into(), Message::None);

    for element in [&medium, &small, &header] {
        assert_eq!(element.as_widget().children().len(), 2);
        assert_eq!(element.as_widget().size(), Size::new(Length::Shrink, Length::Shrink));
    }
}

#[test]
fn plain_glyph_buttons_keep_single_glyph_child() {
    let medium: Element<'static, Message> =
        medium_plain_glyph_button("M", "ignored".into(), Message::None);
    let small: Element<'static, Message> =
        small_plain_glyph_button("S", "ignored".into(), Message::None);

    for element in [&medium, &small] {
        assert_eq!(element.as_widget().children().len(), 1);
        assert_eq!(element.as_widget().size(), Size::new(Length::Shrink, Length::Shrink));
    }
}

#[test]
fn compact_glyph_button_styles_use_size_specific_feedback_radii() {
    let medium_active = medium_glyph_button_style(button::Status::Active);
    let medium_hovered = medium_glyph_button_style(button::Status::Hovered);
    let small_active = small_glyph_button_style(button::Status::Active);
    let small_pressed = small_glyph_button_style(button::Status::Pressed);

    assert_eq!(medium_active.background, Some(Background::Color(Color::TRANSPARENT)));
    assert_ne!(medium_hovered.background, medium_active.background);
    assert_eq!(radius(&medium_hovered), 8.0);
    assert_eq!(small_active.background, Some(Background::Color(Color::TRANSPARENT)));
    assert_ne!(small_pressed.background, small_active.background);
    assert_eq!(radius(&small_pressed), 6.0);
}

#[test]
fn plain_glyph_button_styles_are_transparent_and_theme_colored() {
    for theme in [Theme::Light, Theme::Dark] {
        let expected_text = theme.extended_palette().background.strong.text;
        let medium = medium_plain_glyph_button_style(&theme);
        let small = small_plain_glyph_button_style(&theme);

        assert_eq!(medium.background, Some(Background::Color(Color::TRANSPARENT)));
        assert_eq!(medium.text_color, expected_text);
        assert_eq!(radius(&medium), 8.0);
        assert_eq!(small.background, Some(Background::Color(Color::TRANSPARENT)));
        assert_eq!(small.text_color, expected_text);
        assert_eq!(radius(&small), 6.0);
    }
}

#[test]
fn glyph_button_styles_keep_disabled_state_transparent() {
    let medium = medium_glyph_button_style(button::Status::Disabled);
    let small = small_glyph_button_style(button::Status::Disabled);

    assert_eq!(medium.background, Some(Background::Color(Color::TRANSPARENT)));
    assert_eq!(radius(&medium), 8.0);
    assert_eq!(small.background, Some(Background::Color(Color::TRANSPARENT)));
    assert_eq!(radius(&small), 6.0);
}

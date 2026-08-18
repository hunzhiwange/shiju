use super::panels::{
    SETTINGS_SECTION_CONTENT_PADDING, settings_content_panel_style, settings_divider_style,
    settings_error_banner_style, settings_section_card_style, settings_section_content_style,
    settings_success_banner_style, settings_value_badge_style,
};
use super::{
    settings_control_with_hint, settings_divider, settings_error_banner, settings_modal_card,
    settings_modal_overlay, settings_page_intro, settings_panel, settings_section_card,
    settings_section_content, settings_success_banner, settings_value_badge,
};
use crate::app::Message;
use iced::widget::text;
use iced::{Background, Element, Length, Padding, Shadow, Theme};

fn assert_size(element: &Element<'_, Message>, width: Length, height: Length) {
    let size = element.as_widget().size();

    assert_eq!(size.width, width);
    assert_eq!(size.height, height);
}

#[test]
fn settings_panel_fills_available_width() {
    let element: Element<'_, Message> = settings_panel(text("panel")).into();

    assert_size(&element, Length::Fill, Length::Shrink);
}

#[test]
fn settings_section_content_is_frameless_and_fills_width() {
    let element: Element<'_, Message> = settings_section_content(text("content")).into();
    let dark = settings_section_content_style(&Theme::Dark);
    let light = settings_section_content_style(&Theme::Light);

    assert_size(&element, Length::Fill, Length::Shrink);
    assert_eq!(SETTINGS_SECTION_CONTENT_PADDING, Padding::ZERO);
    for style in [dark, light] {
        assert_eq!(style.background, None);
        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.shadow, Shadow::default());
    }
}

#[test]
fn settings_modal_card_shrinks_to_content() {
    let element: Element<'_, Message> = settings_modal_card(text("card")).into();

    assert_size(&element, Length::Shrink, Length::Shrink);
}

#[test]
fn settings_modal_overlay_without_base_stacks_backdrop_and_card() {
    let element = settings_modal_overlay(None, Message::None, text("card"));

    assert_eq!(element.as_widget().children().len(), 2);
    assert_size(&element, Length::Fill, Length::Fill);
}

#[test]
fn settings_modal_overlay_with_base_adds_base_layer() {
    let base: Element<'_, Message> = text("base").into();
    let element = settings_modal_overlay(Some(base), Message::None, text("card"));

    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn settings_section_card_fills_width() {
    let element: Element<'_, Message> = settings_section_card("Title", "Description").into();

    assert_size(&element, Length::Fill, Length::Shrink);
}

#[test]
fn settings_page_intro_renders_title_and_description() {
    let element = settings_page_intro("Title", "Description");

    assert_eq!(element.as_widget().children().len(), 2);
    assert_size(&element, Length::Shrink, Length::Shrink);
}

#[test]
fn settings_divider_uses_fill_width_and_fixed_height() {
    let element = settings_divider();

    assert_size(&element, Length::Fill, Length::Fixed(1.0));
}

#[test]
fn settings_value_badge_shrinks_to_content() {
    let element = settings_value_badge("128K");

    assert_size(&element, Length::Shrink, Length::Shrink);
}

#[test]
fn settings_value_badge_stringifies_non_string_values() {
    let element = settings_value_badge(42);

    assert_size(&element, Length::Shrink, Length::Shrink);
}

#[test]
fn settings_control_with_hint_stacks_hint_below_control() {
    let element = settings_control_with_hint(text("control"), "hint");

    assert_eq!(element.as_widget().children().len(), 2);
    assert_size(&element, Length::Fill, Length::Shrink);
}

#[test]
fn settings_banners_fill_available_width() {
    let error = settings_error_banner("bad");
    let success = settings_success_banner("ok");

    assert_size(&error, Length::Fill, Length::Shrink);
    assert_size(&success, Length::Fill, Length::Shrink);
}

#[test]
fn settings_section_card_style_differs_between_themes() {
    let dark = settings_section_card_style(&Theme::Dark);
    let light = settings_section_card_style(&Theme::Light);

    assert_ne!(dark.background, light.background);
    assert_ne!(dark.border.color, light.border.color);
    assert_eq!(dark.border.width, 1.0);
    assert_eq!(dark.border.radius, 8.0.into());
}

#[test]
fn settings_content_panel_uses_half_radius() {
    let dark = settings_content_panel_style(&Theme::Dark);
    let light = settings_content_panel_style(&Theme::Light);

    assert_eq!(dark.border.radius, 9.0.into());
    assert_eq!(light.border.radius, 9.0.into());
}

#[test]
fn settings_divider_style_differs_between_themes() {
    let dark = settings_divider_style(&Theme::Dark);
    let light = settings_divider_style(&Theme::Light);

    assert_ne!(dark.background, light.background);
}

#[test]
fn settings_value_badge_style_differs_between_themes() {
    let dark = settings_value_badge_style(&Theme::Dark);
    let light = settings_value_badge_style(&Theme::Light);

    assert_ne!(dark.background, light.background);
    assert_ne!(dark.border.color, light.border.color);
    assert_eq!(dark.border.radius, 999.0.into());
    assert!(dark.text_color.is_some());
}

#[test]
fn settings_error_banner_style_uses_danger_palette() {
    let style = settings_error_banner_style(&Theme::Dark);

    assert!(style.background.is_some());
    assert!(style.text_color.is_some());
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.radius, 14.0.into());

    if let Some(Background::Color(bg)) = style.background {
        assert!(bg.a > 0.0);
    } else {
        panic!("expected solid color background");
    }
}

#[test]
fn settings_success_banner_style_uses_success_palette() {
    let style = settings_success_banner_style(&Theme::Dark);

    assert!(style.background.is_some());
    assert!(style.text_color.is_some());
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.radius, 14.0.into());

    if let Some(Background::Color(bg)) = style.background {
        assert!(bg.a > 0.0);
    } else {
        panic!("expected solid color background");
    }
}

#[test]
fn settings_banner_styles_have_same_shape_both_themes() {
    let dark_err = settings_error_banner_style(&Theme::Dark);
    let light_err = settings_error_banner_style(&Theme::Light);
    let dark_ok = settings_success_banner_style(&Theme::Dark);
    let light_ok = settings_success_banner_style(&Theme::Light);

    assert_eq!(dark_err.border.width, light_err.border.width);
    assert_eq!(dark_ok.border.width, light_ok.border.width);
    assert_eq!(dark_err.border.radius, light_err.border.radius);
    assert_eq!(dark_ok.border.radius, light_ok.border.radius);
}

#[test]
fn settings_section_card_with_empty_strings_does_not_panic() {
    let element: Element<'_, Message> = settings_section_card("", "").into();
    assert_size(&element, Length::Fill, Length::Shrink);
}

#[test]
fn settings_page_intro_with_empty_strings_does_not_panic() {
    let element = settings_page_intro("", "");
    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn settings_value_badge_with_empty_string() {
    let element = settings_value_badge("");
    assert_size(&element, Length::Shrink, Length::Shrink);
}

#[test]
fn settings_value_badge_with_owned_string() {
    let element = settings_value_badge("hello".to_string());
    assert_size(&element, Length::Shrink, Length::Shrink);
}

#[test]
fn settings_error_banner_with_empty_message() {
    let element = settings_error_banner("");
    assert_size(&element, Length::Fill, Length::Shrink);
}

#[test]
fn settings_success_banner_with_empty_message() {
    let element = settings_success_banner("");
    assert_size(&element, Length::Fill, Length::Shrink);
}

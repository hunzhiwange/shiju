use super::{
    empty_session_placeholder, is_dark_theme, placeholder_card_style, placeholder_chip_style,
    session_loading_placeholder,
};
use crate::app::App;
use iced::{Background, Color, Theme, theme::Palette};

#[test]
fn is_dark_theme_detects_dark_mode() {
    assert!(is_dark_theme(&Theme::Dark));
    assert!(!is_dark_theme(&Theme::Light));
}

#[test]
fn placeholder_card_style_light_theme_has_expected_structure() {
    let style = placeholder_card_style(&Theme::Light);
    assert!(style.background.is_some());
    assert_eq!(style.border.width, 1.0);
}

#[test]
fn placeholder_card_style_dark_theme_has_expected_structure() {
    let style = placeholder_card_style(&Theme::Dark);
    assert!(style.background.is_some());
    assert_eq!(style.border.width, 1.0);
}

#[test]
fn placeholder_card_style_preserves_theme_specific_surface_and_shadow() {
    let light = placeholder_card_style(&Theme::Light);
    let dark = placeholder_card_style(&Theme::Dark);

    assert_eq!(light.background, Some(Background::Color(Color::from_rgba8(252, 252, 253, 1.0))));
    assert_eq!(dark.background, Some(Background::Color(Color::from_rgba8(20, 21, 24, 0.96))));
    assert_eq!(light.border.radius, 22.0.into());
    assert_eq!(dark.border.radius, 22.0.into());
    assert_eq!(light.shadow.offset, iced::Vector::new(0.0, 12.0));
    assert_eq!(dark.shadow.offset, iced::Vector::new(0.0, 12.0));
    assert_eq!(light.shadow.blur_radius, 28.0);
    assert_eq!(dark.shadow.blur_radius, 28.0);
    assert!(dark.shadow.color.a > light.shadow.color.a);
}

#[test]
fn placeholder_chip_style_light_theme_has_expected_structure() {
    let style = placeholder_chip_style(&Theme::Light);
    assert!(style.background.is_some());
    assert_eq!(style.border.width, 1.0);
}

#[test]
fn placeholder_chip_style_dark_theme_has_expected_structure() {
    let style = placeholder_chip_style(&Theme::Dark);
    assert!(style.background.is_some());
    assert_eq!(style.border.width, 1.0);
}

#[test]
fn placeholder_chip_style_preserves_pill_radius_and_theme_surfaces() {
    let light = placeholder_chip_style(&Theme::Light);
    let dark = placeholder_chip_style(&Theme::Dark);

    assert_eq!(light.background, Some(Background::Color(Color::from_rgba8(247, 248, 250, 1.0))));
    assert_eq!(dark.background, Some(Background::Color(Color::from_rgba8(24, 25, 29, 0.92))));
    assert_eq!(light.border.radius, 999.0.into());
    assert_eq!(dark.border.radius, 999.0.into());
}

#[test]
fn placeholder_chip_style_keeps_lightweight_defaults() {
    let style = placeholder_chip_style(&Theme::Light);

    assert_eq!(style.text_color, None);
    assert_eq!(style.shadow, iced::Shadow::default());
}

#[test]
fn dark_theme_threshold_is_strictly_below_half_rgb_sum() {
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

#[test]
fn session_loading_placeholder_produces_element_with_default_app() {
    let app = App::new().0;
    let _element = session_loading_placeholder(&app);
}

#[test]
fn session_loading_placeholder_produces_element_with_context() {
    let mut app = App::new().0;
    app.active_session_id = Some("test-id".into());
    app.project_path = Some("/project".into());
    app.selected_branch = Some("main".into());
    app.project_updated_at_ms = Some(1_000_000);
    let _element = session_loading_placeholder(&app);
}

#[test]
fn empty_session_placeholder_produces_element_with_default_app() {
    let app = App::new().0;
    let _element = empty_session_placeholder(&app);
}

#[test]
fn empty_session_placeholder_produces_element_with_context() {
    let mut app = App::new().0;
    app.project_path = Some("/project".into());
    app.selected_branch = Some("develop".into());
    app.project_updated_at_ms = Some(2_000_000);
    let _element = empty_session_placeholder(&app);
}

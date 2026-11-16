use iced::{Background, Color, Theme};
#[test]
fn top_bar_height_remains_28_pixels_for_project_layout() {
    assert_eq!(crate::components::top_bar::TOP_BAR_HEIGHT, 28.0);
}

#[test]
fn top_bar_style_uses_theme_palette_with_fixed_border_and_no_shadow() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let style = crate::components::top_bar::top_bar_style(&theme);

        assert_eq!(style.text_color, None);
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.color, palette.background.strong.color.scale_alpha(0.60));
        assert_eq!(style.border.radius.top_left, 0.0);
        assert_eq!(style.border.radius.top_right, 0.0);
        assert_eq!(style.border.radius.bottom_right, 0.0);
        assert_eq!(style.border.radius.bottom_left, 0.0);
        assert_eq!(style.shadow, Default::default());

        let Some(Background::Color(background)) = style.background else {
            panic!("expected top bar style to set a color background");
        };

        assert_eq!(background, Color { a: 0.88, ..palette.background.weak.color });
    }
}

use super::*;

#[test]
fn color_format_display_uses_user_facing_uppercase_labels() {
    let cases = [
        (ColorFormat::Hex, "HEX"),
        (ColorFormat::Rgba, "RGBA"),
        (ColorFormat::Hsl, "HSL"),
        (ColorFormat::Css, "CSS"),
    ];

    for (format, expected) in cases {
        assert_eq!(format.to_string(), expected);
    }
}

#[test]
fn prop_text_input_style_uses_focus_palette_for_both_themes() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.palette();
        let extended = theme.extended_palette();
        let active = prop_text_input_style(&theme, text_input::Status::Active);
        let focused =
            prop_text_input_style(&theme, text_input::Status::Focused { is_hovered: false });

        assert_eq!(active.background, Background::Color(extended.background.base.color));
        assert_eq!(active.border.color, extended.background.strong.color);
        assert_eq!(focused.background, Background::Color(extended.background.weak.color));
        assert_eq!(focused.border.color, palette.primary);
        assert_eq!(active.border.width, 1.0);
        assert_eq!(focused.border.radius.top_left, 8.0);
        assert_eq!(focused.value, palette.text);
        assert_eq!(focused.icon, palette.text.scale_alpha(0.5));
        assert_eq!(focused.placeholder, palette.text.scale_alpha(0.55));
        assert_eq!(focused.selection, palette.primary.scale_alpha(0.30));
    }
}

#[test]
fn prop_text_input_style_treats_hovered_and_disabled_as_unfocused() {
    let theme = Theme::Dark;
    let active = prop_text_input_style(&theme, text_input::Status::Active);
    let hovered = prop_text_input_style(&theme, text_input::Status::Hovered);
    let disabled = prop_text_input_style(&theme, text_input::Status::Disabled);

    assert_eq!(hovered.background, active.background);
    assert_eq!(hovered.border.color, active.border.color);
    assert_eq!(disabled.background, active.background);
    assert_eq!(disabled.border.color, active.border.color);
}

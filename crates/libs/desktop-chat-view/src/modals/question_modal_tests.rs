use iced::{Background, Color, Theme, theme::Palette};
fn solid_background(background: Option<Background>) -> iced::Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected solid background, got {other:?}"),
    }
}

#[test]
fn theme_detection_classifies_builtin_dark_and_light_palettes() {
    assert!(super::question_modal_is_dark_theme(&Theme::Dark));
    assert!(!super::question_modal_is_dark_theme(&Theme::Light));
}

#[test]
fn theme_detection_treats_exact_brightness_threshold_as_light() {
    let at_threshold = Theme::custom(
        "threshold",
        Palette { background: Color::from_rgb(0.5, 0.5, 0.5), ..Palette::LIGHT },
    );
    let below_threshold = Theme::custom(
        "below-threshold",
        Palette { background: Color::from_rgb(0.49, 0.5, 0.5), ..Palette::LIGHT },
    );

    assert!(!super::question_modal_is_dark_theme(&at_threshold));
    assert!(super::question_modal_is_dark_theme(&below_threshold));
}

#[test]
fn option_text_colors_follow_theme_and_selection_state() {
    for theme in [Theme::Dark, Theme::Light] {
        assert_eq!(super::question_option_title_color(&theme), theme.palette().text);
        assert_eq!(
            super::question_option_description_color(&theme, false),
            theme.palette().text.scale_alpha(0.68)
        );
        assert_eq!(
            super::question_option_description_color(&theme, true),
            theme.palette().text.scale_alpha(0.82)
        );
    }
}

#[test]
fn selected_option_styles_cover_statuses_in_dark_and_light_themes() {
    for (theme, active_alpha, hovered_alpha, pressed_alpha, border_alpha, shadow_alpha) in
        [(Theme::Dark, 0.26, 0.32, 0.38, 0.62, 0.16), (Theme::Light, 0.12, 0.16, 0.20, 0.34, 0.08)]
    {
        let primary = theme.palette().primary;
        let active =
            super::question_option_button_style(&theme, iced::widget::button::Status::Active, true);
        let hovered = super::question_option_button_style(
            &theme,
            iced::widget::button::Status::Hovered,
            true,
        );
        let pressed = super::question_option_button_style(
            &theme,
            iced::widget::button::Status::Pressed,
            true,
        );
        let disabled = super::question_option_button_style(
            &theme,
            iced::widget::button::Status::Disabled,
            true,
        );

        assert_eq!(solid_background(active.background), primary.scale_alpha(active_alpha));
        assert_eq!(solid_background(hovered.background), primary.scale_alpha(hovered_alpha));
        assert_eq!(solid_background(pressed.background), primary.scale_alpha(pressed_alpha));
        assert_eq!(solid_background(disabled.background), primary.scale_alpha(active_alpha));
        assert_eq!(active.border.color, primary.scale_alpha(border_alpha));
        assert_eq!(active.text_color, theme.palette().text);
        assert_eq!(active.shadow.color, primary.scale_alpha(shadow_alpha));
        assert_eq!(active.shadow.blur_radius, 18.0);
    }
}

#[test]
fn unselected_option_styles_cover_statuses_in_dark_and_light_themes() {
    let dark = Theme::Dark;
    let dark_palette = dark.extended_palette();
    let dark_active =
        super::question_option_button_style(&dark, iced::widget::button::Status::Active, false);
    let dark_hovered =
        super::question_option_button_style(&dark, iced::widget::button::Status::Hovered, false);
    let dark_pressed =
        super::question_option_button_style(&dark, iced::widget::button::Status::Pressed, false);
    let dark_disabled =
        super::question_option_button_style(&dark, iced::widget::button::Status::Disabled, false);

    assert_eq!(
        solid_background(dark_active.background),
        dark_palette.background.weak.color.scale_alpha(0.62)
    );
    assert_eq!(
        solid_background(dark_hovered.background),
        dark_palette.background.weak.color.scale_alpha(0.82)
    );
    assert_eq!(
        solid_background(dark_pressed.background),
        dark_palette.background.strong.color.scale_alpha(0.92)
    );
    assert_eq!(
        solid_background(dark_disabled.background),
        solid_background(dark_active.background)
    );
    assert_eq!(dark_active.border.color, dark_palette.background.strong.color.scale_alpha(0.90));
    assert_eq!(dark_active.text_color, dark.palette().text);
    assert_eq!(dark_active.shadow, iced::Shadow::default());

    let light = Theme::Light;
    let light_active =
        super::question_option_button_style(&light, iced::widget::button::Status::Active, false);
    let light_hovered =
        super::question_option_button_style(&light, iced::widget::button::Status::Hovered, false);
    let light_pressed =
        super::question_option_button_style(&light, iced::widget::button::Status::Pressed, false);
    let light_disabled =
        super::question_option_button_style(&light, iced::widget::button::Status::Disabled, false);

    assert_eq!(solid_background(light_active.background), Color::from_rgba8(247, 248, 250, 0.96));
    assert_eq!(solid_background(light_hovered.background), Color::from_rgba8(241, 244, 248, 0.98));
    assert_eq!(solid_background(light_pressed.background), Color::from_rgba8(233, 237, 243, 0.98));
    assert_eq!(
        solid_background(light_disabled.background),
        solid_background(light_active.background)
    );
    assert_eq!(light_active.border.color, Color::from_rgba8(210, 216, 224, 1.0));
    assert_eq!(light_active.text_color, light.palette().text);
    assert_eq!(light_active.shadow, iced::Shadow::default());
}

#[test]
fn question_option_styles_follow_theme_and_selection_state() {
    let dark_theme = Theme::Dark;
    let active = super::question_option_button_style(
        &dark_theme,
        iced::widget::button::Status::Active,
        false,
    );
    let selected = super::question_option_button_style(
        &dark_theme,
        iced::widget::button::Status::Active,
        true,
    );
    let hovered = super::question_option_button_style(
        &dark_theme,
        iced::widget::button::Status::Hovered,
        false,
    );

    assert_ne!(solid_background(active.background), solid_background(selected.background));
    assert_ne!(solid_background(active.background), solid_background(hovered.background));
    assert_eq!(active.text_color, selected.text_color);
    assert!(selected.shadow.blur_radius > 0.0);
}

use iced::widget::button;
use iced::{Background, Color, Theme};

use super::*;

const EPSILON: f32 = 0.000_01;

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= EPSILON, "expected {actual} to be close to {expected}");
}

fn assert_color_close(actual: Color, expected: Color) {
    assert_close(actual.r, expected.r);
    assert_close(actual.g, expected.g);
    assert_close(actual.b, expected.b);
    assert_close(actual.a, expected.a);
}

fn background_color(style: iced::widget::container::Style) -> Color {
    match style.background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}

fn button_background_color(style: iced::widget::button::Style) -> Color {
    match style.background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}

#[test]
fn is_dark_theme_classifies_builtin_themes() {
    assert!(!is_dark_theme(&Theme::Light));
    assert!(is_dark_theme(&Theme::Dark));
}

#[test]
fn blend_color_clamps_amount_and_interpolates_alpha() {
    let from = Color::from_rgba(0.1, 0.2, 0.3, 0.4);
    let to = Color::from_rgba(0.9, 0.6, 0.1, 0.8);

    assert_color_close(blend_color(from, to, -1.0), from);
    assert_color_close(blend_color(from, to, 2.0), to);
    assert_color_close(blend_color(from, to, 0.25), Color::from_rgba(0.3, 0.3, 0.25, 0.5));
}

#[test]
fn elevated_shadow_uses_theme_specific_alpha() {
    let dark = elevated_shadow(&Theme::Dark, 0.42, 0.11);
    let light = elevated_shadow(&Theme::Light, 0.42, 0.11);

    assert_color_close(dark.color, Color::BLACK.scale_alpha(0.42));
    assert_color_close(light.color, Color::BLACK.scale_alpha(0.11));
    assert_close(dark.offset.x, 0.0);
    assert_close(dark.offset.y, 12.0);
    assert_close(dark.blur_radius, 28.0);
}

#[test]
fn workspace_background_style_blends_by_theme() {
    let dark = background_color(workspace_background_style(&Theme::Dark));
    let light = background_color(workspace_background_style(&Theme::Light));

    assert_color_close(
        dark,
        blend_color(Theme::Dark.extended_palette().background.base.color, Color::BLACK, 0.24),
    );
    assert_color_close(
        light,
        blend_color(
            Theme::Light.extended_palette().background.base.color,
            Color::from_rgb8(244, 246, 250),
            0.72,
        ),
    );
}

#[test]
fn content_panel_style_sets_dark_and_light_surfaces() {
    let dark = content_panel_style(&Theme::Dark);
    let light = content_panel_style(&Theme::Light);

    assert_color_close(
        background_color(dark),
        crate::app::theme_catalog::panel_surface_color(&Theme::Dark, 0.98),
    );
    assert_close(dark.border.width, 1.0);
    assert_close(dark.border.radius.top_left, 18.0);
    assert_color_close(
        dark.border.color,
        Theme::Dark.extended_palette().background.strong.color.scale_alpha(0.70),
    );
    assert_color_close(dark.shadow.color, Color::BLACK.scale_alpha(0.14));

    assert_color_close(
        background_color(light),
        crate::app::theme_catalog::panel_surface_color(&Theme::Light, 0.985),
    );
    assert_color_close(
        light.border.color,
        Theme::Light.extended_palette().background.strong.color.scale_alpha(0.55),
    );
    assert_color_close(light.shadow.color, Color::BLACK.scale_alpha(0.04));
}

#[test]
fn content_panel_style_follows_custom_palette_background() {
    let green = Theme::custom(
        "green-surface".to_string(),
        iced::theme::Palette {
            background: Color::from_rgb8(0x12, 0x4A, 0x2E),
            text: Color::from_rgb8(0xE8, 0xF5, 0xEC),
            primary: Color::from_rgb8(0x3D, 0xB8, 0x6B),
            success: Color::from_rgb8(0x3D, 0xB8, 0x6B),
            warning: Color::from_rgb8(0xE6, 0xB8, 0x00),
            danger: Color::from_rgb8(0xE0, 0x4F, 0x5F),
        },
    );
    let style = content_panel_style(&green);
    let bg = background_color(style);
    assert_color_close(bg, crate::app::theme_catalog::panel_surface_color(&green, 0.98));
    assert!(
        (bg.g - Color::from_rgb8(0x12, 0x4A, 0x2E).g).abs() < 1.0e-5,
        "content panel should keep custom green background, got {bg:?}"
    );
}

#[test]
fn left_rail_and_no_border_styles_are_transparent() {
    for style in [left_rail_style(&Theme::Dark), panel_style_no_border(&Theme::Light)] {
        assert!(style.background.is_none());
        assert_color_close(style.border.color, Color::TRANSPARENT);
        assert_close(style.border.width, 0.0);
        assert_close(style.border.radius.top_left, 0.0);
        assert_eq!(style.shadow, iced::Shadow::default());
    }
}

#[test]
fn divider_session_and_highlight_colors_follow_theme() {
    assert_color_close(divider_line_color(&Theme::Dark), Color::from_rgba8(50, 54, 61, 0.96));
    assert_color_close(divider_line_color(&Theme::Light), Color::from_rgba8(221, 225, 232, 1.0));
    assert_color_close(
        session_border_color(&Theme::Dark),
        Color::from_rgba8(50, 54, 61, 0.96).scale_alpha(0.96),
    );
    assert_color_close(session_border_color(&Theme::Light), divider_line_color(&Theme::Light));
    assert_color_close(
        session_row_highlight_color(&Theme::Dark),
        Color::from_rgba8(37, 41, 48, 0.96),
    );
    assert_color_close(
        session_row_highlight_color(&Theme::Light),
        Color::from_rgba8(238, 241, 246, 1.0),
    );
}

#[test]
fn session_panel_style_uses_radius_and_theme_palette() {
    let dark = session_panel_style(&Theme::Dark, 14.0);
    let light = session_panel_style(&Theme::Light, 10.0);
    let dark_palette = Theme::Dark.extended_palette();

    assert_color_close(
        background_color(dark),
        blend_color(dark_palette.background.base.color, dark_palette.background.strong.color, 0.28)
            .scale_alpha(0.99),
    );
    assert_color_close(dark.border.color, dark_palette.background.strong.color.scale_alpha(0.78));
    assert_close(dark.border.radius.top_left, 18.0);
    assert_color_close(dark.shadow.color, Color::BLACK.scale_alpha(0.18));

    assert_color_close(
        background_color(light),
        crate::app::theme_catalog::panel_surface_color(&Theme::Light, 0.99),
    );
    assert_color_close(
        light.border.color,
        Theme::Light.extended_palette().background.strong.color.scale_alpha(0.55),
    );
    assert_close(light.border.radius.top_left, 14.0);
    assert_color_close(light.shadow.color, Color::BLACK.scale_alpha(0.05));
}

#[test]
fn right_column_styles_apply_inner_background_and_outer_border() {
    let dark_inner = right_column_inner_style(&Theme::Dark, 12.0);
    let light_inner = right_column_inner_style(&Theme::Light, 16.0);
    let outer = right_column_outer_style(&Theme::Light, 20.0);
    let dark_palette = Theme::Dark.extended_palette();

    assert_color_close(
        background_color(dark_inner),
        blend_color(dark_palette.background.base.color, dark_palette.background.strong.color, 0.22)
            .scale_alpha(0.99),
    );
    assert_close(dark_inner.border.width, 0.0);
    assert_close(dark_inner.border.radius.top_left, 16.0);
    assert_color_close(
        background_color(light_inner),
        crate::app::theme_catalog::panel_surface_color(&Theme::Light, 0.99),
    );
    assert_close(light_inner.border.radius.top_left, 20.0);

    assert!(outer.background.is_none());
    assert_close(outer.border.width, 1.0);
    assert_close(outer.border.radius.top_left, 24.0);
    assert_color_close(outer.border.color, session_border_color(&Theme::Light));
    assert_eq!(outer.shadow, iced::Shadow::default());
}

#[test]
fn tooltip_bubble_style_sets_dark_popover_surface() {
    let style = tooltip_bubble_style(&Theme::Light);

    assert_color_close(style.text_color.unwrap(), Color::WHITE);
    assert_color_close(background_color(style), Color::from_rgb8(28, 28, 30));
    assert_close(style.border.width, 0.0);
    assert_color_close(style.border.color, Color::TRANSPARENT);
    assert_close(style.border.radius.top_left, 12.0);
    assert_color_close(style.shadow.color, Color::BLACK.scale_alpha(0.32));
    assert_close(style.shadow.offset.x, 0.0);
    assert_close(style.shadow.offset.y, 4.0);
    assert_close(style.shadow.blur_radius, 16.0);
    assert!(!style.snap);
}

#[test]
fn tooltip_bubble_can_be_constructed() {
    let bubble = tooltip_bubble("hover details".to_owned());

    let _bubble = std::hint::black_box(bubble);
}

#[test]
fn project_item_button_style_covers_inactive_and_selected_states() {
    let accent = Color::from_rgb8(64, 145, 255);
    let theme = Theme::Light;
    let base = theme.extended_palette().background.base.color;

    let inactive = project_item_button_style(&theme, false, accent, button::Status::Active);
    assert_color_close(button_background_color(inactive), Color::TRANSPARENT);
    assert_close(inactive.border.width, 0.0);
    assert_color_close(inactive.border.color, Color::TRANSPARENT);
    assert_eq!(inactive.shadow, iced::Shadow::default());

    let selected = project_item_button_style(&theme, true, accent, button::Status::Active);
    assert_color_close(button_background_color(selected), blend_color(base, accent, 0.07));
    assert_color_close(selected.border.color, blend_color(accent, base, 0.36));
    assert_close(selected.border.width, 1.0);
    assert_close(selected.border.radius.top_left, 18.0);
    assert_color_close(selected.shadow.color, accent.scale_alpha(0.05));
    assert_close(selected.shadow.offset.y, 8.0);
    assert_close(selected.shadow.blur_radius, 18.0);
    assert_color_close(selected.text_color, theme.palette().text);
}

#[test]
fn project_item_button_style_covers_hovered_and_pressed_states() {
    let accent = Color::from_rgb8(64, 145, 255);
    let theme = Theme::Dark;
    let base = theme.extended_palette().background.base.color;

    let hovered = project_item_button_style(&theme, false, accent, button::Status::Hovered);
    assert_color_close(button_background_color(hovered), blend_color(base, accent, 0.16));
    assert_color_close(hovered.border.color, blend_color(accent, base, 0.28));
    assert_close(hovered.border.width, 1.0);
    assert_color_close(hovered.shadow.color, accent.scale_alpha(0.14));
    assert_close(hovered.shadow.offset.y, 8.0);
    assert_close(hovered.shadow.blur_radius, 18.0);

    let pressed = project_item_button_style(&theme, true, accent, button::Status::Pressed);
    assert_color_close(button_background_color(pressed), blend_color(base, accent, 0.26));
    assert_color_close(pressed.border.color, blend_color(accent, base, 0.18));
    assert_color_close(pressed.shadow.color, accent.scale_alpha(0.18));
    assert_close(pressed.shadow.offset.y, 4.0);
    assert_close(pressed.shadow.blur_radius, 10.0);
}

#[test]
fn project_item_button_style_covers_remaining_theme_branches() {
    let accent = Color::from_rgb8(64, 145, 255);
    let light = Theme::Light;
    let light_base = light.extended_palette().background.base.color;
    let dark = Theme::Dark;
    let dark_base = dark.extended_palette().background.base.color;

    let light_hovered = project_item_button_style(&light, false, accent, button::Status::Hovered);
    assert_color_close(
        button_background_color(light_hovered),
        blend_color(light_base, accent, 0.11),
    );
    assert_color_close(light_hovered.shadow.color, accent.scale_alpha(0.07));

    let light_pressed = project_item_button_style(&light, false, accent, button::Status::Pressed);
    assert_color_close(
        button_background_color(light_pressed),
        blend_color(light_base, accent, 0.18),
    );
    assert_color_close(light_pressed.shadow.color, accent.scale_alpha(0.10));

    let dark_selected = project_item_button_style(&dark, true, accent, button::Status::Active);
    assert_color_close(
        button_background_color(dark_selected),
        blend_color(dark_base, accent, 0.11),
    );
    assert_color_close(dark_selected.shadow.color, accent.scale_alpha(0.10));

    let disabled = project_item_button_style(&dark, false, accent, button::Status::Disabled);
    assert_color_close(button_background_color(disabled), Color::TRANSPARENT);
    assert_close(disabled.border.width, 0.0);
    assert_eq!(disabled.shadow, iced::Shadow::default());
}

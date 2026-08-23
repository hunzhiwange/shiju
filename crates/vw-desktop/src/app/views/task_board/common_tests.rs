//! 任务看板共享控件的测试，覆盖图标、按钮状态和主题视觉参数。
//! 渲染与颜色近似检查确保已知及未知提供商都能得到稳定展示。
use iced::widget::{button, svg};
use iced::{Background, Color, Element, Length, Theme};

use super::{
    SUBTASK_BADGE_SIZE, auto_icon, button_style_danger, button_style_primary,
    button_style_secondary, button_style_success, provider_logo_handle,
};

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

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected solid color background, got {other:?}"),
    }
}

fn assert_svg_handle_renders_at_size(handle: svg::Handle, size: f32) {
    let element: Element<'_, ()> = svg::Svg::new(handle).width(size).height(size).into();
    let widget_size = element.as_widget().size();

    assert_eq!(widget_size.width, Length::Fixed(size));
    assert_eq!(widget_size.height, Length::Fixed(size));
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("common_tests"));
}

#[test]
fn provider_logo_handle_supports_known_unknown_and_empty_provider_ids() {
    for provider_id in ["openai", "unknown-provider", ""] {
        assert_svg_handle_renders_at_size(provider_logo_handle(provider_id), 16.0);
    }
}

#[test]
fn auto_icon_returns_renderable_star_handle() {
    assert_svg_handle_renders_at_size(auto_icon(), SUBTASK_BADGE_SIZE);
}

#[test]
fn primary_button_style_preserves_status_background_and_adds_board_elevation() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();
    let cases = [
        (button::Status::Active, palette.primary.base.color),
        (button::Status::Hovered, palette.primary.base.color.scale_alpha(0.92)),
        (button::Status::Pressed, palette.primary.base.color.scale_alpha(0.85)),
        (button::Status::Disabled, palette.primary.base.color),
    ];

    for (status, expected_background) in cases {
        let style = button_style_primary(&theme, status);

        assert_color_close(background_color(style.background), expected_background);
        assert_color_close(style.text_color, Color::WHITE);
        assert_close(style.border.radius.top_left, 12.0);
        assert_color_close(style.shadow.color, theme.palette().primary.scale_alpha(0.20));
        assert_eq!(style.shadow.offset, iced::Vector::new(0.0, 10.0));
        assert_close(style.shadow.blur_radius, 22.0);
    }
}

#[test]
fn secondary_button_style_keeps_shared_status_colors_and_uses_board_radius() {
    let dark = Theme::Dark;
    let dark_palette = dark.extended_palette();
    let light = Theme::Light;
    let light_palette = light.extended_palette();
    let cases = [
        (&dark, button::Status::Active, dark_palette.background.base.color.scale_alpha(0.52)),
        (&dark, button::Status::Hovered, dark_palette.background.weak.color.scale_alpha(0.9)),
        (&dark, button::Status::Pressed, dark_palette.background.strong.color.scale_alpha(0.94)),
        (&light, button::Status::Disabled, Color::WHITE.scale_alpha(0.56)),
        (&light, button::Status::Hovered, Color::WHITE.scale_alpha(0.88)),
        (&light, button::Status::Pressed, light_palette.background.weak.color.scale_alpha(0.92)),
    ];

    for (theme, status, expected_background) in cases {
        let style = button_style_secondary(theme, status);

        assert_color_close(background_color(style.background), expected_background);
        assert_close(style.border.radius.top_left, 12.0);
        assert_close(style.border.width, 1.0);
    }
}

#[test]
fn danger_button_style_distinguishes_pressed_state_and_uses_board_radius() {
    let theme = Theme::Light;
    let cases = [
        (button::Status::Active, Color::from_rgba8(255, 59, 48, 0.96)),
        (button::Status::Hovered, Color::from_rgba8(255, 59, 48, 0.96)),
        (button::Status::Pressed, Color::from_rgba8(220, 45, 40, 0.98)),
        (button::Status::Disabled, Color::from_rgba8(255, 59, 48, 0.96)),
    ];

    for (status, expected_background) in cases {
        let style = button_style_danger(&theme, status);

        assert_color_close(background_color(style.background), expected_background);
        assert_color_close(style.text_color, Color::WHITE);
        assert_close(style.border.radius.top_left, 12.0);
    }
}

#[test]
fn success_button_style_maps_statuses_to_colors_and_theme_shadow() {
    let theme = Theme::Dark;
    let cases = [
        (button::Status::Active, Color::from_rgb8(34, 197, 94).scale_alpha(0.90)),
        (button::Status::Hovered, Color::from_rgb8(22, 163, 74).scale_alpha(0.92)),
        (button::Status::Pressed, Color::from_rgb8(21, 128, 61).scale_alpha(0.94)),
        (button::Status::Disabled, Color::from_rgb8(34, 197, 94).scale_alpha(0.90)),
    ];

    for (status, expected_background) in cases {
        let style = button_style_success(&theme, status);

        assert_color_close(background_color(style.background), expected_background);
        assert_color_close(style.text_color, Color::WHITE);
        assert_close(style.border.radius.top_left, 12.0);
        assert_close(style.border.width, 1.0);
        assert_color_close(style.border.color, Color::from_rgb8(22, 163, 74).scale_alpha(0.42));
        assert_color_close(
            style.shadow.color,
            theme.extended_palette().success.base.color.scale_alpha(0.16),
        );
        assert_eq!(style.shadow.offset, iced::Vector::new(0.0, 10.0));
        assert_close(style.shadow.blur_radius, 22.0);
    }
}

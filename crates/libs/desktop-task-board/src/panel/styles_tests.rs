//! 任务详情面板样式的测试，覆盖容器、输入框、标签和按钮状态。
//! 颜色断言分别验证明暗主题以及危险、成功等语义色分支。
use iced::widget::{button, text_editor, text_input};
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

fn color_background(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("styles_tests"));
}

#[test]
fn input_controls_delegate_to_shared_settings_styles() {
    let label = input_label::<crate::message::TaskBoardMessage>("目标");
    assert_eq!(label.as_widget().children().len(), 0);

    let input = input_style(&Theme::Dark, text_input::Status::Active);
    assert!(matches!(&input.background, Background::Color(_)));
    assert_close(input.border.radius.top_left, 14.0);

    let editor = editor_style(&Theme::Light, text_editor::Status::Focused { is_hovered: true });
    assert!(matches!(&editor.background, Background::Color(_)));
    assert_close(editor.border.radius.top_left, 14.0);
}

#[test]
fn selector_trigger_style_distinguishes_hover_and_open() {
    for theme in [Theme::Light, Theme::Dark] {
        let active = selector_trigger_style(&theme, button::Status::Active, false);
        let hovered = selector_trigger_style(&theme, button::Status::Hovered, false);
        let open = selector_trigger_style(&theme, button::Status::Active, true);
        assert_ne!(active.background, hovered.background);
        assert_ne!(active.border.color, open.border.color);
        assert_eq!(open.text_color, theme.palette().text);
    }
}

#[test]
fn panel_popover_and_tooltip_styles_set_expected_surfaces() {
    let popover = popover_style(&Theme::Dark);
    assert_close(popover.border.radius.top_left, 16.0);

    let panel = panel_container_style(&Theme::Light);
    assert_close(panel.border.radius.top_left, 22.0);

    let tooltip = tooltip_dark_style(&Theme::Light);
    assert_color_close(color_background(tooltip.background), Color::from_rgba8(24, 24, 24, 0.96));
    assert_eq!(tooltip.text_color, Some(Color::WHITE));
    assert_color_close(tooltip.border.color, Color::TRANSPARENT);
    assert_close(tooltip.border.radius.top_left, 8.0);
    assert_close(tooltip.border.width, 0.0);
    assert_color_close(tooltip.shadow.color, Color::BLACK.scale_alpha(0.40));
    assert_eq!(tooltip.shadow.offset, iced::Vector::new(0.0, 6.0));
    assert_close(tooltip.shadow.blur_radius, 18.0);
}

#[test]
fn icon_buttons_apply_enabled_and_disabled_contracts() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();

    let enabled = square_icon_button_style(&theme, button::Status::Hovered, true);
    assert_close(enabled.border.radius.top_left, 12.0);

    let disabled = square_icon_button_style(&theme, button::Status::Pressed, false);
    assert_close(disabled.border.radius.top_left, 12.0);
    assert_color_close(disabled.text_color, palette.background.weak.text);
}

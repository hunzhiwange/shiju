//! 任务详情面板样式的测试，覆盖容器、输入框、标签和按钮状态。
//! 颜色断言分别验证明暗主题以及危险、成功等语义色分支。
use iced::widget::{button, text_editor, text_input};
use iced::{Background, Color, Theme};

use crate::app::task::SubTaskStatus;

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
    let label = input_label("目标");
    assert_eq!(label.as_widget().children().len(), 0);

    let input = input_style(&Theme::Dark, text_input::Status::Active);
    assert!(matches!(&input.background, Background::Color(_)));
    assert_close(input.border.radius.top_left, 14.0);

    let editor = editor_style(&Theme::Light, text_editor::Status::Focused { is_hovered: true });
    assert!(matches!(&editor.background, Background::Color(_)));
    assert_close(editor.border.radius.top_left, 14.0);
}

#[test]
fn pill_button_style_tracks_status_and_theme_brightness() {
    let dark_palette = Theme::Dark.extended_palette();
    let light = Theme::Light;

    let dark_active = pill_button_style(&Theme::Dark, button::Status::Active);
    assert_color_close(
        color_background(dark_active.background),
        dark_palette.background.base.color.scale_alpha(0.72),
    );
    assert_color_close(
        dark_active.border.color,
        dark_palette.background.strong.color.scale_alpha(0.82),
    );
    assert_color_close(dark_active.shadow.color, Color::BLACK.scale_alpha(0.14));
    assert_close(dark_active.border.radius.top_left, 14.0);
    assert_close(dark_active.border.width, 1.0);

    let dark_hovered = pill_button_style(&Theme::Dark, button::Status::Hovered);
    assert_color_close(
        color_background(dark_hovered.background),
        dark_palette.background.weak.color.scale_alpha(0.88),
    );

    let dark_pressed = pill_button_style(&Theme::Dark, button::Status::Pressed);
    assert_color_close(
        color_background(dark_pressed.background),
        dark_palette.background.strong.color.scale_alpha(0.92),
    );

    let light_active = pill_button_style(&light, button::Status::Disabled);
    assert_color_close(color_background(light_active.background), Color::WHITE.scale_alpha(0.84));
    assert_color_close(light_active.border.color, Color::from_rgba8(15, 23, 42, 0.08));
    assert_color_close(light_active.shadow.color, Color::BLACK.scale_alpha(0.06));

    let light_hovered = pill_button_style(&light, button::Status::Hovered);
    assert_color_close(color_background(light_hovered.background), Color::WHITE.scale_alpha(0.96));

    let light_pressed = pill_button_style(&light, button::Status::Pressed);
    assert_color_close(
        color_background(light_pressed.background),
        light.extended_palette().background.strong.color.scale_alpha(0.28),
    );
}

#[test]
fn panel_popover_and_tooltip_styles_set_expected_surfaces() {
    let popover = popover_style(&Theme::Dark);
    assert_close(popover.border.radius.top_left, 18.0);

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
fn icon_and_arrow_buttons_apply_enabled_and_disabled_contracts() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();

    let enabled = square_icon_button_style(&theme, button::Status::Hovered, true);
    assert_close(enabled.border.radius.top_left, 12.0);

    let disabled = square_icon_button_style(&theme, button::Status::Pressed, false);
    assert_close(disabled.border.radius.top_left, 12.0);
    assert_color_close(disabled.text_color, palette.background.weak.text);

    let arrow = disabled_arrow_button_style(&theme);
    assert!(arrow.background.is_none());
    assert_close(arrow.border.radius.top_left, 4.0);
    assert_close(arrow.border.width, 0.0);
    assert_color_close(arrow.border.color, Color::TRANSPARENT);
    assert_color_close(arrow.text_color, palette.background.weak.text);
}

#[test]
fn subtask_card_and_generic_badge_styles_follow_theme_palette() {
    let dark_palette = Theme::Dark.extended_palette();

    let dark_card = subtask_card_style(&Theme::Dark);
    assert_color_close(
        color_background(dark_card.background),
        dark_palette.background.weak.color.scale_alpha(0.18),
    );
    assert_color_close(
        dark_card.border.color,
        dark_palette.background.strong.color.scale_alpha(0.72),
    );
    assert_close(dark_card.border.radius.top_left, 16.0);
    assert_close(dark_card.border.width, 1.0);

    let light_card = subtask_card_style(&Theme::Light);
    assert_color_close(color_background(light_card.background), Color::WHITE.scale_alpha(0.82));

    let badge = subtask_badge_style(&Theme::Light, button::Status::Hovered);
    let palette = Theme::Light.extended_palette();
    assert_color_close(
        color_background(badge.background),
        palette.background.weak.color.scale_alpha(0.55),
    );
    assert_color_close(badge.border.color, palette.background.strong.color.scale_alpha(0.78));
    assert_close(badge.border.radius.top_left, 999.0);
    assert_close(badge.border.width, 1.0);
    assert_color_close(badge.text_color, Theme::Light.palette().text);
}

#[test]
fn subtask_status_icon_and_label_cover_all_statuses() {
    assert_eq!(subtask_status_icon(SubTaskStatus::Pending, 0), ".");
    assert_eq!(subtask_status_icon(SubTaskStatus::Completed, 0), "✓");
    assert_eq!(subtask_status_icon(SubTaskStatus::Failed, 0), "x");

    assert_eq!(subtask_status_icon(SubTaskStatus::Running, 0), "◐");
    assert_eq!(subtask_status_icon(SubTaskStatus::Running, 250), "◓");
    assert_eq!(subtask_status_icon(SubTaskStatus::Running, 500), "◑");
    assert_eq!(subtask_status_icon(SubTaskStatus::Running, 750), "◒");
    assert_eq!(subtask_status_icon(SubTaskStatus::Running, 1000), "◐");

    assert_eq!(subtask_status_label(SubTaskStatus::Pending), "待执行");
    assert_eq!(subtask_status_label(SubTaskStatus::Running), "执行中");
    assert_eq!(subtask_status_label(SubTaskStatus::Completed), "执行成功");
    assert_eq!(subtask_status_label(SubTaskStatus::Failed), "执行失败");
}

#[test]
fn subtask_status_colors_cover_light_and_dark_branches() {
    let cases = [
        (
            SubTaskStatus::Pending,
            Color::from_rgba8(243, 244, 246, 0.92),
            Color::from_rgb8(209, 213, 219),
            Color::from_rgb8(75, 85, 99),
        ),
        (
            SubTaskStatus::Running,
            Color::from_rgb8(243, 232, 255),
            Color::from_rgb8(192, 132, 252),
            Color::from_rgb8(126, 34, 206),
        ),
        (
            SubTaskStatus::Completed,
            Color::from_rgb8(220, 252, 231),
            Color::from_rgb8(74, 222, 128),
            Color::from_rgb8(22, 101, 52),
        ),
        (
            SubTaskStatus::Failed,
            Color::from_rgb8(254, 226, 226),
            Color::from_rgb8(248, 113, 113),
            Color::from_rgb8(185, 28, 28),
        ),
    ];

    for (status, expected_bg, expected_border, expected_text) in cases {
        let (bg, border, text) = subtask_status_colors(&Theme::Light, status);
        assert_color_close(bg, expected_bg);
        assert_color_close(border, expected_border);
        assert_color_close(text, expected_text);

        let text_style = subtask_status_text_style(&Theme::Light, status);
        assert_eq!(text_style.color, Some(expected_text));

        let pill = subtask_status_pill_style(&Theme::Light, status);
        assert_color_close(color_background(pill.background), expected_bg);
        assert_color_close(pill.border.color, expected_border);
        assert_eq!(pill.text_color, Some(expected_text));
        assert_close(pill.border.radius.top_left, 999.0);
        assert_close(pill.border.width, 1.0);
    }

    let dark_palette = Theme::Dark.extended_palette();
    let (pending_bg, pending_border, pending_text) =
        subtask_status_colors(&Theme::Dark, SubTaskStatus::Pending);
    assert_color_close(pending_bg, dark_palette.background.weak.color.scale_alpha(0.18));
    assert_color_close(pending_border, dark_palette.background.strong.color.scale_alpha(0.72));
    assert_color_close(pending_text, dark_palette.background.base.text);

    let (running_bg, _, running_text) = subtask_status_colors(&Theme::Dark, SubTaskStatus::Running);
    assert_color_close(running_bg, Color::from_rgba8(168, 85, 247, 0.22));
    assert_color_close(running_text, Color::from_rgb8(216, 180, 254));

    let (completed_bg, _, completed_text) =
        subtask_status_colors(&Theme::Dark, SubTaskStatus::Completed);
    assert_color_close(completed_bg, Color::from_rgba8(34, 197, 94, 0.18));
    assert_color_close(completed_text, Color::from_rgb8(187, 247, 208));

    let (failed_bg, _, failed_text) = subtask_status_colors(&Theme::Dark, SubTaskStatus::Failed);
    assert_color_close(failed_bg, Color::from_rgba8(239, 68, 68, 0.18));
    assert_color_close(failed_text, Color::from_rgb8(254, 202, 202));
}

#[test]
fn subtask_status_badge_style_dims_interactive_backgrounds() {
    let active =
        subtask_status_badge_style(&Theme::Light, button::Status::Active, SubTaskStatus::Completed);
    assert_color_close(color_background(active.background), Color::from_rgb8(220, 252, 231));
    assert_color_close(active.border.color, Color::from_rgb8(74, 222, 128));
    assert_color_close(active.text_color, Color::from_rgb8(22, 101, 52));
    assert_close(active.border.radius.top_left, 999.0);
    assert_close(active.border.width, 1.0);

    let hovered =
        subtask_status_badge_style(&Theme::Light, button::Status::Hovered, SubTaskStatus::Failed);
    assert_color_close(
        color_background(hovered.background),
        Color::from_rgb8(254, 226, 226).scale_alpha(0.86),
    );
    assert_color_close(hovered.border.color, Color::from_rgb8(248, 113, 113));
    assert_color_close(hovered.text_color, Color::from_rgb8(185, 28, 28));

    let pressed =
        subtask_status_badge_style(&Theme::Dark, button::Status::Pressed, SubTaskStatus::Running);
    assert_color_close(
        color_background(pressed.background),
        Color::from_rgba8(168, 85, 247, 0.22).scale_alpha(0.86),
    );
}

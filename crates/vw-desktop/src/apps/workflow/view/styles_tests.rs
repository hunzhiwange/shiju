//! 工作流视图样式的测试，覆盖根背景、画布、检查器、弹窗与校验摘要。
//! 断言分别核对明暗主题的背景、边框、阴影和危险语义色。
use super::*;
use crate::app::components::system_settings_common::{
    settings_modal_backdrop_style, settings_modal_card_style, settings_panel_style,
};
use iced::{Background, Color, Theme, Vector};

fn option_background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected solid color background, got {other:?}"),
    }
}

#[test]
fn floating_panel_style_uses_theme_background_border_and_shadow() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();
    let style = floating_panel_style(&theme);

    assert_eq!(option_background_color(style.background), palette.background.base.color);
    assert_eq!(style.border.color, palette.background.weak.color);
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.radius.top_left, 12.0);
    assert_eq!(style.shadow.color, Color::BLACK.scale_alpha(0.12));
    assert_eq!(style.shadow.offset, Vector::new(0.0, 10.0));
    assert_eq!(style.shadow.blur_radius, 22.0);
}

#[test]
fn root_style_switches_between_dark_and_light_backgrounds() {
    let dark = root_style(&Theme::Dark);
    let light = root_style(&Theme::Light);

    assert_eq!(
        option_background_color(dark.background),
        Theme::Dark.extended_palette().background.base.color.scale_alpha(0.98)
    );
    assert_eq!(option_background_color(light.background), Color::from_rgba8(244, 247, 252, 1.0));
}

#[test]
fn canvas_panel_style_covers_dark_and_light_branches() {
    let dark_theme = Theme::Dark;
    let light_theme = Theme::Light;
    let dark_palette = dark_theme.extended_palette();
    let dark = canvas_panel_style(&dark_theme);
    let light = canvas_panel_style(&light_theme);

    assert_eq!(
        option_background_color(dark.background),
        dark_palette.background.base.color.scale_alpha(0.94)
    );
    assert_eq!(dark.border.color, dark_palette.background.strong.color.scale_alpha(0.82));
    assert_eq!(dark.shadow.color, Color::BLACK.scale_alpha(0.10));
    assert_eq!(dark.border.radius.top_left, 24.0);
    assert_eq!(option_background_color(light.background), Color::from_rgba8(251, 253, 255, 0.98));
    assert_eq!(light.border.color, Color::from_rgba8(15, 23, 42, 0.08));
    assert_eq!(light.shadow.color, Color::BLACK.scale_alpha(0.05));
    assert_eq!(light.shadow.offset, Vector::new(0.0, 10.0));
    assert_eq!(light.shadow.blur_radius, 24.0);
}

#[test]
fn inspector_style_reuses_panel_shape_and_overrides_background() {
    let dark_panel = settings_panel_style(&Theme::Dark);
    let dark = inspector_style(&Theme::Dark);
    let light_panel = settings_panel_style(&Theme::Light);
    let light = inspector_style(&Theme::Light);

    assert_eq!(
        option_background_color(dark.background),
        Theme::Dark.extended_palette().background.base.color.scale_alpha(0.88)
    );
    assert_eq!(dark.border.color, dark_panel.border.color);
    assert_eq!(dark.border.width, dark_panel.border.width);
    assert_eq!(dark.shadow, dark_panel.shadow);
    assert_eq!(dark.border.radius.top_left, 20.0);

    assert_eq!(option_background_color(light.background), Color::from_rgba8(255, 255, 255, 0.92));
    assert_eq!(light.border.color, light_panel.border.color);
    assert_eq!(light.border.width, light_panel.border.width);
    assert_eq!(light.shadow, light_panel.shadow);
    assert_eq!(light.border.radius.top_left, 20.0);
}

#[test]
fn value_card_style_covers_dark_and_light_branches() {
    let dark_theme = Theme::Dark;
    let light_theme = Theme::Light;
    let dark_palette = dark_theme.extended_palette();
    let dark = value_card_style(&dark_theme);
    let light = value_card_style(&light_theme);

    assert_eq!(
        option_background_color(dark.background),
        dark_palette.background.base.color.scale_alpha(0.62)
    );
    assert_eq!(dark.border.color, dark_palette.background.strong.color.scale_alpha(0.72));
    assert_eq!(dark.shadow.color, Color::BLACK.scale_alpha(0.04));
    assert_eq!(dark.shadow.offset, Vector::new(0.0, 2.0));
    assert_eq!(dark.shadow.blur_radius, 8.0);

    assert_eq!(option_background_color(light.background), Color::WHITE.scale_alpha(0.90));
    assert_eq!(light.border.color, Color::from_rgba8(15, 23, 42, 0.06));
    assert_eq!(light.shadow.color, Color::BLACK.scale_alpha(0.02));
    assert_eq!(light.border.radius.top_left, 16.0);
}

#[test]
fn modal_styles_delegate_to_system_settings_common_styles() {
    let backdrop = modal_backdrop_style(&Theme::Dark);
    let expected_backdrop = settings_modal_backdrop_style(&Theme::Dark);
    let card = modal_card_style(&Theme::Light);
    let expected_card = settings_modal_card_style(&Theme::Light);

    assert_eq!(backdrop.background, expected_backdrop.background);
    assert_eq!(backdrop.border, expected_backdrop.border);
    assert_eq!(backdrop.shadow, expected_backdrop.shadow);

    assert_eq!(card.background, expected_card.background);
    assert_eq!(card.border, expected_card.border);
    assert_eq!(card.shadow, expected_card.shadow);
}

#[test]
fn validation_summary_style_uses_danger_palette_values() {
    let theme = Theme::Light;
    let palette = theme.extended_palette();
    let style = validation_summary_style(&theme);

    assert_eq!(
        option_background_color(style.background),
        palette.danger.weak.color.scale_alpha(0.20)
    );
    assert_eq!(style.border.color, palette.danger.base.color.scale_alpha(0.45));
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.radius.top_left, 16.0);
}

//! 用量页样式的测试，覆盖卡片、表格、图例和交互状态。
//! 颜色与边框断言分别检查明暗主题的层级和语义强调。
use iced::{Background, Color, Theme, Vector};

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected solid color background, got {other:?}"),
    }
}

#[test]
fn card_style_uses_dark_theme_palette_and_elevation() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();
    let style = super::card_style(&theme);

    assert_eq!(background_color(style.background), palette.background.base.color);
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.color, palette.background.strong.color);
    assert_eq!(style.border.radius, 12.0.into());
    assert_eq!(style.shadow.color, Color::BLACK.scale_alpha(0.10));
    assert_eq!(style.shadow.offset, Vector::new(0.0, 10.0));
    assert_eq!(style.shadow.blur_radius, 30.0);
}

#[test]
fn card_style_tracks_light_theme_palette_without_changing_shadow_shape() {
    let theme = Theme::Light;
    let palette = theme.extended_palette();
    let style = super::card_style(&theme);

    assert_eq!(background_color(style.background), palette.background.base.color);
    assert_eq!(style.border.color, palette.background.strong.color);
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.radius, 12.0.into());
    assert_eq!(style.shadow.color, Color::BLACK.scale_alpha(0.10));
    assert_eq!(style.shadow.offset, Vector::new(0.0, 10.0));
    assert_eq!(style.shadow.blur_radius, 30.0);
}

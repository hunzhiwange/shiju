//! 验证 Git 面板共享按钮、图标与提示组件的主题样式映射。
//! 使用精确调色板值覆盖各交互状态，并锁定明暗主题判定阈值。

use iced::theme::Palette;
use iced::widget::button;
use iced::{Background, Color, Theme};

use crate::app::Message;
use crate::app::assets::Icon;

use super::shared::{
    compact_outlined_button_style, disabled_button_style, disabled_icon_svg,
    header_plain_glyph_button_style, icon_svg, icon_svg_sized, outlined_button_style,
    small_plain_icon_button_style, subtle_button_style, themed_icon_svg, with_tooltip,
};

fn radius(style: &button::Style) -> f32 {
    style.border.radius.top_left
}

#[test]
fn svg_helpers_accept_default_and_custom_sizes() {
    let _ = icon_svg(Icon::Plus);
    let _ = icon_svg_sized(Icon::Trash, 22.0);
    let _ = disabled_icon_svg(Icon::Image, 15.0);
    let _ = themed_icon_svg(Icon::FileText, 10.0);
}

#[test]
fn with_tooltip_builds_compact_and_regular_variants() {
    let _: iced::Element<'static, Message> =
        with_tooltip(iced::widget::text("regular"), "regular tip".into(), false, 6.0);
    let _: iced::Element<'static, Message> =
        with_tooltip(iced::widget::text("compact"), "compact tip".into(), true, 4.0);
}

#[test]
fn outlined_button_style_maps_palette_colors_and_disabled_fallback() {
    let theme = Theme::Light;
    let palette = theme.extended_palette();
    let active = outlined_button_style(&theme, button::Status::Active, 8.0);
    let hovered = outlined_button_style(&theme, button::Status::Hovered, 8.0);
    let pressed = outlined_button_style(&theme, button::Status::Pressed, 8.0);
    let disabled = outlined_button_style(&theme, button::Status::Disabled, 8.0);

    assert!(active.background.is_none());
    assert_eq!(hovered.background, Some(Background::Color(palette.background.weak.color)));
    assert_eq!(pressed.background, Some(Background::Color(palette.background.strong.color)));
    assert!(disabled.background.is_none());
    assert_eq!(active.border.width, 1.0);
    assert_eq!(active.border.color, palette.background.strong.color);
    assert_eq!(radius(&active), 8.0);
}

#[test]
fn compact_outlined_button_style_uses_exact_light_palette() {
    let theme = Theme::Light;
    let palette = theme.extended_palette();
    let active = compact_outlined_button_style(&theme, button::Status::Active, 7.0);
    let hovered = compact_outlined_button_style(&theme, button::Status::Hovered, 7.0);
    let pressed = compact_outlined_button_style(&theme, button::Status::Pressed, 7.0);
    let disabled = compact_outlined_button_style(&theme, button::Status::Disabled, 7.0);

    assert!(active.background.is_none());
    assert_eq!(
        hovered.background,
        Some(Background::Color(palette.background.weak.color.scale_alpha(0.5)))
    );
    assert_eq!(
        pressed.background,
        Some(Background::Color(palette.background.strong.color.scale_alpha(0.72)))
    );
    assert!(disabled.background.is_none());
    assert_eq!(active.border.color, palette.background.strong.color);
    assert_eq!(active.text_color, palette.background.strong.text);
    assert_eq!(radius(&active), 7.0);
}

#[test]
fn compact_outlined_button_style_uses_exact_dark_palette() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();
    let active = compact_outlined_button_style(&theme, button::Status::Active, 7.0);
    let hovered = compact_outlined_button_style(&theme, button::Status::Hovered, 7.0);
    let pressed = compact_outlined_button_style(&theme, button::Status::Pressed, 7.0);
    let disabled = compact_outlined_button_style(&theme, button::Status::Disabled, 7.0);

    assert_eq!(
        active.background,
        Some(Background::Color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.06)))
    );
    assert_eq!(
        hovered.background,
        Some(Background::Color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.12)))
    );
    assert_eq!(
        pressed.background,
        Some(Background::Color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.20)))
    );
    assert_eq!(disabled.background, active.background);
    assert_eq!(active.border.color, Color::from_rgba8(0xA8, 0xB0, 0xBA, 0.28));
    assert_eq!(active.text_color, palette.background.strong.text);
    assert_eq!(radius(&active), 7.0);
}

#[test]
fn disabled_button_style_uses_theme_palette() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let style = disabled_button_style(&theme, 6.0);

        assert_eq!(style.background, Some(Background::Color(palette.background.weak.color)));
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.color, palette.background.strong.color);
        assert_eq!(style.text_color, palette.background.weak.text);
        assert_eq!(radius(&style), 6.0);
    }
}

#[test]
fn subtle_button_style_maps_every_status() {
    let active = subtle_button_style(button::Status::Active, 5.0);
    let hovered = subtle_button_style(button::Status::Hovered, 5.0);
    let pressed = subtle_button_style(button::Status::Pressed, 5.0);
    let disabled = subtle_button_style(button::Status::Disabled, 5.0);

    assert_eq!(active.background, Some(Background::Color(Color::TRANSPARENT)));
    assert_eq!(hovered.background, Some(Background::Color(Color::from_rgba8(235, 235, 235, 1.0))));
    assert_eq!(pressed.background, Some(Background::Color(Color::from_rgba8(225, 225, 225, 1.0))));
    assert_eq!(disabled.background, active.background);
    assert_eq!(radius(&pressed), 5.0);
}

#[test]
fn small_plain_icon_button_style_maps_pressed_and_fallback_statuses() {
    let theme = Theme::Light;
    let palette = theme.extended_palette();
    let active = small_plain_icon_button_style(&theme, button::Status::Active);
    let hovered = small_plain_icon_button_style(&theme, button::Status::Hovered);
    let pressed = small_plain_icon_button_style(&theme, button::Status::Pressed);
    let disabled = small_plain_icon_button_style(&theme, button::Status::Disabled);

    assert!(active.background.is_none());
    assert!(hovered.background.is_none());
    assert_eq!(
        pressed.background,
        Some(Background::Color(palette.background.strong.color.scale_alpha(0.18)))
    );
    assert!(disabled.background.is_none());
    assert_eq!(pressed.text_color, palette.background.strong.text);
    assert_eq!(radius(&pressed), 6.0);
}

#[test]
fn header_plain_glyph_button_style_uses_exact_light_palette() {
    let theme = Theme::Light;
    let palette = theme.extended_palette();
    let active = header_plain_glyph_button_style(&theme, button::Status::Active);
    let hovered = header_plain_glyph_button_style(&theme, button::Status::Hovered);
    let pressed = header_plain_glyph_button_style(&theme, button::Status::Pressed);
    let disabled = header_plain_glyph_button_style(&theme, button::Status::Disabled);

    assert!(active.background.is_none());
    assert_eq!(hovered.background, Some(Background::Color(palette.background.weak.color)));
    assert_eq!(pressed.background, Some(Background::Color(palette.background.strong.color)));
    assert!(disabled.background.is_none());
    assert_eq!(active.text_color, palette.background.strong.text);
    assert_eq!(hovered.text_color, palette.secondary.base.text);
    assert_eq!(pressed.text_color, palette.secondary.base.text);
    assert_eq!(radius(&pressed), 8.0);
}

#[test]
fn header_plain_glyph_button_style_uses_exact_dark_palette() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();
    let active = header_plain_glyph_button_style(&theme, button::Status::Active);
    let hovered = header_plain_glyph_button_style(&theme, button::Status::Hovered);
    let pressed = header_plain_glyph_button_style(&theme, button::Status::Pressed);
    let disabled = header_plain_glyph_button_style(&theme, button::Status::Disabled);

    assert!(active.background.is_none());
    assert_eq!(
        hovered.background,
        Some(Background::Color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.05)))
    );
    assert_eq!(
        pressed.background,
        Some(Background::Color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.09)))
    );
    assert!(disabled.background.is_none());
    assert_eq!(active.text_color, palette.background.strong.text);
    assert_eq!(hovered.text_color, palette.background.strong.text);
    assert_eq!(pressed.text_color, palette.background.strong.text);
    assert_eq!(radius(&pressed), 8.0);
}

#[test]
fn compact_style_uses_strict_dark_theme_brightness_threshold() {
    let threshold = Theme::custom(
        "threshold",
        Palette { background: Color::from_rgb(0.5, 0.5, 0.5), ..Palette::LIGHT },
    );
    let below_threshold = Theme::custom(
        "below-threshold",
        Palette { background: Color::from_rgb(0.499, 0.5, 0.5), ..Palette::LIGHT },
    );

    let threshold_style = compact_outlined_button_style(&threshold, button::Status::Active, 4.0);
    let below_threshold_style =
        compact_outlined_button_style(&below_threshold, button::Status::Active, 4.0);

    assert!(threshold_style.background.is_none());
    assert_eq!(
        below_threshold_style.background,
        Some(Background::Color(Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.06)))
    );
}

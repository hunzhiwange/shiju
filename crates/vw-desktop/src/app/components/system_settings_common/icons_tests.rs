//! 验证设置图标、提供商标识与提示按钮的尺寸和主题着色。
//! 未知提供商和零尺寸输入用于确认资源回退不会触发布局异常。

use super::{icon_btn, icon_svg, provider_logo_svg};
use crate::app::Message;
use crate::app::assets::Icon;
use iced::widget::{button, svg};
use iced::{Background, Element, Length, Theme};

fn background_color(background: Option<Background>) -> Option<iced::Color> {
    match background {
        Some(Background::Color(color)) => Some(color),
        Some(other) => panic!("expected solid color background, got {other:?}"),
        None => None,
    }
}

fn assert_size(element: &Element<'_, Message>, width: Length, height: Length) {
    let size = element.as_widget().size();

    assert_eq!(size.width, width);
    assert_eq!(size.height, height);
}

#[test]
fn icon_svg_uses_requested_square_size() {
    let element: Element<'_, Message> = icon_svg(Icon::Gear, 18.0).into();

    assert_size(&element, Length::Fixed(18.0), Length::Fixed(18.0));
}

#[test]
fn provider_logo_svg_uses_requested_square_size() {
    let element: Element<'_, Message> = provider_logo_svg("openai", 22.0).into();

    assert_size(&element, Length::Fixed(22.0), Length::Fixed(22.0));
}

#[test]
fn provider_logo_svg_accepts_unknown_provider_ids() {
    let element: Element<'_, Message> = provider_logo_svg("unknown-provider", 1.0).into();

    assert_size(&element, Length::Fixed(1.0), Length::Fixed(1.0));
}

#[test]
fn icon_svg_accepts_zero_size_without_clamping() {
    let element: Element<'_, Message> = icon_svg(Icon::Gear, 0.0).into();

    assert_size(&element, Length::Fixed(0.0), Length::Fixed(0.0));
}

#[test]
fn icon_button_wraps_content_in_tooltip_when_enabled() {
    let element = icon_btn(Icon::Gear, "Settings", Some(Message::None));

    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn icon_button_wraps_content_in_tooltip_when_disabled() {
    let element = icon_btn(Icon::QuestionCircle, "Help", None);

    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn icon_button_keeps_shrink_size_inside_tooltip() {
    let element = icon_btn(Icon::Gear, "Settings", Some(Message::None));

    assert_size(&element, Length::Shrink, Length::Shrink);
}

#[test]
fn provider_logo_svg_style_only_tints_dark_theme() {
    let dark = super::icons::provider_logo_svg_style(&Theme::Dark, svg::Status::Idle);
    let light = super::icons::provider_logo_svg_style(&Theme::Light, svg::Status::Hovered);

    assert_eq!(dark.color, Some(Theme::Dark.palette().text));
    assert_eq!(light.color, None);
}

#[test]
fn icon_button_svg_style_uses_theme_text_color() {
    let dark = super::icons::icon_btn_svg_style(&Theme::Dark, svg::Status::Idle);
    let light = super::icons::icon_btn_svg_style(&Theme::Light, svg::Status::Hovered);

    assert_eq!(dark.color, Some(Theme::Dark.palette().text));
    assert_eq!(light.color, Some(Theme::Light.palette().text));
}

#[test]
fn icon_button_style_covers_interaction_statuses() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();

    let active = super::icons::icon_btn_style(&theme, button::Status::Active);
    let hovered = super::icons::icon_btn_style(&theme, button::Status::Hovered);
    let pressed = super::icons::icon_btn_style(&theme, button::Status::Pressed);
    let disabled = super::icons::icon_btn_style(&theme, button::Status::Disabled);

    assert_eq!(background_color(active.background), None);
    assert_eq!(background_color(hovered.background), Some(palette.background.weak.color));
    assert_eq!(background_color(pressed.background), Some(palette.background.strong.color));
    assert_eq!(background_color(disabled.background), None);
    assert_eq!(active.border.width, 0.0);
    assert_eq!(active.border.radius.top_left, 8.0);
    assert_eq!(active.text_color, theme.palette().text);
}

#[test]
fn icon_button_tooltip_style_matches_theme_surface() {
    let theme = Theme::Light;
    let style = super::icons::icon_btn_tooltip_style(&theme);

    assert_eq!(style.text_color, Some(theme.palette().text));
    assert_eq!(background_color(style.background), Some(theme.palette().background));
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.color, theme.extended_palette().background.strong.color);
    assert_eq!(style.border.radius.top_left, 8.0);
    assert_eq!(style.shadow, iced::Shadow::default());
    assert!(!style.snap);
}

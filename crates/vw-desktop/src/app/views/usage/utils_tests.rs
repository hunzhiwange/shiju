//! 用量页共享工具的测试，覆盖主题样式、控件结构和时间金额格式化。
//! 用例包含溢出时间戳、负金额与不同交互状态，固定展示边界。
use crate::app::Message;
use crate::app::assets::Icon;
use iced::widget::button;
use iced::{Background, Color, Element, Length, Theme};

fn assert_size(element: &Element<'_, Message>, width: Length, height: Length) {
    let size = element.as_widget().size();

    assert_eq!(size.width, width);
    assert_eq!(size.height, height);
}

fn assert_color_close(actual: Color, expected: Color) {
    let epsilon = 0.000_01;

    assert!((actual.r - expected.r).abs() <= epsilon, "red differs: {actual:?} vs {expected:?}");
    assert!((actual.g - expected.g).abs() <= epsilon, "green differs: {actual:?} vs {expected:?}");
    assert!((actual.b - expected.b).abs() <= epsilon, "blue differs: {actual:?} vs {expected:?}");
    assert!((actual.a - expected.a).abs() <= epsilon, "alpha differs: {actual:?} vs {expected:?}");
}

fn background_color(style: iced::widget::container::Style) -> Color {
    match style.background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}

fn button_background(style: iced::widget::button::Style) -> Option<Color> {
    match style.background {
        Some(Background::Color(color)) => Some(color),
        None => None,
        other => panic!("expected color or no background, got {other:?}"),
    }
}

#[test]
fn text_style_helpers_follow_theme_palette() {
    let theme = Theme::Dark;

    assert_color_close(
        super::muted_label_text_style(&theme).color.expect("muted text color"),
        theme.extended_palette().background.weak.text.scale_alpha(0.9),
    );
    assert_eq!(super::primary_text_style(&theme).color, Some(theme.palette().text));
}

#[test]
fn icon_and_tooltip_style_helpers_cover_theme_and_status_branches() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();

    assert_eq!(super::icon_svg_style(&theme).color, Some(theme.palette().text));

    let active = super::icon_button_style(&theme, button::Status::Active);
    assert_eq!(button_background(active), None);

    let hovered = super::icon_button_style(&theme, button::Status::Hovered);
    assert_eq!(button_background(hovered), Some(palette.background.weak.color));

    let pressed = super::icon_button_style(&theme, button::Status::Pressed);
    assert_eq!(button_background(pressed), Some(palette.background.strong.color));

    let disabled = super::icon_button_style(&theme, button::Status::Disabled);
    assert_eq!(button_background(disabled), None);
    assert_eq!(disabled.border.width, 0.0);
    assert_eq!(disabled.border.radius, 6.0.into());
    assert_eq!(disabled.text_color, theme.palette().text);

    let tooltip = super::tooltip_container_style(&theme);
    assert_color_close(background_color(tooltip), theme.palette().background);
    assert_eq!(tooltip.text_color, Some(theme.palette().text));
    assert_eq!(tooltip.border.width, 1.0);
    assert_eq!(tooltip.border.color, palette.background.strong.color);
    assert_eq!(tooltip.border.radius, 8.0.into());
    assert_eq!(tooltip.shadow, iced::Shadow::default());
    assert!(!tooltip.snap);
}

#[test]
fn section_title_kv_icon_svg_and_icon_button_build_expected_structure() {
    let title = super::section_title("用量");
    let kv = super::kv("总 token", "42".to_string());
    let icon: Element<'_, Message> = super::icon_svg(Icon::Copy).into();
    let button = super::icon_btn(Icon::FolderOpen, "打开", Message::None);

    assert_size(&title, Length::Shrink, Length::Shrink);
    assert_eq!(kv.as_widget().children().len(), 3);
    assert_size(&icon, Length::Fixed(14.0), Length::Fixed(14.0));
    assert_size(&button, Length::Shrink, Length::Fixed(24.0));
    assert_eq!(button.as_widget().children().len(), 2);
}

#[test]
fn fmt_ms_formats_epoch_minutes_truncates_subseconds_and_rejects_overflow() {
    assert_eq!(super::fmt_ms(0), "1970-01-01 00:00");
    assert_eq!(super::fmt_ms(1_234), "1970-01-01 00:00");
    assert_eq!(super::fmt_ms(65_000), "1970-01-01 00:01");
    assert_eq!(super::fmt_ms(u64::MAX), "暂无");
}

#[test]
fn fmt_usd_rounds_to_four_decimals() {
    assert_eq!(super::fmt_usd(0.00123456), "US$0.0012");
    assert_eq!(super::fmt_usd(12.5), "US$12.5000");
    assert_eq!(super::fmt_usd(0.99996), "US$1.0000");
}

#[test]
fn fmt_usd_preserves_zero_and_negative_signs() {
    assert_eq!(super::fmt_usd(0.0), "US$0.0000");
    assert_eq!(super::fmt_usd(-12.5), "US$-12.5000");
    assert_eq!(super::fmt_usd(-0.0), "US$-0.0000");
}

use iced::widget::tooltip::Position as TooltipPosition;
use iced::widget::{button, text};
use iced::{Background, Color, Element, Length, Theme};

use super::widgets::{
    color_with_alpha, icon_button, icon_svg, icon_svg_style, icon_toggle_button,
    icon_toggle_button_opt, icon_toggle_button_style, icon_tooltip_style, menu_btn,
    menu_button_style, menu_container, menu_container_style, menu_item_btn, menu_item_button_style,
    menu_item_icon_btn, menu_separator, menu_separator_style,
};
use crate::app::Message;
use crate::app::assets::Icon;
use crate::app::message::view::MenuType;

fn assert_size(element: &Element<'_, Message>, width: Length, height: Length) {
    let size = element.as_widget().size();

    assert_eq!(size.width, width);
    assert_eq!(size.height, height);
}

#[test]
fn color_with_alpha_preserves_rgb_and_replaces_alpha() {
    let color = Color { r: 0.2, g: 0.4, b: 0.6, a: 0.8 };

    assert_eq!(color_with_alpha(color, 0.35), Color { a: 0.35, ..color });
}

#[test]
fn color_with_alpha_accepts_transparent_and_opaque_boundaries() {
    let color = Color::from_rgb(0.1, 0.3, 0.5);

    assert_eq!(color_with_alpha(color, 0.0), Color { a: 0.0, ..color });
    assert_eq!(color_with_alpha(color, 1.0), Color { a: 1.0, ..color });
}

#[test]
fn color_with_alpha_preserves_out_of_range_alpha_without_clamping() {
    let color = Color::from_rgb(0.1, 0.3, 0.5);

    assert_eq!(color_with_alpha(color, -0.25).a, -0.25);
    assert_eq!(color_with_alpha(color, 1.25).a, 1.25);
}

#[test]
fn icon_svg_uses_compact_toolbar_size() {
    let element: Element<'_, Message> = icon_svg(Icon::Gear).into();

    assert_size(&element, Length::Fixed(14.0), Length::Fixed(14.0));
}

#[test]
fn icon_svg_style_distinguishes_enabled_and_disabled() {
    let theme = Theme::Dark;

    assert_eq!(icon_svg_style(&theme, true).color, Some(theme.palette().text));
    assert_eq!(icon_svg_style(&theme, false).color, Some(theme.palette().text.scale_alpha(0.35)));
}

#[test]
fn icon_button_builds_tooltip_wrapped_button() {
    let element = icon_button(Icon::Gear, "Settings", TooltipPosition::Bottom, Message::None);

    assert_size(&element, Length::Shrink, Length::Fixed(24.0));
    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn icon_toggle_button_builds_active_tooltip_wrapped_button() {
    let element = icon_toggle_button(
        Icon::LayoutSidebar,
        "Toggle sidebar",
        TooltipPosition::Bottom,
        true,
        Message::None,
    );

    assert_size(&element, Length::Shrink, Length::Fixed(24.0));
    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn icon_toggle_button_opt_without_message_still_renders_disabled_button() {
    let element = icon_toggle_button_opt(
        Icon::LayoutSidebarReverse,
        "Toggle panel",
        TooltipPosition::Bottom,
        false,
        None,
    );

    assert_size(&element, Length::Shrink, Length::Fixed(24.0));
    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn icon_toggle_button_style_covers_disabled_hovered_pressed_active_and_idle() {
    let theme = Theme::Light;
    let text = theme.palette().text;

    let disabled = icon_toggle_button_style(&theme, button::Status::Hovered, false, true);
    assert_eq!(disabled.background, None);
    assert_eq!(disabled.text_color, text.scale_alpha(0.35));

    let hovered = icon_toggle_button_style(&theme, button::Status::Hovered, true, false);
    assert_eq!(hovered.background, Some(Background::Color(color_with_alpha(text, 0.14))));

    let active_hovered = icon_toggle_button_style(&theme, button::Status::Hovered, true, true);
    assert_eq!(active_hovered.background, Some(Background::Color(color_with_alpha(text, 0.14))));

    let pressed = icon_toggle_button_style(&theme, button::Status::Pressed, true, false);
    assert_eq!(pressed.background, Some(Background::Color(color_with_alpha(text, 0.22))));

    let active = icon_toggle_button_style(&theme, button::Status::Active, true, true);
    assert_eq!(active.background, Some(Background::Color(color_with_alpha(text, 0.10))));

    let idle = icon_toggle_button_style(&theme, button::Status::Active, true, false);
    assert_eq!(idle.background, None);
    assert_eq!(idle.text_color, text);
}

#[test]
fn icon_toggle_button_style_pressed_state_precedes_active_state() {
    let theme = Theme::Dark;
    let text = theme.palette().text;

    let style = icon_toggle_button_style(&theme, button::Status::Pressed, true, true);

    assert_eq!(style.background, Some(Background::Color(color_with_alpha(text, 0.22))));
    assert_eq!(style.border.width, 0.0);
    assert_eq!(style.border.color, Color::TRANSPARENT);
}

#[test]
fn icon_tooltip_style_uses_theme_palette() {
    let theme = Theme::Dark;
    let style = icon_tooltip_style(&theme);

    assert_eq!(style.text_color, Some(theme.palette().text));
    assert_eq!(style.background, Some(Background::Color(theme.palette().background)));
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.color, theme.extended_palette().background.strong.color);
}

#[test]
fn menu_btn_fills_top_bar_height_for_inactive_menu() {
    let element = menu_btn("File", MenuType::File, None);

    assert_size(&element, Length::Shrink, Length::Fill);
}

#[test]
fn menu_btn_fills_top_bar_height_for_active_menu() {
    let element = menu_btn("View", MenuType::View, Some(MenuType::View));

    assert_size(&element, Length::Shrink, Length::Fill);
}

#[test]
fn menu_button_style_covers_active_hovered_and_idle_states() {
    let theme = Theme::Light;
    let text = theme.palette().text;
    let highlighted = Some(Background::Color(color_with_alpha(text, 0.10)));

    assert_eq!(menu_button_style(&theme, button::Status::Active, true).background, highlighted);
    assert_eq!(menu_button_style(&theme, button::Status::Hovered, false).background, highlighted);

    let idle = menu_button_style(&theme, button::Status::Active, false);
    assert_eq!(idle.background, Some(Background::Color(Color::TRANSPARENT)));
    assert_eq!(idle.text_color, text);
}

#[test]
fn menu_button_style_pressed_inactive_menu_remains_transparent() {
    let theme = Theme::Dark;
    let style = menu_button_style(&theme, button::Status::Pressed, false);

    assert_eq!(style.background, Some(Background::Color(Color::TRANSPARENT)));
    assert_eq!(style.text_color, theme.palette().text);
}

#[test]
fn menu_container_uses_fixed_menu_width() {
    let content: Element<'_, Message> = text("content").into();
    let element = menu_container(content);

    assert_size(&element, Length::Fixed(220.0), Length::Shrink);
}

#[test]
fn menu_container_style_uses_translucent_background_and_shadow() {
    let theme = Theme::Dark;
    let style = menu_container_style(&theme);

    assert_eq!(style.text_color, Some(theme.palette().text));
    assert_eq!(
        style.background,
        Some(Background::Color(color_with_alpha(theme.palette().background, 0.96)))
    );
    assert_eq!(
        style.border.color,
        theme.extended_palette().background.strong.color.scale_alpha(0.70)
    );
    assert_eq!(style.shadow.color, Color::BLACK.scale_alpha(0.18));
    assert_eq!(style.shadow.offset, iced::Vector::new(0.0, 10.0));
    assert_eq!(style.shadow.blur_radius, 26.0);
}

#[test]
fn menu_item_btn_with_message_fills_available_width() {
    let element = menu_item_btn("New session", Some("Cmd+N"), Some(Message::None));

    assert_size(&element, Length::Fill, Length::Shrink);
}

#[test]
fn menu_item_btn_without_message_fills_available_width() {
    let element = menu_item_btn("Unavailable", None, None);

    assert_size(&element, Length::Fill, Length::Shrink);
}

#[test]
fn menu_item_button_style_covers_disabled_hovered_and_idle_states() {
    let theme = Theme::Light;

    let disabled = menu_item_button_style(&theme, button::Status::Hovered, false);
    assert_eq!(disabled.background, None);
    assert_eq!(disabled.text_color, theme.palette().text.scale_alpha(0.35));

    let hovered = menu_item_button_style(&theme, button::Status::Hovered, true);
    assert_eq!(
        hovered.background,
        Some(Background::Color(theme.extended_palette().primary.base.color))
    );
    assert_eq!(hovered.text_color, Color::WHITE);

    let idle = menu_item_button_style(&theme, button::Status::Active, true);
    assert_eq!(idle.background, None);
    assert_eq!(idle.text_color, theme.palette().text);
}

#[test]
fn menu_item_button_style_pressed_enabled_item_uses_idle_colors() {
    let theme = Theme::Dark;
    let style = menu_item_button_style(&theme, button::Status::Pressed, true);

    assert_eq!(style.background, None);
    assert_eq!(style.text_color, theme.palette().text);
}

#[test]
fn menu_separator_fills_width_and_shrinks_height() {
    let element = menu_separator();

    assert_size(&element, Length::Shrink, Length::Shrink);
}

#[test]
fn menu_separator_style_uses_strong_translucent_background() {
    let theme = Theme::Dark;

    assert_eq!(
        menu_separator_style(&theme).background,
        Some(Background::Color(theme.extended_palette().background.strong.color.scale_alpha(0.70)))
    );
}

#[test]
fn menu_item_icon_btn_with_message_fills_available_width() {
    let icon: Element<'_, Message> = text("i").into();
    let element = menu_item_icon_btn(icon, "Open", Some("Cmd+O"), Some(Message::None));

    assert_size(&element, Length::Fill, Length::Shrink);
}

#[test]
fn menu_item_icon_btn_without_message_fills_available_width() {
    let icon: Element<'_, Message> = button(text("i")).into();
    let element = menu_item_icon_btn(icon, "Open", None, None);

    assert_size(&element, Length::Fill, Length::Shrink);
}

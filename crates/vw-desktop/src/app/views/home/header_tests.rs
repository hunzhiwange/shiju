use iced::widget::button;
use iced::{Background, Color, Element, Theme, Vector};

use crate::app::{App, Message};

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("header_tests"));
}

#[test]
fn render_builds_home_header_from_default_state() {
    let app = App::new().0;

    keep_element(super::render(&app));
}

#[test]
fn render_keeps_three_header_sections_and_two_project_actions() {
    let app = App::new().0;
    let header = super::render(&app);

    assert_eq!(header.as_widget().size().width, iced::Length::Fill);

    let root_children = header.as_widget().children();
    assert_eq!(root_children.len(), 3);
    assert_eq!(root_children[2].children.len(), 2);
}

#[test]
fn app_chip_builds_tooltip_with_supplied_action() {
    keep_element(super::app_chip(
        crate::app::assets::Icon::Grid1x2,
        "应用中心",
        Color::from_rgb8(0x35, 0x85, 0xD6),
        Message::None,
    ));
}

#[test]
fn home_apps_bar_scroll_changed_preserves_relative_offset() {
    for relative_x in [0.0, 0.375, 1.0] {
        match super::home_apps_bar_scroll_changed(relative_x) {
            Message::View(crate::app::message::ViewMessage::HomeAppsBarScrollChanged(actual)) => {
                assert_eq!(actual, relative_x);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }
}

#[test]
fn home_apps_bar_scroll_changed_forwards_out_of_range_offsets_without_clamping() {
    for relative_x in [-0.25, 1.25] {
        match super::home_apps_bar_scroll_changed(relative_x) {
            Message::View(crate::app::message::ViewMessage::HomeAppsBarScrollChanged(actual)) => {
                assert_eq!(actual, relative_x);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }
}

#[test]
fn header_container_style_removes_border_and_rounds_panel() {
    for theme in [Theme::Light, Theme::Dark] {
        let style = super::header_container_style(&theme);

        assert_eq!(style.border.width, 0.0);
        assert_eq!(style.border.color, Color::TRANSPARENT);
        assert_eq!(style.border.radius.top_left, 20.0);
    }
}

#[test]
fn app_chip_icon_style_uses_accent_color() {
    let accent = Color::from_rgb8(0x35, 0x85, 0xD6);

    assert_eq!(super::app_chip_icon_style(accent).color, Some(accent));
}

#[test]
fn app_chip_button_style_covers_light_interaction_states() {
    let theme = Theme::Light;
    let palette = theme.extended_palette();
    let accent = Color::from_rgb8(0x35, 0x85, 0xD6);
    let active = super::app_chip_button_style(&theme, button::Status::Active, accent);
    let hovered = super::app_chip_button_style(&theme, button::Status::Hovered, accent);
    let pressed = super::app_chip_button_style(&theme, button::Status::Pressed, accent);
    let disabled = super::app_chip_button_style(&theme, button::Status::Disabled, accent);

    assert_eq!(background_color(active.background), Color::WHITE.scale_alpha(0.82));
    assert_eq!(background_color(hovered.background), Color::WHITE.scale_alpha(0.94));
    assert_eq!(
        background_color(pressed.background),
        palette.background.weak.color.scale_alpha(0.92)
    );
    assert_eq!(background_color(disabled.background), Color::WHITE.scale_alpha(0.82));
    assert_eq!(hovered.border.color, accent.scale_alpha(0.34));
    assert_eq!(active.border.color, palette.background.strong.color.scale_alpha(0.72));
    assert_eq!(active.border.radius.top_left, 999.0);
    assert_eq!(active.shadow.color, Color::BLACK.scale_alpha(0.05));
    assert_eq!(active.text_color, theme.palette().text);
}

#[test]
fn app_chip_button_style_covers_dark_interaction_states() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();
    let accent = Color::from_rgb8(0x35, 0x85, 0xD6);
    let active = super::app_chip_button_style(&theme, button::Status::Active, accent);
    let hovered = super::app_chip_button_style(&theme, button::Status::Hovered, accent);
    let pressed = super::app_chip_button_style(&theme, button::Status::Pressed, accent);

    assert_eq!(
        background_color(active.background),
        palette.background.base.color.scale_alpha(0.60)
    );
    assert_eq!(
        background_color(hovered.background),
        palette.background.weak.color.scale_alpha(0.86)
    );
    assert_eq!(
        background_color(pressed.background),
        palette.background.strong.color.scale_alpha(0.92)
    );
    assert_eq!(active.shadow.color, Color::BLACK.scale_alpha(0.14));
    assert_eq!(active.shadow.offset, Vector::new(0.0, 8.0));
    assert_eq!(active.shadow.blur_radius, 16.0);
}

#[test]
fn app_chip_tooltip_style_keeps_high_contrast_surface() {
    let style = super::app_chip_tooltip_style();

    assert_eq!(background_color(style.background), Color::from_rgba8(24, 24, 24, 0.96));
    assert_eq!(style.text_color, Some(Color::WHITE));
    assert_eq!(style.border.width, 0.0);
    assert_eq!(style.border.color, Color::TRANSPARENT);
    assert_eq!(style.border.radius.top_left, 8.0);
    assert_eq!(style.shadow.color, Color::BLACK.scale_alpha(0.40));
    assert_eq!(style.shadow.offset, Vector::new(0.0, 6.0));
    assert_eq!(style.shadow.blur_radius, 18.0);
    assert!(!style.snap);
}

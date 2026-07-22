use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::widget::button;
use iced::{Background, Color, Font, Pixels, Point, Rectangle, Size, Theme};

use crate::app::message::ProjectMessage;
use crate::app::{App, Message};

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        _ => panic!("expected a color background"),
    }
}

#[test]
fn session_menu_button_style_maps_interaction_states_to_palette() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let active = super::session_menu_button_style(&theme, button::Status::Active);
        let hovered = super::session_menu_button_style(&theme, button::Status::Hovered);
        let pressed = super::session_menu_button_style(&theme, button::Status::Pressed);
        let disabled = super::session_menu_button_style(&theme, button::Status::Disabled);

        assert_eq!(background_color(active.background), Color::TRANSPARENT);
        assert_eq!(background_color(hovered.background), palette.background.weak.color);
        assert_eq!(background_color(pressed.background), palette.background.strong.color);
        assert_eq!(background_color(disabled.background), Color::TRANSPARENT);
        assert_eq!(active.text_color, theme.palette().text);
        assert_eq!(active.border.radius.top_left, 4.0);
    }
}

#[test]
fn session_menu_surface_styles_follow_theme_palette() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let separator = super::session_menu_separator_style(&theme);
        let container = super::session_menu_container_style(&theme);

        assert_eq!(background_color(separator.background), palette.background.strong.color);
        assert_eq!(background_color(container.background), palette.background.base.color);
        assert_eq!(container.border.width, 1.0);
        assert_eq!(container.border.color, palette.background.strong.color);
        assert_eq!(container.border.radius.top_left, 8.0);
        assert_eq!(container.shadow.color, Color::BLACK.scale_alpha(0.15));
        assert_eq!(container.shadow.offset.x, 0.0);
        assert_eq!(container.shadow.offset.y, 4.0);
        assert_eq!(container.shadow.blur_radius, 12.0);
    }
}

#[test]
fn session_menu_icon_and_text_styles_keep_expected_colors() {
    assert_eq!(super::session_menu_icon_style().color, Some(Color::from_rgb8(0x80, 0x80, 0x80)));

    for theme in [Theme::Light, Theme::Dark] {
        let icon_button = super::session_menu_icon_button_style(&theme);

        assert!(icon_button.background.is_none());
        assert_eq!(icon_button.border.radius.top_left, 4.0);
        assert_eq!(icon_button.text_color, theme.palette().text);
        assert_eq!(
            super::session_label_style(&theme).color,
            Some(theme.extended_palette().background.weak.text.scale_alpha(0.9))
        );
        assert_eq!(super::session_value_style(&theme).color, Some(theme.palette().text));
    }
}

#[test]
fn session_right_click_message_preserves_id_and_coordinates() {
    let message = super::session_right_click_message("session-42", Point::new(12.5, -3.0));

    match message {
        Message::Project(ProjectMessage::SessionRightClicked(id, x, y)) => {
            assert_eq!(id, "session-42");
            assert_eq!(x, 12.5);
            assert_eq!(y, -3.0);
        }
        _ => panic!("expected session right-click message"),
    }
}

#[test]
fn session_menu_button_and_menu_can_be_constructed() {
    let _button = super::session_menu_button("重命名", Message::None);
    let _menu = super::build_session_menu("session-1".to_string());
}

#[test]
fn kv_with_menu_uses_spacer_when_session_id_is_missing() {
    let (app, _task) = App::new();

    let _row = super::kv_with_menu(&app, "会话", "暂无".to_string(), None);
}

#[test]
fn kv_with_menu_builds_right_click_area_for_closed_menu() {
    let (app, _task) = App::new();

    let _row =
        super::kv_with_menu(&app, "会话", "Session".to_string(), Some("session-1".to_string()));
}

#[test]
fn kv_with_menu_wraps_button_in_overlay_when_menu_is_open() {
    let (mut app, _task) = App::new();
    app.session_menu_id = Some("session-1".to_string());
    app.session_menu_anchor = Some(Point::new(10.0, 20.0));

    let _row =
        super::kv_with_menu(&app, "会话", "Session".to_string(), Some("session-1".to_string()));
}

#[test]
fn kv_with_menu_uses_origin_when_open_menu_anchor_is_missing() {
    let (mut app, _task) = App::new();
    app.session_menu_id = Some("session-1".to_string());
    app.session_menu_anchor = None;

    let _row =
        super::kv_with_menu(&app, "会话", "Session".to_string(), Some("session-1".to_string()));
}

#[tokio::test(flavor = "multi_thread")]
async fn kv_with_menu_draws_icon_and_text_styles_in_dark_theme() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let (app, _task) = App::new();
    let mut element =
        super::kv_with_menu(&app, "会话", "Session".to_string(), Some("session-1".to_string()));
    let bounds = Size::new(480.0, 80.0);
    let limits = layout::Limits::new(Size::ZERO, bounds);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, &mut renderer, &limits);

    element.as_widget().draw(
        &tree,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::with_size(bounds),
    );
}

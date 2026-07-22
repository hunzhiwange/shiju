use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Background, Color, Font, Pixels, Rectangle, Size, Theme};

use crate::app::App;

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("chat_tests"));
}

fn render_chat_area(spacing: f32, corner_radius: f32, chat_content_pad: f32) {
    let (app, _task) = App::new();
    let element = super::chat_area(&app, spacing, corner_radius, chat_content_pad);

    std::hint::black_box(element);
}

#[test]
fn chat_area_builds_with_regular_layout_values() {
    render_chat_area(8.0, 12.0, 16.0);
}

#[test]
fn chat_area_builds_with_zero_layout_values() {
    render_chat_area(0.0, 0.0, 0.0);
}

#[test]
fn chat_area_builds_with_negative_radius() {
    render_chat_area(4.0, -2.0, 8.0);
}

#[tokio::test(flavor = "multi_thread")]
async fn chat_area_draws_captured_surface_style() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let (app, _task) = App::new();
    let mut element = super::chat_area(&app, 8.0, 12.0, 16.0);

    draw_element(&mut element, &mut renderer, &Theme::Dark);
}

#[test]
fn chat_area_style_uses_light_theme_surface() {
    let style = super::chat_area_style(&Theme::Light, 16.0);

    assert_eq!(style.background, Some(Background::Color(Color::from_rgba8(255, 255, 255, 0.985))));
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.color, Color::from_rgba8(224, 228, 236, 0.98));
    assert_eq!(style.border.radius.top_left, 16.0);
    assert_eq!(style.border.radius.top_right, 16.0);
    assert_eq!(style.border.radius.bottom_right, 16.0);
    assert_eq!(style.border.radius.bottom_left, 16.0);
}

#[test]
fn chat_area_style_uses_dark_theme_surface() {
    let style = super::chat_area_style(&Theme::Dark, 4.0);

    assert_eq!(style.background, Some(Background::Color(Color::from_rgba8(17, 18, 22, 0.985))));
    assert_eq!(style.border.width, 1.0);
    assert_ne!(style.border.color, Color::from_rgba8(224, 228, 236, 0.98));
    assert_eq!(style.border.radius.top_left, 4.0);
    assert_eq!(style.border.radius.top_right, 4.0);
    assert_eq!(style.border.radius.bottom_right, 4.0);
    assert_eq!(style.border.radius.bottom_left, 4.0);
}

#[test]
fn chat_area_style_preserves_fractional_radius() {
    let style = super::chat_area_style(&Theme::Light, 2.5);

    assert_eq!(style.border.radius.top_left, 2.5);
    assert_eq!(style.border.radius.top_right, 2.5);
    assert_eq!(style.border.radius.bottom_right, 2.5);
    assert_eq!(style.border.radius.bottom_left, 2.5);
}

#[test]
fn chat_area_style_uses_default_shadow() {
    let style = super::chat_area_style(&Theme::Light, 8.0);

    assert_eq!(style.shadow, iced::Shadow::default());
}

fn draw_element(
    element: &mut iced::Element<'_, crate::app::Message>,
    renderer: &mut iced::Renderer,
    theme: &Theme,
) {
    let bounds = Size::new(800.0, 600.0);
    let limits = layout::Limits::new(Size::ZERO, bounds);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
    let viewport = Rectangle::with_size(bounds);

    element.as_widget().draw(
        &tree,
        renderer,
        theme,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
}

//! 会话侧栏容器的测试，验证可用宽度缩放、负值钳制与圆角样式。
//! 绘制用例检查捕获样式，避免仅凭元素构建遗漏主题渲染问题。
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Font, Length, Pixels, Rectangle, Size, Theme};

use super::{
    project_sessions_panel_container, project_sessions_panel_container_style,
    project_sessions_panel_width,
};
use crate::app::App;

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("sessions_panel_tests"));
}

#[test]
fn project_sessions_panel_container_uses_scaled_available_width() {
    let (app, _task) = App::new();
    let element = project_sessions_panel_container(&app, 320.0, 80.0, 0.5, 12.0, None);
    let size = element.as_widget().size();

    assert_eq!(size.width, Length::Fixed(120.0));
    assert_eq!(size.height, Length::Fill);
}

#[test]
fn project_sessions_panel_container_clamps_negative_width_to_zero() {
    let (app, _task) = App::new();
    let element = project_sessions_panel_container(
        &app,
        80.0,
        120.0,
        1.25,
        8.0,
        Some("/tmp/project".to_string()),
    );
    let size = element.as_widget().size();

    assert_eq!(size.width, Length::Fixed(0.0));
    assert_eq!(size.height, Length::Fill);
}

#[tokio::test(flavor = "multi_thread")]
async fn project_sessions_panel_container_draws_captured_style() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let (app, _task) = App::new();
    let mut element = project_sessions_panel_container(&app, 320.0, 80.0, 0.5, 12.0, None);

    draw_element(&mut element, &mut renderer, &Theme::Dark);
}

#[test]
fn project_sessions_panel_width_scales_available_width() {
    let width = project_sessions_panel_width(420.0, 60.0, 0.25);

    assert_eq!(width, 90.0);
}

#[test]
fn project_sessions_panel_width_clamps_negative_available_width() {
    let width = project_sessions_panel_width(60.0, 420.0, 0.25);

    assert_eq!(width, 0.0);
}

#[test]
fn project_sessions_panel_width_clamps_negative_scaled_width() {
    let width = project_sessions_panel_width(420.0, 60.0, -0.25);

    assert_eq!(width, 0.0);
}

#[test]
fn project_sessions_panel_container_style_uses_corner_radius() {
    let style = project_sessions_panel_container_style(&Theme::Dark, 12.0);

    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.radius.top_left, 16.0);
}

#[test]
fn project_sessions_panel_container_style_keeps_light_theme_shadow() {
    let style = project_sessions_panel_container_style(&Theme::Light, 8.0);

    assert_eq!(style.border.radius.top_left, 12.0);
    assert_eq!(style.shadow.color, iced::Color::BLACK.scale_alpha(0.05));
}

fn draw_element(
    element: &mut iced::Element<'_, crate::app::Message>,
    renderer: &mut iced::Renderer,
    theme: &Theme,
) {
    let bounds = Size::new(320.0, 600.0);
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

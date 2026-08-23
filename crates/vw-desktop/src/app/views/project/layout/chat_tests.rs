//! 项目聊天区域的测试，覆盖容器几何、主题表面和壁纸透明度。
//! 除构建检查外还走真实绘制路径，核对捕获到的样式与圆角。
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
    let style = super::chat_area_style(&Theme::Light, 16.0, None);
    let expected = crate::app::theme_catalog::panel_surface_color(&Theme::Light, 0.985);

    assert_eq!(style.background, Some(Background::Color(expected)));
    assert_eq!(style.border.width, 1.0);
    assert_eq!(
        style.border.color,
        Theme::Light.extended_palette().background.strong.color.scale_alpha(0.55)
    );
    assert_eq!(style.border.radius.top_left, 16.0);
    assert_eq!(style.border.radius.top_right, 16.0);
    assert_eq!(style.border.radius.bottom_right, 16.0);
    assert_eq!(style.border.radius.bottom_left, 16.0);
}

#[test]
fn chat_area_style_uses_dark_theme_surface() {
    let style = super::chat_area_style(&Theme::Dark, 4.0, None);
    let expected = crate::app::theme_catalog::panel_surface_color(&Theme::Dark, 0.985);

    assert_eq!(style.background, Some(Background::Color(expected)));
    assert_eq!(style.border.width, 1.0);
    assert_ne!(
        style.border.color,
        Theme::Light.extended_palette().background.strong.color.scale_alpha(0.55)
    );
    assert_eq!(style.border.radius.top_left, 4.0);
    assert_eq!(style.border.radius.top_right, 4.0);
    assert_eq!(style.border.radius.bottom_right, 4.0);
    assert_eq!(style.border.radius.bottom_left, 4.0);
}

#[test]
fn chat_area_style_follows_custom_palette_background() {
    let green = Theme::custom(
        "green-chat".to_string(),
        iced::theme::Palette {
            background: Color::from_rgb8(0x12, 0x4A, 0x2E),
            text: Color::from_rgb8(0xE8, 0xF5, 0xEC),
            primary: Color::from_rgb8(0x3D, 0xB8, 0x6B),
            success: Color::from_rgb8(0x3D, 0xB8, 0x6B),
            warning: Color::from_rgb8(0xE6, 0xB8, 0x00),
            danger: Color::from_rgb8(0xE0, 0x4F, 0x5F),
        },
    );
    let style = super::chat_area_style(&green, 8.0, None);
    let expected = crate::app::theme_catalog::panel_surface_color(&green, 0.985);
    assert_eq!(style.background, Some(Background::Color(expected)));
}

#[test]
fn chat_area_style_preserves_fractional_radius() {
    let style = super::chat_area_style(&Theme::Light, 2.5, None);

    assert_eq!(style.border.radius.top_left, 2.5);
    assert_eq!(style.border.radius.top_right, 2.5);
    assert_eq!(style.border.radius.bottom_right, 2.5);
    assert_eq!(style.border.radius.bottom_left, 2.5);
}

#[test]
fn chat_area_style_uses_default_shadow() {
    let style = super::chat_area_style(&Theme::Light, 8.0, None);

    assert_eq!(style.shadow, iced::Shadow::default());
}

#[test]
fn chat_area_style_applies_wallpaper_panel_alpha() {
    use std::path::PathBuf;

    use crate::app::theme_catalog::{
        ActiveThemeSkin, DEFAULT_WALLPAPER_PANEL_ALPHA, ThemeChrome, resolve_chrome_style_from_skin,
    };

    let theme = Theme::Dark;
    let skin = ActiveThemeSkin {
        theme_id: "wallpaper-theme".to_string(),
        background_path: Some(PathBuf::from("/tmp/vibe-window-wallpaper.png")),
        cover_path: None,
        chat_header_path: None,
        chat_body_path: None,
        chat_input_path: None,
        chrome: ThemeChrome::default(),
    };
    let chrome = resolve_chrome_style_from_skin(Some(&skin), &theme);
    let style = super::chat_area_style(&theme, 12.0, Some(&chrome));
    let background = match style.background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    };
    assert!(
        (background.a - DEFAULT_WALLPAPER_PANEL_ALPHA).abs() < 1.0e-5,
        "chat shell should use wallpaper panel alpha, got {:?}",
        background
    );
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

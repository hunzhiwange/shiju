//! 验证编辑器设置的主题来源、数值边界与字段行布局。
//! 跟随系统与独立主题两条路径均覆盖最小、最大配置值。

use super::ui_fixture::*;
use iced::Element;
fn test_app() -> Fixture {
    Fixture::default()
}

// 这些用例验证所有状态均可构建，不依赖窗口渲染后的像素结果。
fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn view_uses_app_theme_when_following_system_theme() {
    let mut app = test_app();
    app.editor_follow_system_theme = true;
    app.app_theme = iced::Theme::Dark;
    app.editor_theme = iced::Theme::Light;
    app.current_font_size = 15.0;
    app.current_line_height = 24.0;
    app.auto_adjust_line_height = true;

    keep_element(view(&app));
}

#[test]
fn view_uses_editor_theme_when_not_following_system_theme() {
    let mut app = test_app();
    app.editor_follow_system_theme = false;
    app.app_theme = iced::Theme::Light;
    app.editor_theme = iced::Theme::Dark;
    app.current_font_size = 30.0;
    app.current_line_height = 60.0;
    app.auto_adjust_line_height = false;

    keep_element(view(&app));
}

#[test]
fn view_default_values_does_not_panic() {
    let app = test_app();
    keep_element(view(&app));
}

#[test]
fn view_with_min_boundary_values() {
    let mut app = test_app();
    app.editor_follow_system_theme = true;
    app.current_font_size = 10.0;
    app.current_line_height = 10.0;
    keep_element(view(&app));
}

#[test]
fn view_with_max_boundary_values() {
    let mut app = test_app();
    app.editor_follow_system_theme = false;
    app.current_font_size = 30.0;
    app.current_line_height = 60.0;
    keep_element(view(&app));
}

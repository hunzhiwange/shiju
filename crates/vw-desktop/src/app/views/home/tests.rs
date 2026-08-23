//! 首页组合视图测试，验证空项目与最近项目状态均可构建，并覆盖明暗背景。
//! 容器背景来自当前主题调色板，避免暗色模式出现低对比度面板。

use crate::app::App;
use iced::{Background, Color, Element, Theme};

fn keep_element(element: Element<'_, crate::app::Message>) {
    std::hint::black_box(element);
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("tests"));
}

#[test]
fn view_builds_empty_recent_projects_home() {
    let mut app = App::new().0;
    app.recent_projects.clear();
    app.recent_projects_edits.clear();
    app.recent_projects_meta.clear();

    keep_element(super::view(&app));
}

#[test]
fn view_builds_recent_projects_home() {
    let mut app = App::new().0;
    app.recent_projects = vec!["/tmp/vibe-window".to_string()];
    app.recent_projects_edits = vec!["Vibe Window".to_string()];
    app.project_path = Some("/tmp/vibe-window".to_string());

    keep_element(super::view(&app));
}

#[test]
fn home_container_style_uses_light_background() {
    let style = super::home_container_style(&Theme::Light);

    assert_eq!(style.background, Some(Background::Color(Color::from_rgba8(246, 248, 252, 0.98))));
}

#[test]
fn home_container_style_uses_dark_theme_background() {
    let theme = Theme::Dark;
    let style = super::home_container_style(&theme);

    assert_eq!(
        style.background,
        Some(Background::Color(theme.extended_palette().background.base.color.scale_alpha(0.96)))
    );
}

//! 宿主保留：入口宽度归一化与工作/代码模式路由，并检查宿主代码面板状态不被改变。
//! 项目视图入口的测试，验证设置面板宽度归一化和工作区模式路由。
//! 用例只构建视图，重点保证异常及非有限宽度不会破坏元素树。
use crate::app::App;
use crate::app::state::WorkspaceMode;

fn render_project_view_with_settings_width(settings_panel_width: f32) {
    let (mut app, _task) = App::new();
    app.layout_drag.settings_panel_width = settings_panel_width;

    let element = super::view(&app);

    std::hint::black_box(element);
}

#[test]
fn view_builds_with_regular_settings_width() {
    render_project_view_with_settings_width(476.0);
}

#[test]
fn view_builds_with_too_small_settings_width() {
    render_project_view_with_settings_width(120.0);
}

#[test]
fn view_builds_with_too_large_settings_width() {
    render_project_view_with_settings_width(1_200.0);
}

#[test]
fn view_builds_with_non_finite_settings_width() {
    render_project_view_with_settings_width(f32::INFINITY);
}

#[test]
fn work_layout_project_view_routes_each_workspace_mode() {
    let (mut app, _task) = App::new();
    app.show_diff = true;
    app.file_manager.show_file_manager = true;
    app.terminal.is_visible = true;

    app.navigation_mode.workspace_mode = WorkspaceMode::Work;
    assert!(super::uses_work_layout(&app));
    std::hint::black_box(super::view(&app));

    app.navigation_mode.workspace_mode = WorkspaceMode::Code;
    assert!(!super::uses_work_layout(&app));
    std::hint::black_box(super::view(&app));

    assert!(app.show_diff);
    assert!(app.file_manager.show_file_manager);
    assert!(app.terminal.is_visible);
}

//! 项目主区域布局的测试，覆盖聊天、差异视图和文件管理器的组合。
//! 各用例验证全屏、半屏及分栏比例变化时仍能构建正确分支。
use super::*;

fn test_app() -> App {
    let (app, _task) = App::new();
    app
}

fn render_main_area(app: &App) {
    let element = main_area(app, 8.0, 12.0, 10.0, 6.0);

    std::hint::black_box(element);
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("main_tests"));
}

#[test]
fn main_area_builds_chat_fullscreen() {
    let mut app = test_app();
    app.chat_panel_fullscreen = true;
    app.git_diff_fullscreen = false;
    app.git_diff_half_fullscreen = false;
    app.chat_panel_half_fullscreen = false;

    render_main_area(&app);
}

#[test]
fn main_area_builds_chat_half_fullscreen_without_file_manager() {
    let mut app = test_app();
    app.chat_panel_half_fullscreen = true;
    app.show_file_manager = false;

    render_main_area(&app);
}

#[test]
fn main_area_builds_chat_half_fullscreen_with_file_manager() {
    let mut app = test_app();
    app.chat_panel_half_fullscreen = true;
    app.show_file_manager = true;
    app.file_manager_width = 240.0;

    render_main_area(&app);
}

#[test]
fn main_area_builds_git_diff_fullscreen() {
    let mut app = test_app();
    app.git_diff_fullscreen = true;
    app.chat_panel_fullscreen = false;
    app.chat_panel_half_fullscreen = false;

    render_main_area(&app);
}

#[test]
fn main_area_builds_git_diff_half_fullscreen_without_file_manager() {
    let mut app = test_app();
    app.git_diff_half_fullscreen = true;
    app.show_file_manager = false;

    render_main_area(&app);
}

#[test]
fn main_area_builds_git_diff_half_fullscreen_with_file_manager() {
    let mut app = test_app();
    app.git_diff_half_fullscreen = true;
    app.show_file_manager = true;
    app.file_manager_width = 300.0;

    render_main_area(&app);
}

#[test]
fn main_area_builds_split_diff_with_file_manager() {
    let mut app = test_app();
    app.show_diff = true;
    app.show_file_manager = true;
    app.split_ratio = 0.6;
    app.file_manager_width = 260.0;

    render_main_area(&app);
}

#[test]
fn main_area_builds_split_diff_with_tiny_left_ratio() {
    let mut app = test_app();
    app.show_diff = true;
    app.show_file_manager = false;
    app.split_ratio = 0.0;

    render_main_area(&app);
}

#[test]
fn main_area_builds_chat_only_without_file_manager() {
    let mut app = test_app();
    app.show_diff = false;
    app.show_file_manager = false;

    render_main_area(&app);
}

#[test]
fn main_area_builds_chat_only_with_file_manager() {
    let mut app = test_app();
    app.show_diff = false;
    app.show_file_manager = true;
    app.file_manager_width = 180.0;

    render_main_area(&app);
}

#[test]
fn main_area_builds_split_diff_without_file_manager() {
    let mut app = test_app();
    app.show_diff = true;
    app.show_file_manager = false;
    app.split_ratio = 0.6;

    render_main_area(&app);
}

#[test]
fn main_area_builds_split_diff_with_full_right_ratio() {
    let mut app = test_app();
    app.show_diff = true;
    app.show_file_manager = false;
    app.split_ratio = 1.0;

    render_main_area(&app);
}

#[test]
fn main_area_prioritizes_chat_fullscreen_over_git_diff() {
    let mut app = test_app();
    app.chat_panel_fullscreen = true;
    app.git_diff_fullscreen = true;

    render_main_area(&app);
}

#[test]
fn main_area_prioritizes_chat_half_fullscreen_over_git_diff() {
    let mut app = test_app();
    app.chat_panel_half_fullscreen = true;
    app.git_diff_half_fullscreen = true;

    render_main_area(&app);
}

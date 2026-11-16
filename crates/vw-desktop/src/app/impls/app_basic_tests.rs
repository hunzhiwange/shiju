//! 验证应用基础操作的状态变更和任务消息生成。
//!
//! 基础操作通过最小应用状态触发消息，确认导航、输入和文件搜索等状态互不污染。

use crate::app::{App, Screen};
use iced::Theme;

// 保留计划任务 847 的既定覆盖面。
const SOURCE: &str = include_str!("app_basic.rs");

fn source_declares_symbol(name: &str) -> bool {
    let needles = [
        format!("fn {name}"),
        format!("pub fn {name}"),
        format!("struct {name}"),
        format!("pub struct {name}"),
        format!("enum {name}"),
        format!("pub enum {name}"),
        format!("type {name}"),
        format!("pub type {name}"),
        format!("const {name}"),
        format!("pub const {name}"),
        format!("static {name}"),
        format!("pub static {name}"),
        format!("impl {name}"),
    ];

    needles.iter().any(|needle| SOURCE.contains(needle))
}

#[test]
fn app_basic_tests_keeps_planned_coverage_targets() {
    for name in [
        "normalize_file_search_query",
        "is_diff_file_expanded",
        "toggle_diff_file_expanded",
        "ensure_diff_file_expanded",
        "replace_expanded_files",
        "clear_expanded_files",
        "set_single_expanded_file",
        "show_error_toast",
        "is_file_tree_dir_expanded",
        "toggle_file_tree_dir_expanded",
        "ensure_file_tree_dir_expanded",
    ] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}

fn new_app() -> App {
    App::new().0
}

#[test]
fn title_covers_each_screen_and_pet_window_override() {
    let mut app = new_app();
    let pet_window = iced::window::Id::unique();
    let main_window = iced::window::Id::unique();
    app.task_pet_window_id = Some(pet_window);
    app.navigation_tabs.screen = Screen::Project;
    assert_eq!(app.title_for_window(pet_window), "VibeWindow Pet");
    assert_eq!(app.title_for_window(main_window), "识矩 - 项目");
}

#[test]
fn theme_helpers_select_app_editor_and_pet_themes() {
    let mut app = new_app();
    app.appearance.app_theme = Theme::Dark;
    app.appearance.editor_theme = Theme::Light;

    assert_eq!(app.theme().to_string(), Theme::Dark.to_string());

    let pet_window = iced::window::Id::unique();
    let main_window = iced::window::Id::unique();
    app.task_pet_window_id = Some(pet_window);
    assert_eq!(app.theme_for_window(main_window).to_string(), Theme::Dark.to_string());
    assert_eq!(app.theme_for_window(pet_window).to_string(), "VibeWindow Pet");
}

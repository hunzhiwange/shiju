use crate::app::App;
use crate::app::state::ToolDetailDialog;
use iced::widget::{Id, text_editor};

fn test_app() -> App {
    let (app, _task) = App::new();
    app
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("layout_tests"));
}

#[test]
fn base_layout_builds_with_settings_sidebar_and_bottom_panel() {
    let mut app = test_app();
    app.show_settings = true;
    app.terminal.is_visible = true;

    let element = super::build_base_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 6.0, 0.0);

    std::hint::black_box(element);
}

#[test]
fn base_layout_builds_without_settings_sidebar_or_bottom_panel() {
    let mut app = test_app();
    app.show_settings = false;
    app.terminal.is_visible = false;

    let element = super::build_base_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 6.0, 0.0);

    std::hint::black_box(element);
}

#[test]
fn base_layout_builds_hovered_recent_project_overlay() {
    let mut app = test_app();
    app.show_settings = false;
    app.hovered_recent_project = Some("/tmp/vibe-window-hovered".to_string());

    let element = super::build_base_layout(&app, 476.0, 56.0, 0.6, 12.0, 2.0, 6.0, 0.0);

    std::hint::black_box(element);
}

#[test]
fn base_layout_clamps_sidebar_overlay_width_inputs() {
    let mut app = test_app();
    app.show_settings = false;
    app.hovered_recent_project = Some("/tmp/vibe-window-hovered".to_string());

    let element = super::build_base_layout(&app, 40.0, 80.0, 1.25, 0.0, 1.0, 0.0, 0.0);

    std::hint::black_box(element);
}

#[test]
fn base_layout_builds_chat_fullscreen_without_bottom_panel() {
    let mut app = test_app();
    app.chat_panel_fullscreen = true;
    app.terminal.is_visible = true;

    let element = super::build_base_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 6.0, 0.0);

    std::hint::black_box(element);
}

#[test]
fn base_layout_builds_git_fullscreen_without_bottom_panel() {
    let mut app = test_app();
    app.git_diff_fullscreen = true;
    app.terminal.is_visible = true;

    let element = super::build_base_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 6.0, 0.0);

    std::hint::black_box(element);
}

#[test]
fn base_layout_builds_with_tool_detail_overlay() {
    let mut app = test_app();
    app.tool_detail_dialog = Some(ToolDetailDialog {
        msg_idx: 1,
        tool_idx: 2,
        title: "Tool output".to_string(),
        content: "line 1\nline 2".to_string(),
        editor: text_editor::Content::with_text("line 1\nline 2"),
        editor_id: Id::unique(),
        context_menu_open: false,
        context_menu_pos: None,
        scroll_top_line: 0.0,
        scroll_remainder: 0.0,
        viewport_height: 100.0,
    });

    let element = super::build_base_layout(&app, 476.0, 56.0, 0.6, 12.0, 0.0, 6.0, 0.0);

    std::hint::black_box(element);
}

#[test]
fn drag_badge_layer_builds_empty_layer() {
    let app = test_app();

    let element = super::drag_badge_layer(&app);

    std::hint::black_box(element);
}

#[test]
fn drag_badge_layer_builds_active_badge() {
    let mut app = test_app();
    app.project_path = Some("/tmp/project".to_string());
    app.dragging_file_paths =
        vec!["/tmp/project/src/main.rs".to_string(), "/tmp/project/README.md".to_string()];
    app.cursor_position = iced::Point::new(9_999.0, -20.0);
    app.window_size = (180.0, 36.0);

    let element = super::drag_badge_layer(&app);

    std::hint::black_box(element);
}

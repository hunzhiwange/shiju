#![allow(unused_must_use)]
//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。

use super::file_search::{handle_file_search_input_changed, handle_file_search_navigate_down, handle_file_search_navigate_up, handle_file_search_select, handle_file_search_select_current, handle_input_area_drag_drop, handle_input_editor_action, handle_remove_file_reference, ranked_file_search_entries, ranked_file_search_results};
use crate::app::{App, FocusArea};
use iced::widget::text_editor;
use std::sync::Arc;

fn paste(app: &mut App, value: String) -> iced::Task<crate::app::Message> {
    handle_input_editor_action(
        app,
        text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(value))),
    )
}

fn test_app() -> App {
    App::new().0
}

fn set_input_text(app: &mut App, text: &str) {
    let runtime = app.current_session_runtime_mut();
    runtime.input_editor = text_editor::Content::with_text(text);
    runtime.input_editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
}

#[test]
fn app_ranked_file_search_results_use_refreshed_cache() {
    let mut app = test_app();
    app.navigation_tabs.project_path = Some("/workspace".to_string());
    app.input_suggestions.file_search_query = "main".to_string();
    app.set_file_index("/workspace", vec!["src/main.rs".to_string(), "src/lib.rs".to_string()]);

    assert_eq!(ranked_file_search_results(&app), ["src/main.rs"]);
    assert_eq!(ranked_file_search_entries(&app)[0].path, "src/main.rs");
}







#[test]
fn file_search_navigation_clamps_to_available_results() {
    let mut app = test_app();
    app.navigation_tabs.project_path = Some("/workspace".to_string());
    app.input_suggestions.file_search_query = "src".to_string();
    app.set_file_index(
        "/workspace",
        vec!["src/a.rs".to_string(), "src/b.rs".to_string(), "src/c.rs".to_string()],
    );

    handle_file_search_navigate_down(&mut app);
    handle_file_search_navigate_down(&mut app);
    assert_eq!(app.input_suggestions.file_search_selected_index, 2);

    handle_file_search_navigate_down(&mut app);
    assert_eq!(app.input_suggestions.file_search_selected_index, 3);

    handle_file_search_navigate_up(&mut app);
    assert_eq!(app.input_suggestions.file_search_selected_index, 2);

    app.input_suggestions.file_search_query = "missing".to_string();
    app.refresh_file_search_cache();
    app.input_suggestions.file_search_selected_index = 0;
    handle_file_search_navigate_down(&mut app);
    handle_file_search_navigate_up(&mut app);
    assert_eq!(app.input_suggestions.file_search_selected_index, 0);
}







#[test]
fn selecting_current_file_search_result_uses_selected_index() {
    let mut app = test_app();
    app.navigation_tabs.project_path = Some("/workspace".to_string());
    app.input_suggestions.file_search_query = "src".to_string();
    app.set_file_index("/workspace", vec!["src/a.rs".to_string(), "src/b.rs".to_string()]);
    set_input_text(&mut app, "@s");
    app.input_suggestions.file_search_selected_index = 1;

    handle_file_search_select_current(&mut app);

    assert!(app.current_session_runtime().input_editor.text().starts_with("@src/"));

    app.input_suggestions.file_search_selected_index = 999;
    let previous = app.current_session_runtime().input_editor.text();
    handle_file_search_select_current(&mut app);
    assert_eq!(app.current_session_runtime().input_editor.text(), previous);
}





#[test]
fn input_area_drag_drop_inserts_mentions_and_clears_state() {
    let temp = tempfile::tempdir().expect("tempdir");
    let file_path = temp.path().join("main.rs");
    std::fs::write(&file_path, "fn main() {}").expect("write temp file");

    let mut app = test_app();
    app.navigation_tabs.project_path = Some(temp.path().to_string_lossy().to_string());
    app.file_tree_interaction.pending_drop_file_paths = vec![file_path.to_string_lossy().to_string()];
    app.file_tree_interaction.pending_drop_file_position = Some((9, 2));
    app.file_tree_interaction.input_drop_hovered = true;

    handle_input_area_drag_drop(&mut app);

    assert_eq!(app.current_session_runtime().input_editor.text(), "@main.rs:9:2 ");
    assert!(app.file_tree_interaction.pending_drop_file_paths.is_empty());
    assert_eq!(app.file_tree_interaction.pending_drop_file_position, None);
    assert!(!app.file_tree_interaction.input_drop_hovered);
    assert_eq!(app.focus_area, FocusArea::None);
}

#[test]
fn input_area_drag_drop_clears_empty_pending_drop_without_inserting() {
    let mut app = test_app();
    app.file_tree_interaction.pending_drop_file_paths.clear();
    app.file_tree_interaction.pending_drop_file_position = Some((1, 1));
    app.file_tree_interaction.input_drop_hovered = true;

    handle_input_area_drag_drop(&mut app);

    assert_eq!(app.current_session_runtime().input_editor.text(), "");
    assert_eq!(app.file_tree_interaction.pending_drop_file_position, None);
    assert!(!app.file_tree_interaction.input_drop_hovered);
}

#[test]
fn input_area_drag_drop_prefers_active_drag_paths_over_pending_paths() {
    let mut app = test_app();
    app.file_tree_interaction.dragging_file_paths = vec!["dragged.rs".to_string()];
    app.file_tree_interaction.dragging_file_position = Some((3, 7));
    app.file_tree_interaction.pending_drop_file_paths = vec!["pending.rs".to_string()];
    app.file_tree_interaction.pending_drop_file_position = Some((8, 9));

    handle_input_area_drag_drop(&mut app);

    assert_eq!(app.current_session_runtime().input_editor.text(), "@dragged.rs:3:7 ");
    assert!(app.file_tree_interaction.dragging_file_paths.is_empty());
    assert_eq!(app.file_tree_interaction.dragging_file_position, None);
    assert!(app.file_tree_interaction.pending_drop_file_paths.is_empty());
    assert_eq!(app.file_tree_interaction.pending_drop_file_position, None);
}

#[test]
fn input_area_drag_drop_ignores_single_position_for_multiple_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    let file_path = temp.path().join("main.rs");
    let directory_path = temp.path().join("assets");
    std::fs::write(&file_path, "fn main() {}").expect("write temp file");
    std::fs::create_dir(&directory_path).expect("create temp directory");

    let mut app = test_app();
    app.navigation_tabs.project_path = Some(temp.path().to_string_lossy().to_string());
    app.file_tree_interaction.pending_drop_file_paths =
        vec![file_path.to_string_lossy().to_string(), directory_path.to_string_lossy().to_string()];
    app.file_tree_interaction.pending_drop_file_position = Some((9, 2));

    handle_input_area_drag_drop(&mut app);

    assert_eq!(app.current_session_runtime().input_editor.text(), "@main.rs @assets/ ");
    assert!(app.file_tree_interaction.pending_drop_file_paths.is_empty());
    assert_eq!(app.file_tree_interaction.pending_drop_file_position, None);
}



// 宿主只验证引用编辑完成后的悬停状态装配。
#[test]
fn remove_file_reference_clears_host_hover() {
    let mut app = test_app();
    set_input_text(&mut app, "read @src/main.rs and @tests/input.rs");
    app.input_suggestions.file_ref_hovered_index = Some(2);
    handle_remove_file_reference(&mut app, "src/main.rs".to_string());
    assert_eq!(app.input_suggestions.file_ref_hovered_index, None);
}

#[test]
fn editor_actions_clear_host_focus() {
    let mut app = test_app();
    paste(&mut app, "@src".into());
    assert_eq!(app.focus_area, FocusArea::None);
}

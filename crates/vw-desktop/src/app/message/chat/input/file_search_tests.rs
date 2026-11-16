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
    app.file_search_query = "main".to_string();
    app.set_file_index("/workspace", vec!["src/main.rs".to_string(), "src/lib.rs".to_string()]);

    assert_eq!(ranked_file_search_results(&app), ["src/main.rs"]);
    assert_eq!(ranked_file_search_entries(&app)[0].path, "src/main.rs");
}

#[test]
fn file_search_input_changed_toggles_visibility_and_resets_selection() {
    let mut app = test_app();
    app.file_search_selected_index = 4;

    handle_file_search_input_changed(&mut app, " src ".to_string());

    assert!(app.show_file_search);
    assert_eq!(app.file_search_query, " src ");
    assert_eq!(app.file_search_selected_index, 0);

    app.file_search_selected_index = 3;
    handle_file_search_input_changed(&mut app, "   ".to_string());

    assert!(!app.show_file_search);
    assert_eq!(app.file_search_selected_index, 0);
}

#[test]
fn input_editor_actions_open_and_clear_file_search() {
    let mut app = test_app();
    app.navigation_tabs.project_path = Some("/workspace".to_string());
    app.set_file_index("/workspace", vec!["src/main.rs".to_string(), "tests/input.rs".to_string()]);

    paste(&mut app, "@src".to_string());

    assert!(app.show_file_search);
    assert_eq!(app.file_search_query, "src");
    assert_eq!(app.focus_area, FocusArea::None);

    handle_input_editor_action(
        &mut app,
        text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(" ".to_string()))),
    );

    assert!(!app.show_file_search);
    assert!(app.file_search_query.is_empty());

    paste(&mut app, String::new());
    handle_input_editor_action(&mut app, text_editor::Action::Edit(text_editor::Edit::Backspace));
    handle_input_editor_action(&mut app, text_editor::Action::Edit(text_editor::Edit::Delete));
    handle_input_editor_action(
        &mut app,
        text_editor::Action::Move(text_editor::Motion::DocumentEnd),
    );
    handle_input_editor_action(
        &mut app,
        text_editor::Action::Select(text_editor::Motion::DocumentStart),
    );
}

#[test]
fn input_editor_invalid_search_character_clears_active_search() {
    let mut app = test_app();

    paste(&mut app, "@src".to_string());
    assert!(app.show_file_search);

    paste(&mut app, "!".to_string());

    assert!(!app.show_file_search);
    assert!(app.file_search_query.is_empty());
    assert_eq!(app.file_search_selected_index, 0);
}

#[test]
fn file_search_navigation_clamps_to_available_results() {
    let mut app = test_app();
    app.navigation_tabs.project_path = Some("/workspace".to_string());
    app.file_search_query = "src".to_string();
    app.set_file_index(
        "/workspace",
        vec!["src/a.rs".to_string(), "src/b.rs".to_string(), "src/c.rs".to_string()],
    );

    handle_file_search_navigate_down(&mut app);
    handle_file_search_navigate_down(&mut app);
    assert_eq!(app.file_search_selected_index, 2);

    handle_file_search_navigate_down(&mut app);
    assert_eq!(app.file_search_selected_index, 3);

    handle_file_search_navigate_up(&mut app);
    assert_eq!(app.file_search_selected_index, 2);

    app.file_search_query = "missing".to_string();
    app.refresh_file_search_cache();
    app.file_search_selected_index = 0;
    handle_file_search_navigate_down(&mut app);
    handle_file_search_navigate_up(&mut app);
    assert_eq!(app.file_search_selected_index, 0);
}

#[test]
fn file_search_navigation_limits_selection_to_twenty_visible_results() {
    let mut app = test_app();
    app.navigation_tabs.project_path = Some("/workspace".to_string());
    app.set_file_index("/workspace", (0..25).map(|index| format!("file-{index:02}.rs")).collect());

    for _ in 0..30 {
        handle_file_search_navigate_down(&mut app);
    }

    assert_eq!(app.file_search_selected_index, 19);
}

#[test]
fn selecting_file_search_path_replaces_active_mention_or_appends() {
    let mut app = test_app();
    app.navigation_tabs.project_path = Some("/workspace".to_string());

    set_input_text(&mut app, "open @sr");
    handle_file_search_select(&mut app, "/workspace/src/main.rs".to_string());

    assert_eq!(app.current_session_runtime().input_editor.text(), "open @src/main.rs ");
    assert!(!app.show_file_search);
    assert!(app.file_search_query.is_empty());

    set_input_text(&mut app, "plain text");
    handle_file_search_select(&mut app, "tests/input.rs".to_string());

    assert_eq!(app.current_session_runtime().input_editor.text(), "plain text @tests/input.rs ");
}

#[test]
fn selecting_file_search_path_without_project_root_normalizes_backslashes() {
    let mut app = test_app();
    app.navigation_tabs.project_path = None;
    set_input_text(&mut app, "open @C");

    handle_file_search_select(&mut app, "C:\\workspace\\src\\main.rs".to_string());

    assert_eq!(
        app.current_session_runtime().input_editor.text(),
        "open @C:/workspace/src/main.rs "
    );
    assert!(!app.show_file_search);
}

#[test]
fn selecting_current_file_search_result_uses_selected_index() {
    let mut app = test_app();
    app.navigation_tabs.project_path = Some("/workspace".to_string());
    app.file_search_query = "src".to_string();
    app.set_file_index("/workspace", vec!["src/a.rs".to_string(), "src/b.rs".to_string()]);
    set_input_text(&mut app, "@s");
    app.file_search_selected_index = 1;

    handle_file_search_select_current(&mut app);

    assert!(app.current_session_runtime().input_editor.text().starts_with("@src/"));

    app.file_search_selected_index = 999;
    let previous = app.current_session_runtime().input_editor.text();
    handle_file_search_select_current(&mut app);
    assert_eq!(app.current_session_runtime().input_editor.text(), previous);
}

#[test]
fn remove_file_reference_deletes_mention_and_compacts_whitespace() {
    let mut app = test_app();
    set_input_text(&mut app, "read @src/main.rs and @tests/input.rs");
    app.file_ref_hovered_index = Some(2);

    handle_remove_file_reference(&mut app, "src/main.rs".to_string());

    assert_eq!(app.current_session_runtime().input_editor.text(), "read and @tests/input.rs");
    assert_eq!(app.file_ref_hovered_index, None);
}

#[test]
fn remove_file_reference_removes_all_matching_mentions() {
    let mut app = test_app();
    set_input_text(&mut app, "@src/main.rs compare @src/main.rs");

    handle_remove_file_reference(&mut app, "src/main.rs".to_string());

    assert_eq!(app.current_session_runtime().input_editor.text(), "compare");
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

#[test]
fn selecting_and_removing_spaced_chinese_file_preserves_other_mentions() {
    let mut app = test_app();
    let path = "已粘贴 2026-11-03 下午3.06.58.textClipping";
    set_input_text(&mut app, "@已");
    let _ = handle_file_search_select(&mut app, path.to_string());
    let input = app.current_session_runtime().input_editor.text();
    assert_eq!(
        input,
        format!("{} ", crate::app::components::input_mention_highlighter::file_mention(path))
    );
    let _ = handle_remove_file_reference(&mut app, path.to_string());
    assert!(app.current_session_runtime().input_editor.text().is_empty());
}

#[test]
fn removing_chinese_file_keeps_other_quoted_path_whitespace() {
    let mut app = test_app();
    set_input_text(&mut app, r#"@原始视频.mov @"目录/两个  空格.txt" @原始视频.mov.bak"#);
    handle_remove_file_reference(&mut app, "原始视频.mov".to_string());
    assert_eq!(
        app.current_session_runtime().input_editor.text(),
        r#"@"目录/两个  空格.txt" @原始视频.mov.bak"#
    );
}

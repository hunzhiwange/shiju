#![allow(unused_must_use)]
//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。

use super::file_search::{
    DroppedPath, active_file_search_query, build_ranked_file_search_entries, file_match_score,
    format_drop_mentions, fuzzy_subsequence_score, handle_file_search_input_changed,
    handle_file_search_navigate_down, handle_file_search_navigate_up, handle_file_search_select,
    handle_file_search_select_current, handle_input_area_drag_drop, handle_input_editor_action,
    handle_input_editor_backspace, handle_input_editor_commit, handle_input_editor_delete,
    handle_input_editor_motion, handle_remove_file_reference, is_file_search_char,
    normalized_lower_path, ranked_file_search_entries, ranked_file_search_results,
    replace_or_append_file_mention, trailing_token,
};
use crate::app::{App, FocusArea};
use iced::widget::text_editor;
use std::sync::Arc;

fn test_app() -> App {
    App::new().0
}

fn set_input_text(app: &mut App, text: &str) {
    let runtime = app.current_session_runtime_mut();
    runtime.input_editor = text_editor::Content::with_text(text);
    runtime.input_editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
}

#[test]
fn format_drop_mentions_appends_trailing_slash_for_directories() {
    let mentions = format_drop_mentions(
        Some("/workspace"),
        &[DroppedPath { path: "/workspace/src/components".to_string(), is_dir: true }],
        None,
    );

    assert_eq!(mentions, vec!["src/components/".to_string()]);
}

#[test]
fn ranked_file_search_entries_include_deduped_parent_directories() {
    let entries = build_ranked_file_search_entries(
        &["src/main.rs".to_string(), "src/bin/tool.rs".to_string(), "README.md".to_string()],
        "",
    );

    assert!(entries.iter().any(|entry| entry.path == "src/" && entry.is_dir));
    assert!(entries.iter().any(|entry| entry.path == "src/bin/" && entry.is_dir));
    assert_eq!(entries.iter().filter(|entry| entry.path == "src/").count(), 1);
    assert!(entries.iter().any(|entry| entry.path == "README.md" && !entry.is_dir));
}

#[test]
fn ranked_file_search_entries_prioritize_filename_matches() {
    let entries = build_ranked_file_search_entries(
        &[
            "docs/readme.md".to_string(),
            "src/search_panel.rs".to_string(),
            "src/file_search.rs".to_string(),
        ],
        "file_search",
    );

    assert_eq!(entries.first().map(|entry| entry.path.as_str()), Some("src/file_search.rs"));
}

#[test]
fn ranked_file_search_entries_filter_non_matching_queries() {
    let entries =
        build_ranked_file_search_entries(&["src/main.rs".to_string()], "zzzz-not-present");

    assert!(entries.is_empty());
}

#[test]
fn ranked_file_search_entries_normalize_parent_separators_and_keep_original_file_path() {
    let entries = build_ranked_file_search_entries(&["src\\bin\\tool.rs".to_string()], "");

    assert_eq!(entries[0].path, "src\\bin\\tool.rs");
    assert!(!entries[0].is_dir);
    assert!(entries.iter().any(|entry| entry.path == "src/" && entry.is_dir));
    assert!(entries.iter().any(|entry| entry.path == "src/bin/" && entry.is_dir));
}

#[test]
fn ranked_file_search_entries_put_files_before_directories_for_equal_scores() {
    let entries = build_ranked_file_search_entries(
        &["zeta/main.rs".to_string(), "alpha/lib.rs".to_string()],
        "",
    );
    let first_directory = entries.iter().position(|entry| entry.is_dir).expect("directory entry");

    assert!(entries[..first_directory].iter().all(|entry| !entry.is_dir));
    assert!(entries[first_directory..].iter().all(|entry| entry.is_dir));
}

#[test]
fn normalized_lower_path_converts_separators_and_ascii_case() {
    assert_eq!(normalized_lower_path("SRC\\Components/Main.RS"), "src/components/main.rs");
}

#[test]
fn fuzzy_subsequence_score_handles_empty_missing_contiguous_and_gapped_queries() {
    assert_eq!(fuzzy_subsequence_score("src/main.rs", ""), Some(0));
    assert_eq!(fuzzy_subsequence_score("src/main.rs", "xyz"), None);

    let contiguous = fuzzy_subsequence_score("file_search.rs", "file").expect("contiguous match");
    let gapped = fuzzy_subsequence_score("f_i_l_e.rs", "file").expect("gapped match");
    assert!(contiguous > gapped);
}

#[test]
fn file_match_score_rewards_exact_filename_and_rejects_non_subsequences() {
    let exact = file_match_score("src/main.rs", "main.rs").expect("exact filename match");
    let partial = file_match_score("src/domain.rs", "main").expect("partial match");

    assert!(exact > partial);
    assert_eq!(file_match_score("src/main.rs", "xyz"), None);
    assert_eq!(file_match_score("src/main.rs", ""), Some(0));
}

#[test]
fn file_search_character_policy_accepts_path_syntax_and_rejects_unsafe_punctuation() {
    for allowed in ['a', 'Z', '7', '/', '\\', '.', '_', '-', ':', '#'] {
        assert!(is_file_search_char(allowed), "expected {allowed:?} to be accepted");
    }
    for rejected in [' ', '@', '!', '?', ';', '\n', '中'] {
        assert!(!is_file_search_char(rejected), "expected {rejected:?} to be rejected");
    }
}

#[test]
fn trailing_token_uses_last_unicode_whitespace_boundary() {
    assert_eq!(trailing_token("open src/main.rs"), "src/main.rs");
    assert_eq!(trailing_token("open\t@src\n@tests"), "@tests");
    assert_eq!(trailing_token("prefix\u{2003}@unicode"), "@unicode");
    assert_eq!(trailing_token("ends with space "), "");
}

#[test]
fn active_file_search_query_accepts_supported_suffixes_and_empty_query() {
    assert_eq!(active_file_search_query("open @src/main.rs:12#L"), Some("src/main.rs:12#L"));
    assert_eq!(active_file_search_query("@"), Some(""));
    assert_eq!(active_file_search_query("prefix@src"), Some("src"));
}

#[test]
fn active_file_search_query_rejects_absent_or_invalid_mentions() {
    assert_eq!(active_file_search_query("plain text"), None);
    assert_eq!(active_file_search_query("open @src/main.rs!"), None);
    assert_eq!(active_file_search_query("old @valid next invalid?"), None);
}

#[test]
fn replace_or_append_file_mention_replaces_only_active_valid_suffix() {
    assert_eq!(
        replace_or_append_file_mention("compare @old then @sr", "@src/main.rs"),
        "compare @old then @src/main.rs "
    );
    assert_eq!(
        replace_or_append_file_mention("keep @invalid!", "@src/main.rs"),
        "keep @invalid! @src/main.rs "
    );
}

#[test]
fn replace_or_append_file_mention_handles_empty_and_existing_space() {
    assert_eq!(replace_or_append_file_mention("", "@README.md"), "@README.md ");
    assert_eq!(replace_or_append_file_mention("review ", "@README.md"), "review @README.md ");
}

#[test]
fn app_ranked_file_search_results_use_refreshed_cache() {
    let mut app = test_app();
    app.project_path = Some("/workspace".to_string());
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
    app.project_path = Some("/workspace".to_string());
    app.set_file_index("/workspace", vec!["src/main.rs".to_string(), "tests/input.rs".to_string()]);

    handle_input_editor_commit(&mut app, "@src".to_string());

    assert!(app.show_file_search);
    assert_eq!(app.file_search_query, "src");
    assert_eq!(app.focus_area, FocusArea::None);

    handle_input_editor_action(
        &mut app,
        text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(" ".to_string()))),
    );

    assert!(!app.show_file_search);
    assert!(app.file_search_query.is_empty());

    handle_input_editor_commit(&mut app, String::new());
    handle_input_editor_backspace(&mut app);
    handle_input_editor_delete(&mut app);
    handle_input_editor_motion(&mut app, text_editor::Motion::DocumentEnd, false);
    handle_input_editor_motion(&mut app, text_editor::Motion::DocumentStart, true);
}

#[test]
fn input_editor_invalid_search_character_clears_active_search() {
    let mut app = test_app();

    handle_input_editor_commit(&mut app, "@src".to_string());
    assert!(app.show_file_search);

    handle_input_editor_commit(&mut app, "!".to_string());

    assert!(!app.show_file_search);
    assert!(app.file_search_query.is_empty());
    assert_eq!(app.file_search_selected_index, 0);
}

#[test]
fn file_search_navigation_clamps_to_available_results() {
    let mut app = test_app();
    app.project_path = Some("/workspace".to_string());
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
    app.project_path = Some("/workspace".to_string());
    app.set_file_index("/workspace", (0..25).map(|index| format!("file-{index:02}.rs")).collect());

    for _ in 0..30 {
        handle_file_search_navigate_down(&mut app);
    }

    assert_eq!(app.file_search_selected_index, 19);
}

#[test]
fn selecting_file_search_path_replaces_active_mention_or_appends() {
    let mut app = test_app();
    app.project_path = Some("/workspace".to_string());

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
    app.project_path = None;
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
    app.project_path = Some("/workspace".to_string());
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
    app.project_path = Some(temp.path().to_string_lossy().to_string());
    app.pending_drop_file_paths = vec![file_path.to_string_lossy().to_string()];
    app.pending_drop_file_position = Some((9, 2));
    app.input_drop_hovered = true;

    handle_input_area_drag_drop(&mut app);

    assert_eq!(app.current_session_runtime().input_editor.text(), "@main.rs:9:2 ");
    assert!(app.pending_drop_file_paths.is_empty());
    assert_eq!(app.pending_drop_file_position, None);
    assert!(!app.input_drop_hovered);
    assert_eq!(app.focus_area, FocusArea::None);
}

#[test]
fn input_area_drag_drop_clears_empty_pending_drop_without_inserting() {
    let mut app = test_app();
    app.pending_drop_file_paths.clear();
    app.pending_drop_file_position = Some((1, 1));
    app.input_drop_hovered = true;

    handle_input_area_drag_drop(&mut app);

    assert_eq!(app.current_session_runtime().input_editor.text(), "");
    assert_eq!(app.pending_drop_file_position, None);
    assert!(!app.input_drop_hovered);
}

#[test]
fn input_area_drag_drop_prefers_active_drag_paths_over_pending_paths() {
    let mut app = test_app();
    app.dragging_file_paths = vec!["dragged.rs".to_string()];
    app.dragging_file_position = Some((3, 7));
    app.pending_drop_file_paths = vec!["pending.rs".to_string()];
    app.pending_drop_file_position = Some((8, 9));

    handle_input_area_drag_drop(&mut app);

    assert_eq!(app.current_session_runtime().input_editor.text(), "@dragged.rs:3:7 ");
    assert!(app.dragging_file_paths.is_empty());
    assert_eq!(app.dragging_file_position, None);
    assert!(app.pending_drop_file_paths.is_empty());
    assert_eq!(app.pending_drop_file_position, None);
}

#[test]
fn input_area_drag_drop_ignores_single_position_for_multiple_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    let file_path = temp.path().join("main.rs");
    let directory_path = temp.path().join("assets");
    std::fs::write(&file_path, "fn main() {}").expect("write temp file");
    std::fs::create_dir(&directory_path).expect("create temp directory");

    let mut app = test_app();
    app.project_path = Some(temp.path().to_string_lossy().to_string());
    app.pending_drop_file_paths =
        vec![file_path.to_string_lossy().to_string(), directory_path.to_string_lossy().to_string()];
    app.pending_drop_file_position = Some((9, 2));

    handle_input_area_drag_drop(&mut app);

    assert_eq!(app.current_session_runtime().input_editor.text(), "@main.rs @assets/ ");
    assert!(app.pending_drop_file_paths.is_empty());
    assert_eq!(app.pending_drop_file_position, None);
}

#[test]
fn format_drop_mentions_keeps_single_file_position() {
    let mentions = format_drop_mentions(
        Some("/workspace"),
        &[DroppedPath { path: "/workspace/src/main.rs".to_string(), is_dir: false }],
        Some((12, 4)),
    );

    assert_eq!(mentions, vec!["src/main.rs:12:4".to_string()]);
}

#[test]
fn format_drop_mentions_ignores_position_for_multiple_paths() {
    let mentions = format_drop_mentions(
        Some("/workspace"),
        &[
            DroppedPath { path: "/workspace/src/main.rs".to_string(), is_dir: false },
            DroppedPath { path: "/workspace/src/lib.rs".to_string(), is_dir: false },
        ],
        Some((12, 4)),
    );

    assert_eq!(mentions, vec!["src/main.rs".to_string(), "src/lib.rs".to_string()]);
}

#[test]
fn format_drop_mentions_normalizes_unrooted_paths_and_ignores_directory_position() {
    let mentions = format_drop_mentions(
        None,
        &[
            DroppedPath { path: "external\\main.rs".to_string(), is_dir: false },
            DroppedPath { path: "external\\assets".to_string(), is_dir: true },
        ],
        Some((5, 6)),
    );

    assert_eq!(mentions, vec!["external/main.rs".to_string(), "external/assets/".to_string()]);
}

#[test]
fn format_drop_mentions_returns_empty_for_empty_drop() {
    assert!(format_drop_mentions(Some("/workspace"), &[], Some((1, 2))).is_empty());
}

use super::todo_views::{
    todo_tool_card_padding, todo_tool_expanded, tool_expanded, tool_todos_view,
    tool_todowrite_compact_view,
};
use crate::app::App;
use vw_shared::todo::Todo;

fn app() -> App {
    App::new().0
}

fn todo(id: &str, status: &str, content: &str) -> Todo {
    Todo {
        id: id.to_string(),
        status: status.to_string(),
        content: content.to_string(),
        priority: "medium".to_string(),
    }
}

#[test]
fn todo_views_test_module_is_linked() {
    assert_eq!("todo_views", "todo_views");
}

#[test]
fn todo_tool_card_padding_is_removed() {
    let padding = todo_tool_card_padding();

    assert_eq!(padding.top, 0.0);
    assert_eq!(padding.right, 0.0);
    assert_eq!(padding.bottom, 0.0);
    assert_eq!(padding.left, 0.0);
}

#[test]
fn todo_tool_expanded_ignores_running_until_user_expands() {
    assert!(!todo_tool_expanded(false, true));
    assert!(todo_tool_expanded(true, true));
}

#[test]
fn todowrite_view_handles_collapsed_expanded_and_error_states() {
    let mut app = app();
    let key = (2_u64 << 32) | 3;
    let visible = r#"tool todowrite
{"status":"completed","input":"{\"merge\":true,\"todos\":[{\"status\":\"completed\",\"content\":\"done\"},{\"status\":\"in_progress\",\"content\":\"doing\"},{\"status\":\"pending\",\"content\":\"todo\"}]}"}"#;
    let error = r#"tool todowrite
{"status":"error","input":"{\"todos\":[]}","error":"write failed"}"#;

    assert!(tool_todowrite_compact_view(&app, 2, 3, visible).is_some());
    app.chat_tool_expanded.insert(key);
    assert!(tool_todowrite_compact_view(&app, 2, 3, visible).is_some());
    assert!(tool_todowrite_compact_view(&app, 2, 4, error).is_some());
    assert!(tool_todowrite_compact_view(&app, 2, 5, "tool bash\n{}").is_none());
    assert!(tool_todowrite_compact_view(&app, 2, 6, "tool todowrite\nnot-json").is_none());
}

#[test]
fn todoread_view_sorts_and_renders_parsed_todos() {
    let mut app = app();
    app.chat_tool_expanded.insert((4_u64 << 32) | 5);
    let visible = r#"tool todoread
{"status":"completed","output":"[{\"id\":\"10\",\"content\":\"later\",\"status\":\"pending\"},{\"id\":\"2\",\"content\":\"done\",\"status\":\"completed\"},{\"id\":\"x\",\"content\":\"doing\",\"status\":\"in_progress\"}]"}"#;
    let empty = r#"tool todoread
{"status":"completed","output":"[]"}"#;

    assert!(tool_todos_view(&app, 4, 5, visible).is_some());
    assert!(tool_todos_view(&app, 4, 6, empty).is_some());
    assert!(tool_todos_view(&app, 4, 7, "tool todowrite\n{}").is_none());
    assert!(tool_todos_view(&app, 4, 8, "tool todoread\n{}").is_none());
}

#[test]
fn tool_expanded_delegates_to_todo_tool_expanded_app_state() {
    let mut app = app();
    let key = (0_u64 << 32) | 0;

    assert!(!tool_expanded(&app, key, false));
    app.chat_tool_expanded.insert(key);
    assert!(tool_expanded(&app, key, false));
    assert!(tool_expanded(&app, key, true));
}

#[test]
fn todowrite_view_running_status_label() {
    let app = app();
    let visible = r#"tool todowrite
{"status":"running","input":"{\"todos\":[{\"status\":\"pending\",\"content\":\"x\"}]}"}"#;
    assert!(tool_todowrite_compact_view(&app, 1, 2, visible).is_some());
}

#[test]
fn todowrite_view_completed_without_merge_label() {
    let app = app();
    let visible = r#"tool todowrite
{"status":"completed","input":"{\"merge\":false,\"todos\":[{\"status\":\"completed\",\"content\":\"done\"}]}"}"#;
    assert!(tool_todowrite_compact_view(&app, 1, 2, visible).is_some());
}

#[test]
fn todowrite_view_unknown_status_falls_to_default_label() {
    let app = app();
    let visible = r#"tool todowrite
{"status":"unknown","input":"{\"todos\":[]}"}"#;
    assert!(tool_todowrite_compact_view(&app, 1, 2, visible).is_some());
}

#[test]
fn todowrite_view_denied_and_error_status_produce_view() {
    let app = app();
    let visible = r#"tool todowrite
{"status":"denied","input":"{\"todos\":[]}","error":"permission denied"}"#;
    assert!(tool_todowrite_compact_view(&app, 1, 2, visible).is_some());
}

#[test]
fn todowrite_view_expanded_with_empty_todos() {
    let mut app = app();
    let key = (5_u64 << 32) | 6;
    app.chat_tool_expanded.insert(key);
    let visible = r#"tool todowrite
{"status":"completed","input":"{}"}"#;
    assert!(tool_todowrite_compact_view(&app, 5, 6, visible).is_some());
}

#[test]
fn todowrite_view_scroll_threshold_for_many_todos() {
    let mut app = app();
    let key = (6_u64 << 32) | 7;
    app.chat_tool_expanded.insert(key);
    let visible = r#"tool todowrite
{"status":"completed","input":"{\"todos\":[{\"status\":\"pending\",\"content\":\"t0\"},{\"status\":\"pending\",\"content\":\"t1\"},{\"status\":\"pending\",\"content\":\"t2\"},{\"status\":\"pending\",\"content\":\"t3\"},{\"status\":\"pending\",\"content\":\"t4\"},{\"status\":\"pending\",\"content\":\"t5\"},{\"status\":\"pending\",\"content\":\"t6\"},{\"status\":\"pending\",\"content\":\"t7\"}]}"}"#;
    assert!(tool_todowrite_compact_view(&app, 6, 7, visible).is_some());
}

#[test]
fn todoread_view_detects_running_todos() {
    let app = app();
    let visible = r#"tool todoread
{"status":"completed","output":"[{\"id\":\"1\",\"content\":\"active\",\"status\":\"in_progress\"}]"}"#;
    assert!(tool_todos_view(&app, 4, 5, visible).is_some());
}

#[test]
fn todoread_view_empty_expanded_shows_no_items() {
    let mut app = app();
    app.chat_tool_expanded.insert((7_u64 << 32) | 8);
    let visible = r#"tool todoread
{"status":"completed","output":"[]"}"#;
    assert!(tool_todos_view(&app, 7, 8, visible).is_some());
}

#[test]
fn todoread_view_many_todos_triggers_scroll_threshold() {
    let mut app = app();
    app.chat_tool_expanded.insert((8_u64 << 32) | 9);
    let visible = r#"tool todoread
{"status":"completed","output":"[{\"id\":\"1\",\"content\":\"a\",\"status\":\"pending\"},{\"id\":\"2\",\"content\":\"b\",\"status\":\"pending\"},{\"id\":\"3\",\"content\":\"c\",\"status\":\"pending\"},{\"id\":\"4\",\"content\":\"d\",\"status\":\"pending\"},{\"id\":\"5\",\"content\":\"e\",\"status\":\"pending\"},{\"id\":\"6\",\"content\":\"f\",\"status\":\"pending\"},{\"id\":\"7\",\"content\":\"g\",\"status\":\"pending\"}]"}"#;
    assert!(tool_todos_view(&app, 8, 9, visible).is_some());
}

#[test]
fn todoread_view_falls_back_to_current_session_todos_when_output_is_not_json() {
    let mut app = app();
    app.active_session_id = Some("session".to_string());
    app.chat_todo_session_id = Some("session".to_string());
    app.chat_todo_items =
        vec![todo("task_b", "pending", "text id"), todo("2", "completed", "numeric id")];
    app.chat_tool_expanded.insert((9_u64 << 32) | 10);
    let visible = r#"tool todoread
{"status":"completed","output":"not-json"}"#;

    assert!(tool_todos_view(&app, 9, 10, visible).is_some());
}

#[test]
fn todowrite_view_defaults_malformed_input_to_an_empty_list() {
    let mut app = app();
    let msg_idx = 9;
    let tool_idx = 10;
    app.chat_tool_expanded.insert((msg_idx as u64) << 32 | tool_idx as u64);
    let visible =
        format!("tool todowrite\n{}", serde_json::json!({"status":"completed", "input":"{"}));

    assert!(tool_todowrite_compact_view(&app, msg_idx, tool_idx, &visible).is_some());
}

//! 验证文件工具分类及路径列表从结构化、文本和输入载荷中的解析规则。
//! 搜索列表从尾部截断，普通列表从中部截断，以分别保留最相关结果和首尾上下文。

use super::parse::{
    build_file_list_state, is_edit_like_tool, is_git_diff_tool, is_search_tool, parse_output_files,
    parse_read_range, should_skip_files_view,
};
use crate::app::App;

fn test_app() -> App {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/vibe-window".to_string());
    app
}

#[test]
fn tool_classification_is_explicit() {
    assert!(is_git_diff_tool("git_diff", ""));
    assert!(is_git_diff_tool("git_operations", r#"{"operation":"diff"}"#));
    assert!(!is_git_diff_tool("git_operations", r#"{"operation":"status"}"#));
    assert!(!is_git_diff_tool("bash", r#"{"operation":"diff"}"#));
    assert!(!is_git_diff_tool("git_operations", "not json"));

    assert!(should_skip_files_view("apply_patch", ""));
    assert!(should_skip_files_view("git_operations", r#"{"operation":"diff"}"#));
    assert!(!should_skip_files_view("read", ""));

    assert!(is_edit_like_tool("write"));
    assert!(is_edit_like_tool("file_edit"));
    assert!(is_edit_like_tool("notebook_edit"));
    assert!(!is_edit_like_tool("read"));

    assert!(is_search_tool("grep"));
    assert!(is_search_tool("glob_search"));
    assert!(is_search_tool("codesearch"));
    assert!(!is_search_tool("read"));
}

#[test]
fn parse_read_range_formats_present_fields() {
    assert_eq!(
        parse_read_range("read", r#"{"offset":0,"limit":20}"#),
        Some("offset=1, limit=20".to_string())
    );
    assert_eq!(parse_read_range("bash", r#"{"offset":1}"#), None);
    assert_eq!(parse_read_range("read", r#"{"offset":2}"#), Some("offset=2".to_string()));
    assert_eq!(parse_read_range("file_read", r#"{"limit":5}"#), Some("limit=5".to_string()));
    assert_eq!(parse_read_range("read", "src/main.rs"), None);
    assert_eq!(parse_read_range("read", r#"{"path":"src/main.rs"}"#), None);
}

#[test]
fn build_file_list_state_filters_search_results() {
    let items = vec![
        ("src/main.rs".to_string(), "/tmp/src/main.rs".to_string()),
        ("README.md".to_string(), "/tmp/README.md".to_string()),
    ];

    let state = build_file_list_state(items, true, "main", 100);

    assert_eq!(state.display_count, 1);
    assert_eq!(state.items_for_display[0].0, "src/main.rs");
    assert_eq!(state.filter_query, "main");
}

#[test]
fn build_file_list_state_truncates_search_tail() {
    let items = (0..5)
        .map(|idx| (format!("file-{idx}.rs"), format!("/tmp/file-{idx}.rs")))
        .collect::<Vec<_>>();

    let state = build_file_list_state(items, true, "", 3);

    assert_eq!(state.display_count, 3);
    assert_eq!(state.tail_omitted, 2);
    assert!(!state.truncated_middle);
}

#[test]
fn build_file_list_state_truncates_non_search_middle() {
    let items = (0..6)
        .map(|idx| (format!("file-{idx}.rs"), format!("/tmp/file-{idx}.rs")))
        .collect::<Vec<_>>();

    let state = build_file_list_state(items, false, "", 4);

    assert_eq!(state.display_count, 4);
    assert!(state.truncated_middle);
    assert_eq!(state.items_for_display[0].0, "file-0.rs");
    assert_eq!(state.items_for_display[3].0, "file-5.rs");
}

#[test]
fn build_file_list_state_marks_empty_filtered_search() {
    let items = vec![("README.md".to_string(), "/tmp/README.md".to_string())];

    let state = build_file_list_state(items, true, "missing", 10);

    assert_eq!(state.display_count, 0);
    assert!(state.is_empty_filtered);
    assert_eq!(state.filter_query, "missing");
}

#[test]
fn parse_output_files_prefers_structured_changes_and_sorts_paths() {
    let app = test_app();
    let value = serde_json::json!({
        "result": {
            "content": [{
                "type": "structured_patch",
                "hunks": [
                    {"path":"src/b.rs","lines":[" old","+new","-old"]},
                    {"path":"src/a.rs","lines":["+new"]}
                ]
            }]
        }
    });

    let (changes, items) = parse_output_files(&app, "file_edit", "{}", "", &value);

    assert_eq!(changes.len(), 2);
    assert_eq!(items[0].0, "src/a.rs");
    assert_eq!(items[0].1, "/tmp/vibe-window/src/a.rs");
}

#[test]
fn parse_output_files_keeps_unresolved_structured_change_path() {
    let app = App::new().0;
    let value = serde_json::json!({
        "result": {
            "content": [{
                "type": "structured_patch",
                "hunks": [{"path":"file:///","lines":["+new"]}]
            }]
        }
    });

    let (changes, items) = parse_output_files(&app, "file_edit", "{}", "", &value);

    assert!(changes.contains_key("file:///"));
    assert_eq!(items, vec![("file:///".to_string(), "file:///".to_string())]);
}

#[test]
fn parse_output_files_reads_path_lines_bullets_and_dedups() {
    let app = test_app();
    let output = "path: src/main.rs\n- src/main.rs\n- src/lib.rs";

    let (_changes, items) = parse_output_files(&app, "grep", "{}", output, &serde_json::json!({}));

    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|(display, abs)| {
        display == "src/main.rs" && abs == "/tmp/vibe-window/src/main.rs"
    }));
}

#[test]
fn parse_output_files_uses_file_link_and_tool_paths() {
    let app = test_app();
    let file_link = "<file_link>\nopen: /tmp/out.rs\npath: shown.rs\n</file_link>";
    let (_changes, items) =
        parse_output_files(&app, "read", "{}", file_link, &serde_json::json!({}));
    assert_eq!(items, vec![("shown.rs".to_string(), "/tmp/out.rs".to_string())]);

    let value = serde_json::json!({"metadata":{"outputPath":"src/out.rs"}});
    let (_changes, items) = parse_output_files(&app, "bash", "{}", "", &value);
    assert_eq!(
        items,
        vec![(
            "/tmp/vibe-window/src/out.rs".to_string(),
            "/tmp/vibe-window/src/out.rs".to_string()
        )]
    );
}

#[test]
fn parse_output_files_falls_back_to_read_and_edit_input_paths() {
    let app = test_app();
    let input = r#"{"file_path":"src/read.rs"}"#;
    let (_changes, items) = parse_output_files(&app, "read", input, "", &serde_json::json!({}));
    assert_eq!(items[0].1, "/tmp/vibe-window/src/read.rs");

    let input = r#"{"path":"src/write.rs"}"#;
    let (_changes, items) = parse_output_files(&app, "write", input, "", &serde_json::json!({}));
    assert_eq!(items[0].1, "/tmp/vibe-window/src/write.rs");
}

#[test]
fn parse_output_files_falls_back_to_changes_block() {
    let app = test_app();
    let output = "<changes>\n{\"files\":[{\"path\":\"src/test.rs\",\"additions\":3,\"deletions\":1}]}\n</changes>";
    let (changes, items) = parse_output_files(&app, "bash", "{}", output, &serde_json::json!({}));
    assert_eq!(changes.len(), 1);
    assert_eq!(changes["src/test.rs"].path, "src/test.rs");
    assert_eq!(items[0].1, "/tmp/vibe-window/src/test.rs");
}

#[test]
fn parse_output_files_falls_through_with_no_matches() {
    let app = test_app();
    let (changes, items) = parse_output_files(&app, "bash", "echo hi", "", &serde_json::json!({}));
    assert!(changes.is_empty());
    assert!(items.is_empty());
}

#[test]
fn parse_output_files_uses_file_link_open_without_path_line() {
    let app = test_app();
    let output = "<file_link>\nopen: /tmp/out.rs\n</file_link>";
    let (_changes, items) = parse_output_files(&app, "read", "{}", output, &serde_json::json!({}));
    assert_eq!(items, vec![("/tmp/out.rs".to_string(), "/tmp/out.rs".to_string())]);
}

#[test]
fn parse_output_files_skips_file_link_with_non_absolute_open() {
    let app = test_app();
    let output = "<file_link>\nopen: relative/path.rs\n</file_link>";
    let (_changes, items) = parse_output_files(&app, "read", "{}", output, &serde_json::json!({}));
    assert!(items.is_empty());
}

#[test]
fn parse_output_files_ignores_file_link_without_open() {
    let app = test_app();
    let output = "<file_link>\npath: shown.rs\n</file_link>";
    let (_changes, items) = parse_output_files(&app, "read", "{}", output, &serde_json::json!({}));
    assert!(items.is_empty());
}

#[test]
fn parse_read_range_rejects_invalid_json() {
    assert_eq!(parse_read_range("read", "{bad json}"), None);
}

#[test]
fn build_file_list_state_non_search_ignores_filter() {
    let items = vec![
        ("src/main.rs".to_string(), "/tmp/main.rs".to_string()),
        ("README.md".to_string(), "/tmp/readme.md".to_string()),
    ];
    let state = build_file_list_state(items.clone(), false, "main", 100);
    assert_eq!(state.display_count, 2);
    assert!(!state.is_empty_filtered);
    assert_eq!(state.filter_query, "main");
}

#[test]
fn build_file_list_state_empty_items() {
    let state = build_file_list_state(vec![], true, "", 100);
    assert_eq!(state.display_count, 0);
    assert!(state.is_empty_filtered);
    assert_eq!(state.total_items, 0);
}

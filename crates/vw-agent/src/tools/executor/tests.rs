//! 工具执行器历史消息构造的测试。
//!
//! 当前覆盖原生 tool message ID 存在时，历史输出应保留为 tool 角色消息。

use super::*;
use serde_json::json;

#[test]
fn build_tool_result_history_messages_uses_native_tool_messages_when_ids_exist() {
    let messages = build_tool_result_history_messages(
        &[],
        &[ToolResultHistoryEntry {
            tool_name: "file_read".to_string(),
            tool_call_id: Some("call_1".to_string()),
            output: "hello".to_string(),
        }],
        true,
    );

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].role, "tool");
    assert!(messages[0].content.contains("call_1"));
    assert!(messages[0].content.contains("hello"));
}

#[test]
fn build_tool_result_history_messages_covers_empty_prompt_and_native_paths() {
    assert!(build_tool_result_history_messages(&[], &[], false).is_empty());

    let entries = vec![ToolResultHistoryEntry {
        tool_name: "file_read".to_string(),
        tool_call_id: None,
        output: "plain output".to_string(),
    }];
    let prompt_messages = build_tool_result_history_messages(&[], &entries, false);
    assert_eq!(prompt_messages.len(), 1);
    assert_eq!(prompt_messages[0].role, "user");
    assert!(prompt_messages[0].content.contains("<tool_result name=\"file_read\">"));

    let native_calls = vec![ToolCall {
        id: "native-1".to_string(),
        name: "file_read".to_string(),
        arguments: "{}".to_string(),
    }];
    let native_messages = build_tool_result_history_messages(&native_calls, &entries, true);
    assert_eq!(native_messages.len(), 1);
    assert_eq!(native_messages[0].role, "tool");
    assert!(native_messages[0].content.contains("native-1"));
    assert!(native_messages[0].content.contains("plain output"));
}

#[test]
fn normalize_tool_args_maps_supported_aliases_without_overwriting_canonical_values() {
    assert_eq!(
        normalize_tool_args("file_read", json!({ "filePath": "README.md" }))["path"],
        "README.md"
    );
    assert_eq!(
        normalize_tool_args(
            "file_edit",
            json!({
                "path": "src/lib.rs",
                "oldString": "old",
                "newString": "new",
                "replaceAll": true
            })
        ),
        json!({
            "path": "src/lib.rs",
            "file_path": "src/lib.rs",
            "oldString": "old",
            "old_string": "old",
            "newString": "new",
            "new_string": "new",
            "replaceAll": true,
            "replace_all": true
        })
    );
    assert_eq!(
        normalize_tool_args("web_fetch", json!({ "href": "https://example.com" }))["url"],
        "https://example.com"
    );

    let canonical = json!({ "path": "canonical", "filePath": "alias" });
    assert_eq!(normalize_tool_args("file_write", canonical)["path"], "canonical");
    assert_eq!(normalize_tool_args("file_read", json!("unchanged")), json!("unchanged"));
}

#[test]
fn default_render_hint_fills_missing_title_and_metadata() {
    let spec = ToolSpec::new("sample", "sample tool", json!({})).with_display_name("Sample Tool");
    let mut result = ToolCallResult::default();

    ensure_default_render_hint(&mut result, &spec);

    let hint = result.render_hint.expect("default hint should be inserted");
    assert_eq!(hint.title.as_deref(), Some("Sample Tool"));
    assert!(hint.metadata.is_object());

    let mut result = ToolCallResult {
        render_hint: Some(ToolRenderHint {
            title: None,
            metadata: Value::Null,
            ..ToolRenderHint::default()
        }),
        ..ToolCallResult::default()
    };
    ensure_default_render_hint(&mut result, &spec);
    let hint = result.render_hint.expect("existing hint should remain present");
    assert_eq!(hint.title.as_deref(), Some("Sample Tool"));
    assert!(hint.metadata.is_object());
}

#[test]
fn tool_call_errors_classify_and_expose_denials() {
    let denied = classify_message("Approval required for this action".to_string());
    assert!(matches!(&denied, ToolCallError::Denied { .. }));
    assert_eq!(denied.message(), "Approval required for this action");
    assert!(denied.permission_request().is_none());
    assert_eq!(denied.to_string(), denied.message());

    let failed = classify_anyhow_error(anyhow::anyhow!("ordinary failure"));
    assert!(matches!(&failed, ToolCallError::Failed(_)));
    assert_eq!(failed.message(), "ordinary failure");
    assert!(failed.permission_request().is_none());
}

#[test]
fn primary_path_and_numeric_inputs_accept_only_supported_values() {
    assert_eq!(primary_path(&json!({ "path": "a" })), Some("a"));
    assert_eq!(primary_path(&json!({ "file_path": "b" })), Some("b"));
    assert_eq!(primary_path(&json!({ "filePath": "c" })), Some("c"));
    assert_eq!(primary_path(&json!([])), None);
    assert_eq!(primary_path(&json!({ "path": 1 })), None);

    assert_eq!(input_value_usize(&json!({ "offset": 7 }), "offset"), Some(7));
    assert_eq!(input_value_usize(&json!({ "offset": -1 }), "offset"), None);
    assert_eq!(input_value_usize(&json!({}), "offset"), None);
}

#[test]
fn file_read_partial_view_handles_segment_kinds_and_unchanged_aliases() {
    let complete = ToolCallResult {
        data: json!({ "kind": "text", "total_lines": 4, "start_line": 1 }),
        ..ToolCallResult::default()
    };
    assert!(!file_read_partial_view(&complete));

    for data in [
        json!({ "kind": "text", "total_lines": 4, "start_line": 2 }),
        json!({ "kind": "pdf", "total_pages": 4, "start_page": 1, "has_more": true }),
        json!({
            "kind": "notebook",
            "total_cells": 4,
            "start_cell": 1,
            "truncated_by_bytes": true
        }),
        json!({ "kind": "file_unchanged", "partialView": true }),
    ] {
        assert!(file_read_partial_view(&ToolCallResult { data, ..ToolCallResult::default() }));
    }

    assert!(!segmented_read_partial(Some(0), Some(2), Some(true), Some(true)));
    assert!(!file_read_partial_view(&ToolCallResult {
        data: json!({ "kind": "binary" }),
        ..ToolCallResult::default()
    }));
}

#[test]
fn snapshot_for_input_path_reads_relative_absolute_and_missing_files() {
    let root = tempfile::TempDir::new().expect("tempdir should create");
    let file = root.path().join("snapshot.txt");
    std::fs::write(&file, b"snapshot").expect("fixture should write");

    assert!(
        snapshot_for_input_path(Some(root.path()), &json!({ "path": "snapshot.txt" })).is_some()
    );
    assert!(snapshot_for_input_path(None, &json!({ "path": file.to_string_lossy() })).is_some());
    assert!(snapshot_for_input_path(Some(root.path()), &json!({ "path": "missing" })).is_none());
    assert!(snapshot_for_input_path(Some(root.path()), &json!({})).is_none());
}

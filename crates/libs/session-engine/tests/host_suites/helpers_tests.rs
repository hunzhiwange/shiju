//! 验证工具筛选、ACP 判定以及敏感预览的清理与截断。

use super::*;
use serde_json::json;
use std::collections::HashSet;

#[test]
fn allowed_tool_ids_include_aliases_and_request_filter_intersects_base_set() {
    let base = allowed_tool_ids(None);
    assert!(!base.is_empty());

    let selected = base
        .iter()
        .find(|tool| !tool.eq_ignore_ascii_case("skill"))
        .expect("at least one registered non-skill tool")
        .clone();
    let filtered = allowed_tool_ids_for_request(
        None,
        &json!({"allowed_tools": [selected.clone(), "missing-tool", "  ", 42]}),
    );
    assert!(filtered.contains(&selected));
    let expected = base
        .iter()
        .filter(|tool| {
            tool.as_str() == selected || config_tools::is_builtin_required_tool_id(tool)
        })
        .cloned()
        .collect::<HashSet<_>>();
    assert_eq!(filtered, expected);

    assert_eq!(allowed_tool_ids_for_request(None, &json!({})), base);
    let empty_selection = allowed_tool_ids_for_request(None, &json!({"allowed_tools": ["  ", 42]}));
    assert!(
        empty_selection
            .iter()
            .all(|tool| { config_tools::is_builtin_required_tool_id(tool) })
    );
}

#[test]
fn tool_call_preview_sanitizes_large_or_sensitive_arguments() {
    let preview = tool_call_preview(
        "file_write",
        r#"{"path":"/tmp/example","content":"full file content should be omitted"}"#,
    );

    assert!(preview.starts_with("file_write("));
    assert!(preview.contains("<omitted"));
    assert!(!preview.contains("full file content should be omitted"));
}

#[test]
fn tool_call_preview_handles_empty_and_short_plain_arguments() {
    assert_eq!(tool_call_preview("file_read", "  "), "file_read()");
    assert_eq!(tool_call_preview("search", "  plain search query  "), "search(plain search query)");
}

#[test]
fn tool_call_preview_truncates_sanitized_arguments_at_character_boundary() {
    let preview = tool_call_preview("search", &"界".repeat(121));

    assert_eq!(preview, format!("search({}...)", "界".repeat(120)));
}

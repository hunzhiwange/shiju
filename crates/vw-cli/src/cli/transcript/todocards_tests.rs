//! Todo 工具卡片的数据解析与标识映射测试。
//! 校验工具族别名、任务状态符号、字段默认值与原始顺序。
//! 输入同时覆盖直接 JSON、JSON 字符串和类型不匹配的降级结果。

use super::super::theme::{ACCENT_CYAN, SUCCESS, TEXT_SUBTLE, WARNING};
use super::todocards::{
    parse_json_from_value_or_json_string, parse_todoread_card_data, parse_todos_from_array,
    parse_todowrite_card_data, todo_status_symbol, tool_badge_cli,
};
use ratatui::style::Color;
use serde_json::json;

#[test]
fn tool_badge_cli_maps_every_tool_family_and_alias() {
    let cases = [
        ("read", "[R]", SUCCESS),
        ("write", "[E]", WARNING),
        ("apply_patch", "[E]", WARNING),
        ("grep", "[S]", Color::Rgb(118, 176, 255)),
        ("content_search", "[S]", Color::Rgb(118, 176, 255)),
        ("glob", "[S]", Color::Rgb(118, 176, 255)),
        ("glob_search", "[S]", Color::Rgb(118, 176, 255)),
        ("lsp", "[S]", Color::Rgb(118, 176, 255)),
        ("codesearch", "[S]", Color::Rgb(118, 176, 255)),
        ("bash", "[B]", Color::Rgb(216, 150, 255)),
        ("shell", "[B]", Color::Rgb(216, 150, 255)),
        ("todowrite", "[T]", ACCENT_CYAN),
        ("todoread", "[T]", ACCENT_CYAN),
        ("unknown", "[U]", TEXT_SUBTLE),
    ];

    for (tool_name, expected_badge, expected_color) in cases {
        assert_eq!(tool_badge_cli(tool_name), (expected_badge, expected_color));
    }
}

#[test]
fn todo_status_symbol_maps_known_statuses_and_defaults_unknowns() {
    assert_eq!(todo_status_symbol("completed"), "✓");
    assert_eq!(todo_status_symbol("in_progress"), "·");
    assert_eq!(todo_status_symbol("pending"), "○");
    assert_eq!(todo_status_symbol("cancelled"), "○");
    assert_eq!(todo_status_symbol(""), "○");
}

#[test]
fn parse_todos_from_array_counts_trims_and_preserves_order() {
    let todos = vec![
        json!({"status": " completed ", "content": " done "}),
        json!({"status": "in_progress", "content": " running "}),
        json!({"status": "blocked", "content": " waiting "}),
        json!({"status": "  ", "content": "  "}),
    ];

    let data = parse_todos_from_array(&todos);

    assert_eq!(data.total, 4);
    assert_eq!(data.done, 1);
    assert_eq!(data.running, 1);
    assert_eq!(data.pending, 2);
    assert_eq!(
        data.items,
        vec![
            ("completed".to_string(), "done".to_string()),
            ("in_progress".to_string(), "running".to_string()),
            ("blocked".to_string(), "waiting".to_string()),
            (String::new(), String::new()),
        ]
    );
}

#[test]
fn parse_todos_from_array_defaults_missing_and_non_string_fields() {
    let todos = vec![json!({}), json!({"status": null, "content": 9}), json!("unexpected")];

    let data = parse_todos_from_array(&todos);

    assert_eq!(data.total, 3);
    assert_eq!(data.done, 0);
    assert_eq!(data.running, 0);
    assert_eq!(data.pending, 3);
    assert_eq!(
        data.items,
        vec![
            ("pending".to_string(), "(empty)".to_string()),
            ("pending".to_string(), "(empty)".to_string()),
            ("pending".to_string(), "(empty)".to_string()),
        ]
    );
}

#[test]
fn parse_todos_from_array_returns_default_for_empty_input() {
    let data = parse_todos_from_array(&[]);

    assert_eq!(data.total, 0);
    assert_eq!(data.done, 0);
    assert_eq!(data.running, 0);
    assert_eq!(data.pending, 0);
    assert_eq!(data.items, Vec::new());
}

#[test]
fn parse_json_from_value_or_json_string_handles_encoded_direct_and_invalid_values() {
    let encoded = json!("  {\"todos\":[1,2]}  ");
    let direct = json!({"todos": []});

    assert_eq!(parse_json_from_value_or_json_string(&encoded).unwrap(), json!({"todos": [1, 2]}));
    assert_eq!(parse_json_from_value_or_json_string(&direct).unwrap(), direct);
    assert_eq!(parse_json_from_value_or_json_string(&json!(null)).unwrap(), json!(null));
    assert!(parse_json_from_value_or_json_string(&json!("not-json")).is_none());
    assert!(parse_json_from_value_or_json_string(&json!("   ")).is_none());
}

#[test]
fn parse_todowrite_card_data_accepts_direct_and_encoded_inputs() {
    let direct_payload = json!({
        "input": {
            "todos": [
                {"status": "completed", "content": "ship"},
                {"status": "pending", "content": "review"}
            ]
        }
    })
    .to_string();
    let encoded_payload = json!({
        "input": "  {\"todos\":[{\"status\":\"in_progress\",\"content\":\"verify\"}]}  "
    })
    .to_string();

    let direct = parse_todowrite_card_data(&direct_payload).unwrap();
    let encoded = parse_todowrite_card_data(&encoded_payload).unwrap();

    assert_eq!(direct.total, 2);
    assert_eq!(direct.done, 1);
    assert_eq!(direct.pending, 1);
    assert_eq!(encoded.total, 1);
    assert_eq!(encoded.running, 1);
    assert_eq!(encoded.items[0], ("in_progress".to_string(), "verify".to_string()));
}

#[test]
fn parse_todowrite_card_data_rejects_malformed_or_mismatched_payloads() {
    let invalid_payloads = [
        "not-json",
        "{}",
        "[]",
        r#"{"input":"not-json"}"#,
        r#"{"input":null}"#,
        r#"{"input":{}}"#,
        r#"{"input":{"todos":{}}}"#,
    ];

    for payload in invalid_payloads {
        assert!(parse_todowrite_card_data(payload).is_none(), "payload: {payload}");
    }
}

#[test]
fn parse_todoread_card_data_accepts_direct_and_encoded_arrays() {
    let direct_payload = json!({
        "output": [
            {"status": "completed", "content": "ship"},
            {"status": "pending", "content": "review"}
        ]
    })
    .to_string();
    let encoded_payload = json!({
        "output": "  [{\"status\":\"in_progress\",\"content\":\"verify\"}]  "
    })
    .to_string();

    let direct = parse_todoread_card_data(&direct_payload).unwrap();
    let encoded = parse_todoread_card_data(&encoded_payload).unwrap();

    assert_eq!(direct.total, 2);
    assert_eq!(direct.done, 1);
    assert_eq!(direct.pending, 1);
    assert_eq!(encoded.total, 1);
    assert_eq!(encoded.running, 1);
    assert_eq!(encoded.items[0], ("in_progress".to_string(), "verify".to_string()));
}

#[test]
fn parse_todoread_card_data_rejects_malformed_or_mismatched_payloads() {
    let invalid_payloads = [
        "not-json",
        "{}",
        "[]",
        r#"{"output":"not-json"}"#,
        r#"{"output":null}"#,
        r#"{"output":{}}"#,
        r#"{"output":"{}"}"#,
    ];

    for payload in invalid_payloads {
        assert!(parse_todoread_card_data(payload).is_none(), "payload: {payload}");
    }
}

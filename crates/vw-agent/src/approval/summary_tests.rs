//! 测试审批参数摘要的格式化、截断与非对象载荷处理。

use super::summary::summarize_args;

#[test]
fn summarize_args_formats_objects_and_truncates_long_strings() {
    let summary = summarize_args(&serde_json::json!({
        "path": "src/main.rs",
        "content": "x".repeat(100)
    }));

    assert!(summary.contains("path: src/main.rs"));
    assert!(summary.contains("content: "));
    assert!(summary.contains('…'));
}

#[test]
fn summarize_args_keeps_short_values_untruncated() {
    let exact = "x".repeat(80);
    let summary = summarize_args(&serde_json::json!({
        "content": exact,
        "count": 3,
        "enabled": true,
        "missing": null
    }));

    assert!(summary.contains(&format!("content: {}", "x".repeat(80))));
    assert!(summary.contains("count: 3"));
    assert!(summary.contains("enabled: true"));
    assert!(summary.contains("missing: null"));
    assert!(!summary.contains('…'));
}

#[test]
fn summarize_args_formats_and_truncates_non_object_values() {
    assert_eq!(summarize_args(&serde_json::json!(true)), "true");
    assert_eq!(summarize_args(&serde_json::json!("hello")), "\"hello\"");

    let summary = summarize_args(&serde_json::json!(["x".repeat(200)]));

    assert!(summary.starts_with("[\""));
    assert!(summary.contains('…'));
}

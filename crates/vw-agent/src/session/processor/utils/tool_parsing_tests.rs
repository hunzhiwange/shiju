//! 验证工具名称及内联、多行和文件引用形式的调用解析边界。

use super::{is_valid_tool_name, parse_tool_at, query_has_any_tool_calls_with_allowed};
use std::collections::HashSet;

fn allowed(names: &[&str]) -> HashSet<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

#[test]
fn validates_tool_names_with_expected_character_set() {
    assert!(is_valid_tool_name("read_file"));
    assert!(is_valid_tool_name("Tool123"));
    assert!(is_valid_tool_name(&"a".repeat(32)));
    assert!(!is_valid_tool_name(""));
    assert!(!is_valid_tool_name(&"a".repeat(33)));
    assert!(!is_valid_tool_name("bad-name"));
    assert!(!is_valid_tool_name("tool.name"));
    assert!(!is_valid_tool_name("tool/name"));
}

#[test]
fn parses_inline_json_tool_call_only_when_allowed_and_valid() {
    let tools = allowed(&["read"]);
    let lines = ["/read {\"path\":\"Cargo.toml\"}"];

    assert_eq!(
        parse_tool_at(&lines, 0, &tools),
        Some(("read".to_string(), "{\"path\":\"Cargo.toml\"}".to_string(), 1))
    );
    assert!(!query_has_any_tool_calls_with_allowed("/write {}", &tools));
    assert_eq!(parse_tool_at(&["/read {not-json}"], 0, &tools), None);
}

#[test]
fn parses_multiline_json_after_blank_lines() {
    let tools = allowed(&["write"]);
    let lines = ["/write", "", "  {", "    \"path\": \"a.txt\",", "    \"content\": \"hi\"", "  }"];

    assert_eq!(
        parse_tool_at(&lines, 0, &tools),
        Some((
            "write".to_string(),
            "{\n    \"path\": \"a.txt\",\n    \"content\": \"hi\"\n  }".to_string(),
            6,
        ))
    );
}

#[test]
fn parses_read_style_at_file_reference_within_scan_window() {
    let tools = allowed(&["read", "file_read"]);
    let lines =
        ["/read", "", "not a file ref", "@ crates/vw-agent/src/session/prompt.rs ", "/write {}"];

    assert_eq!(
        parse_tool_at(&lines, 0, &tools),
        Some(("read".to_string(), "crates/vw-agent/src/session/prompt.rs".to_string(), 4,))
    );
}

#[test]
fn read_style_scan_stops_at_next_tool_call() {
    let tools = allowed(&["read", "write"]);
    let lines = ["/read", "/write {}", "@ must-not-be-consumed.txt"];

    assert_eq!(parse_tool_at(&lines, 0, &tools), Some(("read".to_string(), String::new(), 1)));
    assert_eq!(parse_tool_at(&lines, 1, &tools), Some(("write".to_string(), "{}".to_string(), 1)));
}

#[test]
fn returns_empty_args_when_no_inline_or_json_payload() {
    let tools = allowed(&["sleep"]);
    let lines = ["/sleep", "plain text that belongs to the user"];

    assert_eq!(parse_tool_at(&lines, 0, &tools), Some(("sleep".to_string(), String::new(), 1)));
}

#[test]
fn rejects_bad_starts_unknown_tools_and_unclosed_json_payloads() {
    let tools = allowed(&["write"]);

    assert_eq!(parse_tool_at(&["not a call"], 0, &tools), None);
    assert_eq!(parse_tool_at(&["/"], 0, &tools), None);
    assert_eq!(parse_tool_at(&["/bad-name {}"], 0, &tools), None);
    assert_eq!(parse_tool_at(&["/unknown {}"], 0, &tools), None);
    assert_eq!(parse_tool_at(&["/write", "{", "\"path\":\"a.txt\""], 0, &tools), None);
    assert!(!query_has_any_tool_calls_with_allowed("hello\n/write {bad}", &tools));
}

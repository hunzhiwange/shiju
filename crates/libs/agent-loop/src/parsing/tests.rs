//! 验证工具调用解析器对兼容格式的参数归一化。
//!
//! 这些测试覆盖结构化 provider 输出和 GLM 缩写格式，确保历史别名工具在只给出
//! 字符串参数时仍能映射到当前工具 schema。

use super::{parse_glm_shortened_body, parse_tool_call_value};
use serde_json::json;

#[test]
fn structured_grep_string_arguments_become_pattern() {
    // grep 的旧格式可能只传字符串；归一化为 pattern 能复用当前工具定义。
    let value = json!({
        "function": {
            "name": "grep",
            "arguments": "TODO"
        }
    });

    let parsed = parse_tool_call_value(&value).expect("grep call should parse");

    assert_eq!(parsed.name, "grep");
    assert_eq!(parsed.arguments, json!({ "pattern": "TODO" }));
}

#[test]
fn structured_codesearch_string_arguments_become_query() {
    // codesearch 是历史别名，字符串参数应落到 query，避免旧 provider 输出失效。
    let value = json!({
        "function": {
            "name": "codesearch",
            "arguments": "Rust async"
        }
    });

    let parsed = parse_tool_call_value(&value).expect("legacy codesearch call should parse");

    assert_eq!(parsed.name, "codesearch");
    assert_eq!(parsed.arguments, json!({ "query": "Rust async" }));
}

#[test]
fn glm_shortened_grep_uses_pattern_default() {
    // GLM 缩写格式没有显式 JSON 字段名，因此解析器需要按工具名补默认字段。
    let parsed = parse_glm_shortened_body("grep>needle").expect("glm grep call should parse");

    assert_eq!(parsed.name, "grep");
    assert_eq!(parsed.arguments, json!({ "pattern": "needle" }));
}

#[test]
fn glm_shortened_legacy_codesearch_alias_uses_query_default() {
    // 兼容 legacy 名称可以让旧 prompt/模型输出继续被工具循环消费。
    let parsed = parse_glm_shortened_body("codesearch>symbol search")
        .expect("glm legacy codesearch call should parse");

    assert_eq!(parsed.name, "codesearch");
    assert_eq!(parsed.arguments, json!({ "query": "symbol search" }));
}

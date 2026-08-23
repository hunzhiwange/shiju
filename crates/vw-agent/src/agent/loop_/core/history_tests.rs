//! 测试工具规格和工具调用写入供应商历史时的规范化行为。

use super::*;
use crate::app::agent::tools::{ToolResult, ToolSpec};
use async_trait::async_trait;
use serde_json::Value;

struct Testcov0084Tool;

#[async_trait]
impl Tool for Testcov0084Tool {
    fn name(&self) -> &str {
        "testcov_0084"
    }

    fn description(&self) -> &str {
        "history coverage tool"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult { success: true, output: String::new(), error: None })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
    }
}

#[test]
fn autosave_memory_key_uses_prefix_and_unique_suffix() {
    let first = autosave_memory_key("chat");
    let second = autosave_memory_key("chat");

    assert!(first.starts_with("chat_"));
    assert!(second.starts_with("chat_"));
    assert_ne!(first, second);
}

#[test]
fn autosave_memory_key_allows_empty_prefix() {
    let key = autosave_memory_key("");

    assert!(key.starts_with('_'));
    assert!(key.len() > 1);
}

#[test]
fn tools_to_openai_format_maps_tool_specs() {
    let tools: Vec<Box<dyn Tool>> = vec![Box::new(Testcov0084Tool)];

    let formatted = tools_to_openai_format(&tools);

    assert_eq!(formatted.len(), 1);
    assert_eq!(formatted[0]["type"], "function");
    assert_eq!(formatted[0]["function"]["name"], "testcov_0084");
    assert_eq!(formatted[0]["function"]["description"], "history coverage tool");
    assert_eq!(formatted[0]["function"]["parameters"]["required"][0], "query");
}

#[test]
fn tools_to_openai_format_returns_empty_for_empty_registry() {
    let tools: Vec<Box<dyn Tool>> = Vec::new();

    assert!(tools_to_openai_format(&tools).is_empty());
}

#[test]
fn native_assistant_history_trims_content_and_keeps_tool_arguments() {
    let calls = vec![ToolCall {
        id: "call-testcov-0084".to_string(),
        name: "shell".to_string(),
        arguments: r#"{"command":"pwd"}"#.to_string(),
    }];

    let history =
        build_native_assistant_history("  completed  ", &calls, Some("reasoning-testcov-0084"));
    let value: Value = serde_json::from_str(&history).expect("history should be valid json");

    assert_eq!(value["content"], "completed");
    assert_eq!(value["tool_calls"][0]["id"], "call-testcov-0084");
    assert_eq!(value["tool_calls"][0]["name"], "shell");
    assert_eq!(value["tool_calls"][0]["arguments"], r#"{"command":"pwd"}"#);
    assert_eq!(value["reasoning_content"], "reasoning-testcov-0084");
}

#[test]
fn native_assistant_history_uses_null_for_blank_content_without_reasoning() {
    let history = build_native_assistant_history(" \n\t ", &[], None);
    let value: Value = serde_json::from_str(&history).expect("history should be valid json");

    assert!(value["content"].is_null());
    assert!(value["tool_calls"].as_array().expect("tool calls should be an array").is_empty());
    assert!(value.get("reasoning_content").is_none());
}

#[test]
fn parsed_tool_calls_build_native_assistant_history() {
    let calls = vec![ParsedToolCall {
        name: "shell".to_string(),
        arguments: serde_json::json!({"command": "pwd"}),
        tool_call_id: Some("call-1".to_string()),
    }];

    let history = build_native_assistant_history_from_parsed_calls("text", &calls, Some("think"))
        .expect("parsed calls should build history");

    assert!(history.contains("call-1"));
    assert!(history.contains("shell"));
    assert!(history.contains("think"));
}

#[test]
fn parsed_tool_calls_build_history_with_empty_calls() {
    let history = build_native_assistant_history_from_parsed_calls("answer", &[], Some("reason"))
        .expect("empty parsed calls should build history");
    let value: Value = serde_json::from_str(&history).expect("history should be valid json");

    assert_eq!(value["content"], "answer");
    assert!(value["tool_calls"].as_array().expect("tool calls should be an array").is_empty());
    assert_eq!(value["reasoning_content"], "reason");
}

#[test]
fn parsed_tool_calls_serialize_arguments_and_blank_content() {
    let calls = vec![ParsedToolCall {
        name: "search".to_string(),
        arguments: serde_json::json!({"query": "testcov-0084", "limit": 2}),
        tool_call_id: Some("parsed-call-0084".to_string()),
    }];

    let history = build_native_assistant_history_from_parsed_calls("   ", &calls, None)
        .expect("parsed calls with ids should build history");
    let value: Value = serde_json::from_str(&history).expect("history should be valid json");

    assert!(value["content"].is_null());
    assert_eq!(value["tool_calls"][0]["id"], "parsed-call-0084");
    assert_eq!(value["tool_calls"][0]["name"], "search");

    let arguments: Value =
        serde_json::from_str(value["tool_calls"][0]["arguments"].as_str().unwrap())
            .expect("arguments should be serialized json");
    assert_eq!(arguments["query"], "testcov-0084");
    assert_eq!(arguments["limit"], 2);
    assert!(value.get("reasoning_content").is_none());
}

#[test]
fn parsed_tool_calls_without_id_return_none() {
    let calls = vec![ParsedToolCall {
        name: "shell".to_string(),
        arguments: serde_json::json!({}),
        tool_call_id: None,
    }];

    assert!(build_native_assistant_history_from_parsed_calls("text", &calls, None).is_none());
}

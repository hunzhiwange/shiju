//! `SessionLlmProvider` 的消息投影与用量解析测试。
//!
//! 这些测试聚焦 provider 和 gateway 边界的数据转换，避免直接依赖真实 gateway 进程。

use super::SessionLlmProvider;
use crate::app::agent::providers::{ChatMessage, Provider};

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn parse_usage_maps_gateway_done_usage() {
    let payload = serde_json::json!({
        "input_tokens": 120,
        "output_tokens": 45,
        "cached_tokens": 30,
        "reasoning_tokens": 12,
    });

    let usage = SessionLlmProvider::parse_usage(Some(&payload)).expect("usage should parse");

    assert_eq!(usage.input_tokens, Some(120));
    assert_eq!(usage.output_tokens, Some(45));
    assert_eq!(usage.cached_tokens, Some(30));
    assert_eq!(usage.reasoning_tokens, Some(12));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn parse_usage_returns_none_without_payload() {
    assert!(SessionLlmProvider::parse_usage(None).is_none());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn parse_usage_returns_none_for_invalid_payload() {
    let payload = serde_json::json!({"input_tokens": "many"});
    assert!(SessionLlmProvider::parse_usage(Some(&payload)).is_none());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn parse_usage_defaults_missing_reasoning_tokens_to_zero() {
    let payload = serde_json::json!({
        "input_tokens": 8,
        "output_tokens": 5,
        "cached_tokens": 3,
    });

    let usage = SessionLlmProvider::parse_usage(Some(&payload)).expect("usage should parse");

    assert_eq!(usage.input_tokens, Some(8));
    assert_eq!(usage.output_tokens, Some(5));
    assert_eq!(usage.cached_tokens, Some(3));
    assert_eq!(usage.reasoning_tokens, Some(0));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn parse_usage_rejects_negative_fields_independently() {
    let payload = serde_json::json!({
        "input_tokens": -1,
        "output_tokens": 5,
        "cached_tokens": -3,
        "reasoning_tokens": -2,
    });

    let usage = SessionLlmProvider::parse_usage(Some(&payload)).expect("usage should parse");

    assert_eq!(usage.input_tokens, None);
    assert_eq!(usage.output_tokens, Some(5));
    assert_eq!(usage.cached_tokens, None);
    assert_eq!(usage.reasoning_tokens, None);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn parse_usage_preserves_zero_token_counts() {
    let payload = serde_json::json!({
        "input_tokens": 0,
        "output_tokens": 0,
        "cached_tokens": 0,
        "reasoning_tokens": 0,
    });

    let usage = SessionLlmProvider::parse_usage(Some(&payload)).expect("usage should parse");

    assert_eq!(usage.input_tokens, Some(0));
    assert_eq!(usage.output_tokens, Some(0));
    assert_eq!(usage.cached_tokens, Some(0));
    assert_eq!(usage.reasoning_tokens, Some(0));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn parse_usage_rejects_fractional_token_counts() {
    let payload = serde_json::json!({
        "input_tokens": 1.5,
        "output_tokens": 2,
        "cached_tokens": 0,
        "reasoning_tokens": 0,
    });

    assert!(SessionLlmProvider::parse_usage(Some(&payload)).is_none());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn build_messages_skips_blank_system_prompt() {
    let without_system = SessionLlmProvider::build_messages(Some("   "), "hello");
    assert_eq!(without_system.len(), 1);
    assert_eq!(without_system[0]["role"], "user");

    let with_system = SessionLlmProvider::build_messages(Some("be kind"), "hello");
    assert_eq!(with_system.len(), 2);
    assert_eq!(with_system[0]["role"], "system");
    assert_eq!(with_system[1]["content"], "hello");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn build_messages_without_system_still_emits_empty_user_message() {
    let messages = SessionLlmProvider::build_messages(None, "");

    assert_eq!(messages, vec![serde_json::json!({"role": "user", "content": ""})]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn build_messages_preserves_nonblank_system_prompt_verbatim() {
    let messages = SessionLlmProvider::build_messages(Some("  keep spacing  "), "hello");

    assert_eq!(messages[0]["content"], "  keep spacing  ");
    assert_eq!(messages[1]["content"], "hello");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn build_history_messages_preserves_supported_roles() {
    let messages = vec![
        ChatMessage::system("sys"),
        ChatMessage::user("u1"),
        ChatMessage::assistant("a1"),
        ChatMessage::user("u2"),
    ];

    let projected = SessionLlmProvider::build_history_messages(&messages);

    assert_eq!(projected.len(), 4);
    assert_eq!(projected[0].get("role").and_then(serde_json::Value::as_str), Some("system"));
    assert_eq!(projected[1].get("role").and_then(serde_json::Value::as_str), Some("user"));
    assert_eq!(projected[2].get("role").and_then(serde_json::Value::as_str), Some("assistant"));
    assert_eq!(projected[3].get("content").and_then(serde_json::Value::as_str), Some("u2"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn build_history_messages_preserves_tool_role_for_gateway_boundary() {
    // 工具消息内容通常包含结构化字段，投影时必须原样保留给 gateway 继续解释。
    let tool_message = ChatMessage::tool(
        serde_json::json!({
            "tool_call_id": "call_123",
            "content": "pwd\n/Users/demo"
        })
        .to_string(),
    );

    let projected = SessionLlmProvider::build_history_messages(&[tool_message]);

    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].get("role").and_then(serde_json::Value::as_str), Some("tool"));
    let content = projected[0]
        .get("content")
        .and_then(serde_json::Value::as_str)
        .expect("tool projection should keep content");
    assert!(content.contains("tool_call_id"));
    assert!(content.contains("/Users/demo"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn build_history_messages_skips_unknown_roles() {
    let messages = vec![ChatMessage { role: "alien".to_string(), content: "???".to_string() }];
    assert!(SessionLlmProvider::build_history_messages(&messages).is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn build_history_messages_skips_unknown_roles_without_dropping_neighbors() {
    let messages = vec![
        ChatMessage::user("before"),
        ChatMessage { role: "alien".to_string(), content: "ignored".to_string() },
        ChatMessage::assistant("after"),
    ];

    let projected = SessionLlmProvider::build_history_messages(&messages);

    assert_eq!(
        projected,
        vec![
            serde_json::json!({"role": "user", "content": "before"}),
            serde_json::json!({"role": "assistant", "content": "after"}),
        ]
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn build_history_messages_preserves_empty_supported_content() {
    let messages = vec![ChatMessage::system(""), ChatMessage::user("")];

    let projected = SessionLlmProvider::build_history_messages(&messages);

    assert_eq!(
        projected,
        vec![
            serde_json::json!({"role": "system", "content": ""}),
            serde_json::json!({"role": "user", "content": ""}),
        ]
    );
}

#[test]
fn extract_system_and_last_user_uses_first_system_and_last_user() {
    let messages = vec![
        ChatMessage::system("sys1"),
        ChatMessage::user("first"),
        ChatMessage::system("sys2"),
        ChatMessage::assistant("answer"),
        ChatMessage::user("last"),
    ];

    let (system, last_user) = SessionLlmProvider::extract_system_and_last_user(&messages);

    assert_eq!(system, Some("sys1"));
    assert_eq!(last_user, "last");
}

#[test]
fn extract_system_and_last_user_returns_empty_fallbacks_without_matching_roles() {
    let messages = vec![ChatMessage::assistant("answer"), ChatMessage::tool("result")];

    let (system, last_user) = SessionLlmProvider::extract_system_and_last_user(&messages);

    assert_eq!(system, None);
    assert_eq!(last_user, "");
}

#[test]
fn extract_system_and_last_user_handles_empty_history() {
    let (system, last_user) = SessionLlmProvider::extract_system_and_last_user(&[]);

    assert_eq!(system, None);
    assert_eq!(last_user, "");
}

#[test]
fn extract_system_and_last_user_preserves_blank_matching_messages() {
    let messages = vec![ChatMessage::system("   "), ChatMessage::user("")];

    let (system, last_user) = SessionLlmProvider::extract_system_and_last_user(&messages);

    assert_eq!(system, Some("   "));
    assert_eq!(last_user, "");
}

#[test]
fn session_provider_declares_fallback_capabilities_and_streaming() {
    let provider = SessionLlmProvider::new();
    let caps = provider.capabilities();
    assert!(!caps.native_tool_calling);
    assert!(!caps.vision);
    assert!(!provider.supports_native_tools());
    assert!(!provider.supports_vision());
    assert!(provider.supports_streaming());
}

//! 验证 AI SDK 消息、工具和响应内容之间的双向转换。

use serde_json::json;

use crate::app::agent::provider::provider;

use super::compute_request_info;

fn test_model(
    provider_id: &str,
    adapter: &str,
    api_url: &str,
    model_id: &str,
    name: &str,
) -> provider::Model {
    serde_json::from_value(json!({
        "id": model_id,
        "providerID": provider_id,
        "api": {
            "id": model_id,
            "url": api_url,
            "adapter": adapter
        },
        "name": name,
        "family": null,
        "capabilities": {
            "temperature": true,
            "reasoning": true,
            "attachment": false,
            "toolcall": true,
            "input": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "output": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "interleaved": false
        },
        "cost": {
            "input": 0.0,
            "output": 0.0,
            "cache": {
                "read": 0.0,
                "write": 0.0
            },
            "experimental_over_200k": null
        },
        "limit": {
            "context": 8192,
            "input": null,
            "output": 4096
        },
        "status": "active",
        "options": {},
        "headers": {},
        "release_date": "2026-01-01",
        "variants": {}
    }))
    .expect("test model should deserialize")
}

fn chat_messages() -> serde_json::Value {
    json!([
        {"role": "system", "content": "system prompt"},
        {
            "role": "user",
            "content": [
                {"type": "text", "text": "hello"},
                {"type": "image_url", "image_url": {"url": "ignored"}}
            ]
        },
        {"role": "assistant", "content": "hi"}
    ])
}

#[test]
fn openai_gpt_url_gets_v1_chat_completions_and_strict_tools() {
    let model = test_model("openai", "openai", " https://api.example.com ", "gpt-5", "GPT 5");

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "https://api.example.com/v1");
    assert_eq!(info.request_url, "https://api.example.com/v1/chat/completions");
    assert_eq!(info.path_override.as_deref(), Some("chat/completions"));
    assert!(info.enforce_strict_tool_schema);
    assert_eq!(info.messages.len(), 3);
}

#[test]
fn compatible_endpoint_urls_are_split_before_query_and_fragment() {
    let model = test_model(
        "openai",
        "openai-compatible",
        "https://api.example.com/v2/responses?api-version=1#frag",
        "gpt-5-mini",
        "GPT 5 mini",
    );

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "https://api.example.com/v2");
    assert_eq!(info.request_url, "https://api.example.com/v2/responses");
    assert_eq!(info.path_override.as_deref(), Some("responses"));
    assert!(info.enforce_strict_tool_schema);
}

#[test]
fn chat_stream_endpoint_maps_to_chat_completions_path_override() {
    let model = test_model(
        "copilot",
        "agent-client-protocol",
        "https://agent.example.com/chat/stream/",
        "gpt-agent",
        "agent",
    );

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "https://agent.example.com/v1");
    assert_eq!(info.request_url, "https://agent.example.com/v1/chat/completions");
    assert_eq!(info.path_override.as_deref(), Some("chat/completions"));
    assert!(info.enforce_strict_tool_schema);
}

#[test]
fn explicit_chat_completions_endpoint_preserves_versioned_base() {
    let model =
        test_model("openai", "acp", "https://api.example.com/v3/chat/completions", "gpt-5", "gpt");

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "https://api.example.com/v3");
    assert_eq!(info.request_url, "https://api.example.com/v3/chat/completions");
    assert_eq!(info.path_override.as_deref(), Some("chat/completions"));
}

#[test]
fn compatible_non_gpt_model_adds_v1_without_strict_schema_or_path_override() {
    let model = test_model(
        "custom",
        "openai-compatible",
        "https://api.example.com/root/",
        "claude-sonnet",
        "Claude Sonnet",
    );

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "https://api.example.com/root/v1");
    assert_eq!(info.request_url, "https://api.example.com/root/v1/chat/completions");
    assert_eq!(info.path_override, None);
    assert!(!info.enforce_strict_tool_schema);
}

#[test]
fn non_openai_adapter_uses_original_url_and_default_chat_completions_request() {
    let model = test_model(
        "anthropic",
        "anthropic",
        "https://anthropic.example.com/v1/messages",
        "claude-4",
        "Claude 4",
    );

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "https://anthropic.example.com/v1/messages");
    assert_eq!(info.request_url, "https://anthropic.example.com/v1/messages/chat/completions");
    assert_eq!(info.path_override, None);
    assert!(!info.enforce_strict_tool_schema);
}

#[test]
fn empty_or_non_array_messages_produce_empty_aisdk_messages() {
    let model = test_model("openai", "openai", "https://api.example.com", "gpt-5", "gpt");

    let info = compute_request_info(&model, &json!({"role": "user"})).expect("request info");

    assert!(info.messages.is_empty());
    assert_eq!(info.request_url, "https://api.example.com/v1/chat/completions");
}

#[test]
fn agent_client_protocol_underscore_variant_is_openai_compatible() {
    let model =
        test_model("custom", "agent_client_protocol", "https://api.example.com", "gpt-5", "GPT 5");

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "https://api.example.com/v1");
    assert_eq!(info.request_url, "https://api.example.com/v1/chat/completions");
    assert_eq!(info.path_override.as_deref(), Some("chat/completions"));
    assert!(info.enforce_strict_tool_schema);
}

#[test]
fn gpt_in_name_only_detects_gpt_like() {
    let model = test_model(
        "custom",
        "openai-compatible",
        "https://api.example.com/v1/",
        "custom-model",
        "GPT-4o Custom",
    );

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert!(info.enforce_strict_tool_schema);
    assert_eq!(info.base_url, "https://api.example.com/v1");
    assert_eq!(info.path_override.as_deref(), Some("chat/completions"));
}

#[test]
fn bare_domain_openai_compatible_non_gpt_adds_v1() {
    let model = test_model(
        "custom",
        "openai-compatible",
        "https://api.example.com",
        "claude-sonnet",
        "Claude Sonnet",
    );

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "https://api.example.com/v1");
    assert_eq!(info.request_url, "https://api.example.com/v1/chat/completions");
    assert_eq!(info.path_override, None);
    assert!(!info.enforce_strict_tool_schema);
}

#[test]
fn responses_endpoint_clean_url_sets_responses_path_override() {
    let model = test_model(
        "custom",
        "openai-compatible",
        "https://api.example.com/v2/responses",
        "gpt-5",
        "GPT-5",
    );

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "https://api.example.com/v2");
    assert_eq!(info.request_url, "https://api.example.com/v2/responses");
    assert_eq!(info.path_override.as_deref(), Some("responses"));
    assert!(info.enforce_strict_tool_schema);
}

#[test]
fn openai_compatible_already_has_v1_respects_existing_version() {
    let model = test_model(
        "custom",
        "openai-compatible",
        "https://api.example.com/v1",
        "claude-sonnet",
        "Claude Sonnet",
    );

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "https://api.example.com/v1");
    assert_eq!(info.request_url, "https://api.example.com/v1/chat/completions");
    assert_eq!(info.path_override, None);
    assert!(!info.enforce_strict_tool_schema);
}

#[test]
fn empty_messages_array_produces_empty_aisdk_messages() {
    let model = test_model("openai", "openai", "https://api.example.com", "gpt-5", "gpt");

    let info = compute_request_info(&model, &json!([])).expect("request info");

    assert!(info.messages.is_empty());
    assert_eq!(info.request_url, "https://api.example.com/v1/chat/completions");
}

#[test]
fn chat_completions_endpoint_without_version_prefix() {
    let model = test_model(
        "openai",
        "openai-compatible",
        "https://api.example.com/chat/completions",
        "gpt-5",
        "GPT-5",
    );

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "https://api.example.com/v1");
    assert_eq!(info.request_url, "https://api.example.com/v1/chat/completions");
    assert_eq!(info.path_override.as_deref(), Some("chat/completions"));
    assert!(info.enforce_strict_tool_schema);
}

#[test]
fn adapter_whitespace_is_trimmed_before_compatibility_detection() {
    let model = test_model(
        "custom",
        "  openai  ",
        "https://api.example.com",
        "claude-sonnet",
        "Claude Sonnet",
    );

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "https://api.example.com/v1");
    assert_eq!(info.request_url, "https://api.example.com/v1/chat/completions");
    assert_eq!(info.path_override, None);
    assert!(!info.enforce_strict_tool_schema);
}

#[test]
fn gpt_in_model_id_only_enables_strict_schema_and_default_override() {
    let model = test_model(
        "custom",
        "openai-compatible",
        "https://api.example.com/v1",
        "vendor-gpt-model",
        "Custom Model",
    );

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert!(info.enforce_strict_tool_schema);
    assert_eq!(info.path_override.as_deref(), Some("chat/completions"));
    assert_eq!(info.request_url, "https://api.example.com/v1/chat/completions");
}

#[test]
fn non_openai_gpt_model_does_not_enable_strict_schema_or_override() {
    let model = test_model(
        "custom",
        "anthropic",
        "https://api.example.com/v1/messages",
        "gpt-proxy",
        "GPT Proxy",
    );

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert!(!info.enforce_strict_tool_schema);
    assert_eq!(info.path_override, None);
    assert_eq!(info.request_url, "https://api.example.com/v1/messages/chat/completions");
}

#[test]
fn non_numeric_version_suffix_gets_v1_appended() {
    let model = test_model(
        "custom",
        "openai-compatible",
        "https://api.example.com/v1beta",
        "claude-sonnet",
        "Claude Sonnet",
    );

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "https://api.example.com/v1beta/v1");
    assert_eq!(info.request_url, "https://api.example.com/v1beta/v1/chat/completions");
    assert_eq!(info.path_override, None);
}

#[test]
fn compatible_non_gpt_explicit_responses_endpoint_keeps_override_without_strict_schema() {
    let model = test_model(
        "custom",
        "openai-compatible",
        "https://api.example.com/v2/responses/",
        "claude-sonnet",
        "Claude Sonnet",
    );

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "https://api.example.com/v2");
    assert_eq!(info.request_url, "https://api.example.com/v2/responses");
    assert_eq!(info.path_override.as_deref(), Some("responses"));
    assert!(!info.enforce_strict_tool_schema);
}

#[test]
fn empty_non_openai_url_keeps_empty_base_and_builds_relative_default_request() {
    let model = test_model("custom", "anthropic", "   ", "claude-sonnet", "Claude Sonnet");

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "");
    assert_eq!(info.request_url, "/chat/completions");
    assert_eq!(info.path_override, None);
    assert!(!info.enforce_strict_tool_schema);
}

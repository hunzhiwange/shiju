//! Request endpoint decisions and model capability gates.

use model_types as provider;
use serde_json::json;

#[path = "convert_test_support.rs"]
mod fixtures;
use fixtures::test_model;

fn compute_request_info(
    model: &provider::Model,
    messages: &serde_json::Value,
) -> Result<super::AisdkRequestInfo, crate::Error> {
    super::compute_request_info(model, messages, &json!({}), false)
}

fn chat_messages() -> serde_json::Value {
    json!([
        {"role": "system", "content": "system prompt"},
        {
            "role": "user",
            "content": [
                {"type": "text", "text": "hello"}
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
// 端点规范化只修改路径，查询参数和片段不能参与基础地址拼接。
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
fn anthropic_adapter_uses_native_messages_endpoint() {
    let model = test_model(
        "anthropic",
        "anthropic",
        "https://anthropic.example.com/v1/messages",
        "claude-4",
        "Claude 4",
    );

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "https://anthropic.example.com/v1");
    assert_eq!(info.request_url, "https://anthropic.example.com/v1/messages");
    assert_eq!(info.path_override.as_deref(), Some("messages"));
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
// 仅兼容端点上的 GPT 模型启用严格工具模式，名称相似不足以跨供应商套用。
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
fn anthropic_adapter_preserves_native_protocol_for_gpt_named_proxy() {
    let model = test_model(
        "custom",
        "anthropic",
        "https://api.example.com/v1/messages",
        "gpt-proxy",
        "GPT Proxy",
    );

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert!(!info.enforce_strict_tool_schema);
    assert_eq!(info.path_override.as_deref(), Some("messages"));
    assert_eq!(info.request_url, "https://api.example.com/v1/messages");
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
fn empty_anthropic_url_is_rejected_before_dispatch() {
    let model = test_model("custom", "anthropic", "   ", "claude-sonnet", "Claude Sonnet");
    assert!(compute_request_info(&model, &chat_messages()).is_err());
}

#[test]
fn responses_adapter_selects_native_endpoint_without_url_suffix() {
    let model =
        test_model("custom", "openai-responses", "https://api.example.com/v1", "gpt-5.4", "alias");
    let info = compute_request_info(&model, &chat_messages()).unwrap();
    assert_eq!(info.base_url, "https://api.example.com/v1");
    assert_eq!(info.request_url, "https://api.example.com/v1/responses");
}

#[test]
fn model_protocol_012_ordinary_request_uses_exact_catalog_endpoint() {
    let mut model =
        test_model("fixture", "openai-compatible", "https://example.test/v1", "wire", "fixture");
    model.api.protocols = Some(serde_json::from_value(json!([{
        "protocol":"chat", "url":"https://example.test/deployment/custom/chat/completions",
        "deployment":"fixture", "authentication":{"kind":"bearer"},
        "models":{"kind":"models","ids":["wire"]},
        "evidence":[{"kind":"configuration","source":"fixture","reference":"explicit","checked_at":"2026-11-11"}]
    }])).unwrap());
    for adapter in ["openai-compatible", "vendor-sdk"] {
        model.api.adapter = adapter.into();
        let info = compute_request_info(&model, &chat_messages()).unwrap();
        assert_eq!(info.request_url, "https://example.test/deployment/custom/chat/completions");
    }
}

#[test]
fn model_protocol_012_ordinary_request_refuses_empty_and_foreign_endpoints() {
    let mut model =
        test_model("fixture", "openai-compatible", "https://example.test/v1", "wire", "fixture");
    model.api.protocols = Some(vec![]);
    assert!(compute_request_info(&model, &chat_messages()).is_err());
    model.api.protocols = Some(serde_json::from_value(json!([{
        "protocol":"chat", "url":"https://foreign.test/v1/chat/completions",
        "deployment":"fixture", "authentication":{"kind":"bearer"},
        "models":{"kind":"all"},
        "evidence":[{"kind":"configuration","source":"fixture","reference":"explicit","checked_at":"2026-11-11"}]
    }])).unwrap());
    assert!(compute_request_info(&model, &chat_messages()).is_err());
}

#[test]
fn model_protocol_012_ordinary_request_rejects_unsupported_image() {
    let model =
        test_model("fixture", "openai-compatible", "https://example.test/v1", "wire", "fixture");
    let messages = json!([{"role":"user","content":[{"type":"image_url","image_url":{"url":"https://example.test/image.png"}}]}]);
    assert!(compute_request_info(&model, &messages).is_err());
}

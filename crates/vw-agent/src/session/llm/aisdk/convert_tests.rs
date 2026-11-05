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
fn empty_anthropic_url_keeps_empty_base_for_client_validation() {
    let model = test_model("custom", "anthropic", "   ", "claude-sonnet", "Claude Sonnet");

    let info = compute_request_info(&model, &chat_messages()).expect("request info");

    assert_eq!(info.base_url, "");
    assert_eq!(info.request_url, "/messages");
    assert_eq!(info.path_override.as_deref(), Some("messages"));
    assert!(!info.enforce_strict_tool_schema);
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
fn native_tool_search_request_request_uses_native_search_type() {
    use crate::tools::ToolSpec;
    let tools = std::collections::HashMap::from([(
        "tool_search".into(),
        ToolSpec::new(
            "tool_search",
            "Search tools",
            json!({"type":"object","properties":{"query":{"type":"string"}}}),
        ),
    )]);
    let declarations = super::native_request_tools(&tools, false).unwrap();
    assert!(matches!(
        &declarations[0],
        aisdk::core::NativeToolSpec::ToolSearch {
            execution: aisdk::core::ToolSearchExecution::Client,
            ..
        }
    ));
}

#[test]
fn native_tool_search_request_namespaces_filter_hidden_and_preserve_distinct_names() {
    use crate::tools::ToolSpec;
    use vw_api_types::tools::ToolExposure;
    let mut first = ToolSpec::new("stable_a", "A", json!({"type":"object"}));
    first.identity.name = "read".into();
    first.identity.namespace = Some("alpha".into());
    let mut second = first.clone();
    second.id = "stable_b".into();
    second.identity.namespace = Some("beta".into());
    let mut hidden = ToolSpec::new("secret", "Secret", json!({"type":"object"}));
    hidden.exposure = ToolExposure::Hidden;
    let tools = std::collections::HashMap::from([
        ("a".into(), first),
        ("b".into(), second),
        ("hidden".into(), hidden),
    ]);
    let wire = serde_json::to_value(super::native_request_tools(&tools, false).unwrap()).unwrap();
    assert_eq!(wire.as_array().unwrap().len(), 2);
    assert_eq!(wire[0]["name"], "alpha");
    assert_eq!(wire[1]["name"], "beta");
    assert_eq!(wire[0]["tools"][0]["name"], "read");
    assert_eq!(wire[1]["tools"][0]["name"], "read");
    assert!(!wire.to_string().contains("secret"));
    assert_eq!(super::native_tool_id(&tools, Some("alpha"), "read").unwrap(), "stable_a");
    assert_eq!(super::native_tool_id(&tools, Some("beta"), "read").unwrap(), "stable_b");
    assert!(super::native_tool_id(&tools, None, "read").is_err());
    assert!(super::native_tool_id(&tools, None, "secret").is_err());
}

#[test]
fn native_tool_search_request_schema_and_identity_errors_are_explicit() {
    use crate::tools::ToolSpec;
    let mut spec = ToolSpec::new("bad", "Bad", json!(42));
    let mut tools = std::collections::HashMap::from([("a".into(), spec.clone())]);
    assert!(super::native_request_tools(&tools, false).unwrap_err().to_string().contains("schema"));
    spec.input_schema = json!({"type":"object"});
    tools.insert("a".into(), spec.clone());
    tools.insert("b".into(), spec);
    assert!(
        super::native_request_tools(&tools, false).unwrap_err().to_string().contains("duplicate")
    );
}

#[test]
fn native_tool_search_request_capability_requires_responses_and_explicit_metadata() {
    for (adapter, endpoint, enabled, expected) in [
        ("openai-responses", "responses", true, true),
        ("openai-compatible", "chat/completions", true, false),
        ("openai-responses", "responses", false, false),
        ("anthropic", "messages", true, false),
    ] {
        let mut model = test_model(
            "fixture",
            adapter,
            &format!("http://localhost/v1/{endpoint}"),
            "gpt-fixture",
            "GPT fixture",
        );
        model.capabilities.native_tool_search = enabled;
        let info = compute_request_info(&model, &json!([])).unwrap();
        let capabilities = provider::resolve_native_search_capabilities(
            &model.capabilities,
            adapter,
            info.path_override.as_deref().unwrap_or("chat/completions"),
        );
        assert_eq!(capabilities.native_tool_search, expected, "{adapter}/{endpoint}/{enabled}");
    }
}

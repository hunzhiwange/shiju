//! 验证供应商选项重映射、消息规范化与输出令牌上限。

use super::*;
use serde_json::json;

#[test]
fn remap_provider_options_moves_matching_key() {
    let remapped = remap_provider_options(
        &json!({"openai": {"temperature": 0.2}, "keep": true}),
        "openai",
        "azure",
    );
    assert_eq!(remapped["azure"]["temperature"], json!(0.2));
    assert!(remapped.get("openai").is_none());
}

#[test]
fn remap_provider_options_preserves_non_objects_and_missing_keys() {
    let non_object = json!("raw");
    assert_eq!(remap_provider_options(&non_object, "openai", "anthropic"), non_object);

    let missing_key = json!({"anthropic": {"temperature": 0.2}});
    assert_eq!(remap_provider_options(&missing_key, "openai", "anthropic"), missing_key);
}

#[test]
fn provider_options_normalizes_adapter_aliases() {
    let options =
        provider_options("openai", "openai-compatible", json!({"openai": {"maxTokens": 12}}));
    assert_eq!(options["openai"]["maxTokens"], json!(12));
}

#[test]
fn provider_options_wraps_options_with_adapter_key_or_provider_id() {
    let wrapped = provider_options("custom", "github-copilot", json!({"maxTokens": 12}));
    assert_eq!(wrapped, json!({"copilot": {"maxTokens": 12}}));

    let fallback = provider_options("custom", "unknown", json!({"maxTokens": 12}));
    assert_eq!(fallback, json!({"custom": {"maxTokens": 12}}));
}

#[test]
fn provider_options_maps_all_supported_adapter_keys() {
    let cases = [
        (" github-copilot ", "copilot"),
        ("openai", "openai"),
        ("openai-compatible", "openai"),
        ("azure", "openai"),
        ("acp", "openai"),
        ("agent-client-protocol", "openai"),
        ("agent_client_protocol", "openai"),
        ("amazon-bedrock", "bedrock"),
        ("bedrock", "bedrock"),
        ("anthropic", "anthropic"),
        ("google-vertex/anthropic", "anthropic"),
        ("google-vertex", "google"),
        ("google", "google"),
        ("gateway", "gateway"),
        ("openrouter", "openrouter"),
    ];

    for (adapter, expected_key) in cases {
        let wrapped = provider_options("fallback", adapter, json!("value"));
        let object = wrapped.as_object().unwrap();
        assert_eq!(object.len(), 1);
        assert_eq!(object[expected_key], json!("value"));
    }
}

#[test]
fn normalize_messages_filters_anthropic_empty_content_parts() {
    let messages = vec![
        json!({"role": "user", "content": ""}),
        json!({"role": "user", "content": "kept"}),
        json!({"role": "user", "content": {"type": "text", "text": ""}}),
        json!({"role": "user"}),
        json!({"role": "assistant", "content": [
            {"type": "text", "text": ""},
            {"type": "reasoning", "text": ""},
            {"type": "text", "text": "kept"},
            {"type": "image", "data": "x"}
        ]}),
        json!({"role": "user", "content": [{"type": "text", "text": ""}]}),
    ];

    let normalized = normalize_messages(messages, " google-vertex/anthropic ", "claude-3");

    assert_eq!(normalized.len(), 4);
    assert_eq!(normalized[0]["content"], "kept");
    assert_eq!(normalized[1]["content"]["type"], "text");
    assert_eq!(normalized[2]["role"], "user");
    let content = normalized[3]["content"].as_array().unwrap();
    assert_eq!(content.len(), 2);
    assert_eq!(content[0]["text"], "kept");
}

#[test]
fn normalize_messages_filters_anthropic_text_parts_without_nonempty_text() {
    let messages = vec![
        json!({"role": "assistant", "content": [
            {"type": "text"},
            {"type": "reasoning", "text": null}
        ]}),
        json!({"role": "assistant", "content": [
            {"type": "text", "text": 7},
            {"type": "reasoning", "text": "kept"},
            {"type": "custom", "text": ""},
            null
        ]}),
        json!(7),
    ];

    let normalized = normalize_messages(messages, "anthropic", "other-model");

    assert_eq!(normalized.len(), 2);
    assert_eq!(normalized[0]["content"].as_array().unwrap().len(), 3);
    assert_eq!(normalized[0]["content"][0]["text"], "kept");
    assert_eq!(normalized[0]["content"][1]["type"], "custom");
    assert_eq!(normalized[0]["content"][2], json!(null));
    assert_eq!(normalized[1], json!(7));
}

#[test]
fn normalize_messages_leaves_non_anthropic_non_claude_messages_unchanged() {
    let messages = vec![json!({"role": "user", "content": ""})];
    assert_eq!(normalize_messages(messages.clone(), "openai", "gpt-4"), messages);
}

#[test]
fn normalize_messages_sanitizes_claude_tool_call_ids() {
    let messages = vec![
        json!({"role": "assistant", "content": [
            {"type": "tool-call", "toolCallId": "call.1/needs space"},
            {"type": "tool-call", "toolCallId": 42},
            {"type": "tool-result", "toolCallId": "ok-id_2"}
        ]}),
        json!({"role": "user", "content": [
            {"type": "tool-call", "toolCallId": "unchanged / user"}
        ]}),
    ];

    let normalized = normalize_messages(messages, "openai", "claude-sonnet");

    assert_eq!(normalized[0]["content"][0]["toolCallId"], "call_1_needs_space");
    assert_eq!(normalized[0]["content"][1]["toolCallId"], json!(42));
    assert_eq!(normalized[0]["content"][2]["toolCallId"], "ok-id_2");
    assert_eq!(normalized[1]["content"][0]["toolCallId"], "unchanged / user");
}

#[test]
fn normalize_messages_sanitizes_claude_tool_role_and_preserves_other_shapes() {
    let messages = vec![
        json!({"role": "tool", "content": [
            {"type": "tool-result", "toolCallId": "call:\u{4e00}"},
            {"type": "tool-call"},
            {"type": "text", "toolCallId": "text/id"},
            null
        ]}),
        json!({"role": "assistant", "content": "plain"}),
        json!({"content": [{"type": "tool-call", "toolCallId": "missing/role"}]}),
        json!(null),
    ];

    let normalized = normalize_messages(messages, "openai", "claude-3-haiku");

    assert_eq!(normalized[0]["content"][0]["toolCallId"], "call__");
    assert!(normalized[0]["content"][1].get("toolCallId").is_none());
    assert_eq!(normalized[0]["content"][2]["toolCallId"], "text/id");
    assert_eq!(normalized[0]["content"][3], json!(null));
    assert_eq!(normalized[1]["content"], "plain");
    assert_eq!(normalized[2]["content"][0]["toolCallId"], "missing/role");
    assert_eq!(normalized[3], json!(null));
}

#[test]
fn apply_provider_options_key_remap_updates_message_and_parts() {
    let messages = vec![
        json!({
            "role": "user",
            "content": [
                {"type": "text", "text": "hi", "providerOptions": {"openai": {"foo": true}}},
                "plain"
            ],
            "providerOptions": {"openai": {"temperature": 0.1}, "keep": 1}
        }),
        json!({"role": "user", "content": "plain"}),
        json!(null),
    ];

    let remapped = apply_provider_options_key_remap(messages, "openai", "anthropic");

    assert_eq!(remapped[0]["providerOptions"]["anthropic"]["temperature"], json!(0.1));
    assert_eq!(remapped[0]["providerOptions"]["keep"], json!(1));
    assert_eq!(remapped[0]["content"][0]["providerOptions"]["anthropic"]["foo"], json!(true));
    assert_eq!(remapped[0]["content"][1], "plain");
    assert_eq!(remapped[1]["content"], "plain");
    assert_eq!(remapped[2], json!(null));
}

#[test]
fn provider_option_remap_preserves_non_object_options_and_replaces_destination() {
    let messages = vec![json!({
        "providerOptions": "raw",
        "content": [
            {"providerOptions": ["raw"]},
            {"providerOptions": {
                "openai": {"source": true},
                "anthropic": {"stale": true}
            }},
            {"type": "text", "text": "unchanged"}
        ]
    })];

    let remapped = apply_provider_options_key_remap(messages, "openai", "anthropic");

    assert_eq!(remapped[0]["providerOptions"], "raw");
    assert_eq!(remapped[0]["content"][0]["providerOptions"], json!(["raw"]));
    assert_eq!(
        remapped[0]["content"][1]["providerOptions"],
        json!({"anthropic": {"source": true}})
    );
    assert_eq!(remapped[0]["content"][2]["text"], "unchanged");
}

#[test]
fn provider_option_remap_skips_unknown_same_and_azure_adapters() {
    let messages = vec![json!({"providerOptions": {"openai": {"x": 1}}})];
    assert_eq!(apply_provider_options_key_remap(messages.clone(), "openai", "unknown"), messages);
    assert_eq!(
        apply_provider_options_key_remap(messages.clone(), "openai", "openai-compatible"),
        messages
    );
    assert_eq!(apply_provider_options_key_remap(messages.clone(), "openai", "azure"), messages);
}

#[test]
fn max_output_tokens_respects_global_cap_and_zero_model_limit() {
    let global_max = output_token_max();

    assert_eq!(max_output_tokens(0), global_max);
    assert_eq!(max_output_tokens(global_max.saturating_add(1)), global_max);
    if global_max > 1 {
        assert_eq!(max_output_tokens(global_max - 1), global_max - 1);
    }
}

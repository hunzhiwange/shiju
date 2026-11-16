//! 聊天消息规范化与工具结果序列的库内契约。

use super::{build_chat_messages, ensure_openai_tool_call_sequence};
use model_types as provider;
use serde_json::{Value, json};

fn test_model(provider_id: &str, api_id: &str, adapter: &str) -> provider::Model {
    serde_json::from_value(json!({
        "id": api_id,
        "providerID": provider_id,
        "api": {
            "id": api_id,
            "url": "http://localhost",
            "adapter": adapter
        },
        "name": api_id,
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

#[test]
fn build_chat_messages_prepends_system_messages_and_preserves_user_messages() {
    let model = test_model("test-provider", "custom-model", "openai-compatible");
    let messages = vec![json!({ "role": "user", "content": "hello" })];

    let out = build_chat_messages(&["system one".to_string()], &messages, &model);
    let arr = out.as_array().expect("chat messages should be an array");

    assert_eq!(arr[0], json!({ "role": "system", "content": "system one" }));
    assert_eq!(arr[1], json!({ "role": "user", "content": "hello" }));
}

#[test]
// 缺失工具结果会破坏 OpenAI 调用序列，因此发送前补齐占位结果。
fn build_chat_messages_fills_missing_openai_tool_results() {
    let model = test_model("test-provider", "custom-model", "openai-compatible");
    let messages = vec![
        json!({
            "role": "assistant",
            "tool_calls": [
                { "id": "call-1", "type": "function", "function": { "name": "A", "arguments": "{}" } },
                { "id": "call-2", "type": "function", "function": { "name": "B", "arguments": "{}" } }
            ]
        }),
        json!({ "role": "tool", "tool_call_id": "call-1", "content": "done" }),
        json!({ "role": "user", "content": "next" }),
    ];

    let out = build_chat_messages(&[], &messages, &model);
    let arr = out.as_array().expect("chat messages should be an array");

    assert_eq!(arr[0]["content"], json!(" "));
    assert_eq!(arr[1]["tool_call_id"], json!("call-1"));
    assert_eq!(arr[2]["role"], json!("tool"));
    assert_eq!(arr[2]["tool_call_id"], json!("call-2"));
    assert!(arr[2]["content"].as_str().unwrap().contains("missing_tool_result: true"));
    assert_eq!(arr[3], json!({ "role": "user", "content": "next" }));
}

#[test]
fn build_chat_messages_does_not_duplicate_existing_tool_results() {
    let model = test_model("test-provider", "custom-model", "openai-compatible");
    let messages = vec![
        json!({
            "role": "assistant",
            "content": "",
            "tool_calls": [
                { "id": "call-1", "type": "function", "function": { "name": "A", "arguments": "{}" } }
            ]
        }),
        json!({ "role": "tool", "tool_call_id": "call-1", "content": "done" }),
    ];

    let out = build_chat_messages(&[], &messages, &model);
    let arr = out.as_array().expect("chat messages should be an array");

    assert_eq!(arr.len(), 2);
    assert_eq!(arr[1]["content"], json!("done"));
}

#[test]
fn build_chat_messages_removes_reasoning_content_except_for_deepseek() {
    let regular = test_model("test-provider", "custom-model", "openai-compatible");
    let deepseek = test_model("deepseek", "custom-model", "openai-compatible");
    let messages = vec![json!({
        "role": "assistant",
        "content": "answer",
        "reasoning_content": "private reasoning"
    })];

    let regular_out = build_chat_messages(&[], &messages, &regular);
    let deepseek_out = build_chat_messages(&[], &messages, &deepseek);

    assert!(regular_out.as_array().unwrap()[0].get("reasoning_content").is_none());
    assert_eq!(
        deepseek_out.as_array().unwrap()[0]["reasoning_content"],
        json!("private reasoning")
    );
}

#[test]
fn build_chat_messages_preserves_non_object_messages_while_filtering_reasoning() {
    let model = test_model("test-provider", "custom-model", "openai-compatible");
    let messages = vec![
        json!("opaque message"),
        json!({
            "role": "assistant",
            "content": "answer",
            "reasoning_content": "private reasoning"
        }),
    ];

    let out = build_chat_messages(&[], &messages, &model);
    let messages = out.as_array().expect("chat messages should be an array");

    assert_eq!(messages[0], json!("opaque message"));
    assert!(messages[1].get("reasoning_content").is_none());
}

#[test]
fn build_chat_messages_keeps_declared_reasoning_field_for_routed_model() {
    let mut model = test_model("shiju", "deepseek-v4.1-flash", "openai-compatible");
    model.capabilities.interleaved =
        provider::InterleavedCapability::Field { field: "reasoning_content".into() };
    let out = build_chat_messages(
        &[],
        &[json!({
            "role": "assistant", "content": "", "reasoning_content": "reasoning",
            "tool_calls": [{"id":"call-1","type":"function","function":{"name":"bash","arguments":"{}"}}]
        })],
        &model,
    );
    assert_eq!(out[0]["reasoning_content"], "reasoning");
}

#[test]
// 仅 DeepSeek 推理模型保留 reasoning_content，其他模型发送前移除。
fn build_chat_messages_keeps_reasoning_content_for_deepseek_reasoner_model_id() {
    let model = test_model("test-provider", "deepseek-reasoner-v3", "openai-compatible");
    let messages = vec![json!({
        "role": "assistant",
        "content": "answer",
        "reasoning_content": "reasoning"
    })];

    let out = build_chat_messages(&[], &messages, &model);

    assert_eq!(out.as_array().unwrap()[0]["reasoning_content"], json!("reasoning"));
}

#[test]
fn build_chat_messages_applies_provider_options_key_remap() {
    let model = test_model("openai", "claude-test", "anthropic");
    let messages = vec![json!({
        "role": "user",
        "content": [
            {
                "type": "text",
                "text": "hello",
                "providerOptions": {
                    "openai": {
                        "cacheControl": { "type": "ephemeral" }
                    }
                }
            }
        ],
        "providerOptions": {
            "openai": {
                "foo": true
            }
        }
    })];

    let out = build_chat_messages(&[], &messages, &model);
    let msg = &out.as_array().unwrap()[0];

    assert_eq!(msg["providerOptions"]["anthropic"]["foo"], json!(true));
    assert!(msg["providerOptions"].get("openai").is_none());
    assert_eq!(
        msg["content"][0]["providerOptions"]["anthropic"]["cacheControl"]["type"],
        json!("ephemeral")
    );
}

#[test]
fn ensure_openai_tool_call_sequence_ignores_messages_without_valid_call_ids() {
    let mut messages = vec![
        json!("not an object"),
        json!({ "role": "user", "content": "hello" }),
        json!({
            "role": "assistant",
            "tool_calls": [
                { "type": "function" },
                { "id": "" },
                { "id": "  " }
            ]
        }),
    ];
    let original = messages.clone();

    ensure_openai_tool_call_sequence(&mut messages);

    assert_eq!(messages, original);
}

#[test]
fn ensure_openai_tool_call_sequence_matches_trimmed_assistant_call_ids() {
    let mut messages = vec![
        json!({
            "role": "assistant",
            "tool_calls": [{ "id": "  call-1  " }]
        }),
        json!({ "role": "tool", "tool_call_id": "call-1", "content": "done" }),
    ];

    ensure_openai_tool_call_sequence(&mut messages);

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["content"], json!(" "));
    assert_eq!(messages[1]["content"], json!("done"));
}

#[test]
fn ensure_openai_tool_call_sequence_inserts_after_unrelated_contiguous_tool_results() {
    let mut messages = vec![
        json!({
            "role": "assistant",
            "content": "working",
            "tool_calls": [{ "id": "call-1" }]
        }),
        json!({ "role": "tool", "tool_call_id": "other-call", "content": "other" }),
        json!({ "role": "user", "content": "next" }),
    ];

    ensure_openai_tool_call_sequence(&mut messages);

    assert_eq!(messages.len(), 4);
    assert_eq!(messages[1]["tool_call_id"], json!("other-call"));
    assert_eq!(messages[2]["tool_call_id"], json!("call-1"));
    assert!(messages[2]["content"].as_str().unwrap().contains("missing_tool_result: true"));
    assert_eq!(messages[3], json!({ "role": "user", "content": "next" }));
}

#[test]
fn ensure_openai_tool_call_sequence_fills_each_assistant_sequence() {
    let mut messages = vec![
        json!({
            "role": "assistant",
            "tool_calls": [{ "id": "call-1" }]
        }),
        json!({ "role": "user", "content": "continue" }),
        json!({
            "role": "assistant",
            "tool_calls": [{ "id": "call-2" }]
        }),
    ];

    ensure_openai_tool_call_sequence(&mut messages);

    let tool_call_ids = messages
        .iter()
        .filter(|message| message.get("role").and_then(Value::as_str) == Some("tool"))
        .map(|message| message["tool_call_id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(tool_call_ids, vec!["call-1", "call-2"]);
    assert_eq!(messages[0]["content"], json!(" "));
    assert_eq!(messages[3]["content"], json!(" "));
}


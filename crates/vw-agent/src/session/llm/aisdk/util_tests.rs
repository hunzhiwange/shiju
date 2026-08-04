use aisdk::core::language_model::{LanguageModelResponseContentType, Usage};
use aisdk::core::{Message, UserContentPart};
use serde_json::{Value, json};

use crate::app::agent::provider::provider;

use super::{
    StreamingToolCallState, assistant_text_with_reasoning, merge_tool_call_delta,
    normalize_strict_object_required, openai_message_content_to_text,
    openai_messages_to_aisdk_messages, openai_user_content_to_aisdk_message, should_abort,
    stop_sequences_from_value, take_next_sse_event, token_usage_from_aisdk_usage,
    token_usage_from_openai_usage,
};

fn test_model(provider_id: &str, model_id: &str, name: &str) -> provider::Model {
    serde_json::from_value(json!({
        "id": model_id,
        "providerID": provider_id,
        "api": {
            "id": model_id,
            "url": "https://api.example.com/v1",
            "adapter": "openai-compatible"
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

fn user_text(message: &Message) -> Option<String> {
    match message {
        Message::User(user) => Some(
            user.content
                .iter()
                .filter_map(|part| match part {
                    UserContentPart::Text(text) => Some(text.as_str()),
                    UserContentPart::Image(_) => None,
                })
                .collect::<Vec<_>>()
                .join(""),
        ),
        _ => None,
    }
}

fn user_image_urls(message: &Message) -> Vec<String> {
    match message {
        Message::User(user) => user
            .content
            .iter()
            .filter_map(|part| match part {
                UserContentPart::Image(image) => Some(image.image_url.clone()),
                UserContentPart::Text(_) => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

#[test]
fn normalize_strict_object_required_recurses_through_schema_shapes() {
    let mut schema = json!({
        "type": "object",
        "properties": {
            "z": { "type": "string" },
            "a": {
                "type": "object",
                "properties": {
                    "nested": { "type": "number" }
                }
            },
            "list": {
                "type": "array",
                "items": {
                    "properties": {
                        "item": { "type": "boolean" }
                    }
                }
            }
        },
        "anyOf": [
            { "type": "object", "properties": { "kind": { "type": "string" } } }
        ],
        "allOf": [
            { "properties": { "all": { "type": "string" } } }
        ],
        "oneOf": [
            { "type": "object" }
        ]
    });

    normalize_strict_object_required(&mut schema);

    assert_eq!(schema["additionalProperties"], json!(false));
    assert_eq!(schema["required"], json!(["a", "list", "z"]));
    assert_eq!(schema["properties"]["a"]["additionalProperties"], json!(false));
    assert_eq!(schema["properties"]["a"]["required"], json!(["nested"]));
    assert_eq!(schema["properties"]["list"]["items"]["additionalProperties"], json!(false));
    assert_eq!(schema["properties"]["list"]["items"]["required"], json!(["item"]));
    assert_eq!(schema["anyOf"][0]["additionalProperties"], json!(false));
    assert_eq!(schema["allOf"][0]["required"], json!(["all"]));
    assert_eq!(schema["oneOf"][0]["additionalProperties"], json!(false));
}

#[test]
fn normalize_strict_object_required_ignores_non_objects() {
    let mut schema = json!(["not", "an", "object"]);

    normalize_strict_object_required(&mut schema);

    assert_eq!(schema, json!(["not", "an", "object"]));
}

#[test]
fn openai_message_content_to_text_accepts_strings_and_text_parts_only() {
    assert_eq!(openai_message_content_to_text(Some(&json!("plain text"))), "plain text");
    assert_eq!(
        openai_message_content_to_text(Some(&json!([
            { "type": "text", "text": "hello" },
            { "type": "image_url", "image_url": { "url": "https://example.com/a.png" } },
            { "type": "text", "text": " world" },
            { "type": "text", "text": 7 },
            { "text": "missing type" }
        ]))),
        "hello world"
    );
    assert_eq!(openai_message_content_to_text(Some(&json!({ "text": "nope" }))), "");
    assert_eq!(openai_message_content_to_text(None), "");
}

#[test]
fn openai_user_content_to_aisdk_message_keeps_image_markers_and_image_url_parts() {
    let from_markers = openai_user_content_to_aisdk_message(Some(&json!(
        "look\n\n[IMAGE:data:image/png;base64,abc]\n[IMAGE:https://example.com/a.png]"
    )))
    .expect("markers should convert");
    assert_eq!(
        from_markers
            .content
            .iter()
            .filter_map(|part| match part {
                UserContentPart::Text(text) => Some(text.as_str()),
                UserContentPart::Image(_) => None,
            })
            .collect::<Vec<_>>(),
        vec!["look"]
    );
    assert_eq!(
        from_markers
            .content
            .iter()
            .filter_map(|part| match part {
                UserContentPart::Image(image) => Some(image.image_url.as_str()),
                UserContentPart::Text(_) => None,
            })
            .collect::<Vec<_>>(),
        vec!["data:image/png;base64,abc", "https://example.com/a.png"]
    );

    let from_parts = openai_user_content_to_aisdk_message(Some(&json!([
        { "type": "text", "text": "hello" },
        { "type": "image_url", "image_url": { "url": "https://example.com/a.png" } },
        { "type": "image", "url": "data:image/jpeg;base64,xyz" }
    ])))
    .expect("parts should convert");
    assert_eq!(user_text(&Message::User(from_parts.clone())).as_deref(), Some("hello"));
    assert_eq!(
        user_image_urls(&Message::User(from_parts)),
        vec!["https://example.com/a.png".to_string(), "data:image/jpeg;base64,xyz".to_string()]
    );
}

#[test]
fn openai_messages_to_aisdk_messages_preserves_user_images() {
    let model = test_model("openai", "gpt-test", "GPT Test");
    let messages = json!([
        {
            "role": "user",
            "content": [
                { "type": "text", "text": "describe" },
                { "type": "image_url", "image_url": { "url": "https://example.com/a.png" } }
            ]
        },
        {
            "role": "user",
            "content": "caption\n\n[IMAGE:data:image/png;base64,abc]"
        }
    ]);

    let converted = openai_messages_to_aisdk_messages(&messages, &model).expect("convert");
    assert_eq!(converted.len(), 2);
    assert_eq!(user_text(&converted[0]).as_deref(), Some("describe"));
    assert_eq!(user_image_urls(&converted[0]), vec!["https://example.com/a.png".to_string()]);
    assert_eq!(user_text(&converted[1]).as_deref(), Some("caption"));
    assert_eq!(user_image_urls(&converted[1]), vec!["data:image/png;base64,abc".to_string()]);
}

#[test]
fn assistant_text_with_reasoning_combines_content_and_reasoning_labels() {
    let empty = serde_json::Map::new();
    assert_eq!(assistant_text_with_reasoning(&empty), "");

    let mut text_only = serde_json::Map::new();
    text_only.insert("content".to_string(), json!("answer"));
    assert_eq!(assistant_text_with_reasoning(&text_only), "answer");

    let mut reasoning_only = serde_json::Map::new();
    reasoning_only.insert("reasoning_content".to_string(), json!(" thinking "));
    assert_eq!(assistant_text_with_reasoning(&reasoning_only), "[reasoning]\nthinking");

    let mut both = serde_json::Map::new();
    both.insert("content".to_string(), json!("answer"));
    both.insert("reasoning_content".to_string(), json!("thinking"));
    assert_eq!(assistant_text_with_reasoning(&both), "answer\n\n[reasoning]\nthinking");
}

#[test]
fn openai_messages_to_aisdk_messages_converts_roles_and_tool_chain() {
    let model = test_model("openai", "gpt-test", "GPT Test");
    let messages = json!([
        { "role": "system", "content": "system text" },
        {
            "role": "user",
            "content": [
                { "type": "text", "text": "hello " },
                { "type": "text", "text": "there" }
            ]
        },
        {
            "role": "assistant",
            "content": "assistant text",
            "tool_calls": [
                {
                    "id": "call-1",
                    "type": "function",
                    "function": {
                        "name": "lookup",
                        "arguments": "{\"q\":\"rust\"}"
                    }
                },
                {
                    "id": "",
                    "type": "function",
                    "function": {
                        "name": "",
                        "arguments": "not-json"
                    }
                }
            ]
        },
        { "role": "tool", "tool_call_id": "call-1", "content": "{\"ok\":true}" },
        { "role": "developer", "content": "developer text" },
        { "role": "unknown", "content": "skip me" },
        { "role": "user", "content": "   " },
        null
    ]);

    let converted = openai_messages_to_aisdk_messages(&messages, &model).expect("convert");

    assert_eq!(converted.len(), 6);
    assert!(matches!(&converted[0], Message::System(system) if system.content == "system text"));
    assert_eq!(user_text(&converted[1]).as_deref(), Some("hello there"));
    assert!(matches!(
        &converted[2],
        Message::Assistant(assistant)
            if matches!(&assistant.content, LanguageModelResponseContentType::Text(text) if text == "assistant text")
    ));
    assert!(matches!(
        &converted[3],
        Message::Assistant(assistant)
            if matches!(
                &assistant.content,
                LanguageModelResponseContentType::ToolCall(info)
                    if info.tool.id == "call-1"
                        && info.tool.name == "lookup"
                        && info.input == json!({ "q": "rust" })
            )
    ));
    assert!(matches!(
        &converted[4],
        Message::Tool(result)
            if result.tool.id == "call-1"
                && result.tool.name == "lookup"
                && result.output.as_ref().ok() == Some(&json!({ "ok": true }))
    ));
    assert!(matches!(&converted[5], Message::Developer(text) if text == "developer text"));
}

#[test]
fn openai_messages_to_aisdk_messages_interleaves_tool_results_for_strict_providers() {
    let model = test_model("moonshotai-cn", "kimi-k3", "Kimi K3");
    let messages = json!([
        {
            "role": "assistant",
            "content": "先检查再修改",
            "tool_calls": [
                {
                    "id": "bash:0",
                    "type": "function",
                    "function": {
                        "name": "bash",
                        "arguments": "{\"command\":\"pwd\"}"
                    }
                },
                {
                    "id": "write:1",
                    "type": "function",
                    "function": {
                        "name": "write",
                        "arguments": "{\"path\":\"tasks.json\"}"
                    }
                }
            ]
        },
        { "role": "tool", "tool_call_id": "bash:0", "content": "{\"ok\":true}" },
        { "role": "tool", "tool_call_id": "write:1", "content": "{\"ok\":true}" }
    ]);

    let converted = openai_messages_to_aisdk_messages(&messages, &model).expect("convert");

    assert_eq!(converted.len(), 5);
    assert!(matches!(
        &converted[0],
        Message::Assistant(assistant)
            if matches!(
                &assistant.content,
                LanguageModelResponseContentType::Text(text)
                    if text == "先检查再修改"
            )
    ));
    assert!(matches!(
        &converted[1],
        Message::Assistant(assistant)
            if matches!(
                &assistant.content,
                LanguageModelResponseContentType::ToolCall(info)
                    if info.tool.id == "bash:0" && info.tool.name == "bash"
            )
    ));
    assert!(matches!(
        &converted[2],
        Message::Tool(result)
            if result.tool.id == "bash:0" && result.tool.name == "bash"
    ));
    assert!(matches!(
        &converted[3],
        Message::Assistant(assistant)
            if matches!(
                &assistant.content,
                LanguageModelResponseContentType::ToolCall(info)
                    if info.tool.id == "write:1" && info.tool.name == "write"
            )
    ));
    assert!(matches!(
        &converted[4],
        Message::Tool(result)
            if result.tool.id == "write:1" && result.tool.name == "write"
    ));
}

#[test]
fn openai_messages_to_aisdk_messages_handles_non_array_and_deepseek_reasoner() {
    let normal_model = test_model("openai", "gpt-test", "GPT Test");
    assert!(
        openai_messages_to_aisdk_messages(&json!({ "role": "user" }), &normal_model)
            .expect("non array should be accepted")
            .is_empty()
    );

    let deepseek = test_model("deepseek", "deepseek-reasoner", "DeepSeek Reasoner");
    let messages = json!([
        {
            "role": "assistant",
            "content": "final",
            "reasoning_content": "chain",
            "tool_calls": [
                {
                    "id": "call-1",
                    "type": "function",
                    "function": {
                        "name": "lookup",
                        "arguments": "{\"q\":\"rust\"}"
                    }
                }
            ]
        },
        { "role": "tool", "tool_call_id": "call-1", "content": "result" },
        { "role": "tool", "content": "anonymous" }
    ]);

    let converted = openai_messages_to_aisdk_messages(&messages, &deepseek).expect("convert");

    assert_eq!(converted.len(), 3);
    assert!(matches!(
        &converted[0],
        Message::Assistant(assistant)
            if matches!(
                &assistant.content,
                LanguageModelResponseContentType::Text(text)
                    if text == "final\n\n[reasoning]\nchain"
            )
    ));
    assert_eq!(user_text(&converted[1]).as_deref(), Some("[tool_result:call-1]\nresult"));
    assert_eq!(user_text(&converted[2]).as_deref(), Some("[tool_result]\nanonymous"));
}

#[test]
fn stop_sequences_parse_strings_arrays_and_reject_empty_values() {
    assert_eq!(stop_sequences_from_value(&json!(" END ")), Some(vec!["END".to_string()]));
    assert_eq!(stop_sequences_from_value(&json!("   ")), None);
    assert_eq!(
        stop_sequences_from_value(&json!([" one ", "", 3, "two"])),
        Some(vec!["one".to_string(), "two".to_string()])
    );
    assert_eq!(stop_sequences_from_value(&json!([false, 3])), None);
    assert_eq!(stop_sequences_from_value(&json!({ "stop": true })), None);
}

#[test]
fn token_usage_from_openai_usage_supports_legacy_and_new_field_names() {
    let legacy = token_usage_from_openai_usage(&json!({
        "prompt_tokens": 10,
        "completion_tokens": 20,
        "prompt_tokens_details": { "cached_tokens": 3 },
        "completion_tokens_details": { "reasoning_tokens": 4 }
    }));

    assert_eq!(legacy.input_tokens, 10);
    assert_eq!(legacy.output_tokens, 20);
    assert_eq!(legacy.cached_tokens, 3);
    assert_eq!(legacy.reasoning_tokens, 4);

    let modern = token_usage_from_openai_usage(&json!({
        "input_tokens": 7,
        "output_tokens": 8,
        "input_tokens_details": { "cached_tokens": 2 },
        "reasoning_tokens": 5
    }));

    assert_eq!(modern.input_tokens, 7);
    assert_eq!(modern.output_tokens, 8);
    assert_eq!(modern.cached_tokens, 2);
    assert_eq!(modern.reasoning_tokens, 5);

    assert_eq!(token_usage_from_openai_usage(&json!({})).input_tokens, 0);
}

#[test]
fn token_usage_from_aisdk_usage_maps_some_values_and_defaults() {
    let usage = token_usage_from_aisdk_usage(&Usage {
        input_tokens: Some(1),
        output_tokens: Some(2),
        cached_tokens: Some(3),
        reasoning_tokens: Some(4),
    });

    assert_eq!(usage.input_tokens, 1);
    assert_eq!(usage.output_tokens, 2);
    assert_eq!(usage.cached_tokens, 3);
    assert_eq!(usage.reasoning_tokens, 4);

    let usage = token_usage_from_aisdk_usage(&Usage::default());
    assert_eq!(usage.input_tokens, 0);
    assert_eq!(usage.output_tokens, 0);
    assert_eq!(usage.cached_tokens, 0);
    assert_eq!(usage.reasoning_tokens, 0);
}

#[test]
fn take_next_sse_event_handles_lf_and_crlf_frames() {
    let mut buffer = "event: a\r\ndata: one\r\n\r\nevent: b\n\npartial".to_string();

    assert_eq!(take_next_sse_event(&mut buffer).as_deref(), Some("event: a\r\ndata: one"));
    assert_eq!(take_next_sse_event(&mut buffer).as_deref(), Some("event: b"));
    assert_eq!(take_next_sse_event(&mut buffer), None);
    assert_eq!(buffer, "partial");
}

#[test]
fn merge_tool_call_delta_resizes_merges_and_assigns_fallback_ids() {
    let mut states = Vec::<StreamingToolCallState>::new();

    merge_tool_call_delta(
        &mut states,
        1,
        &json!({
            "function": {
                "name": "lookup",
                "arguments": "{\"q\":"
            }
        }),
        5,
    );
    merge_tool_call_delta(
        &mut states,
        1,
        &json!({
            "id": "call-real",
            "function": {
                "arguments": "\"rust\"}"
            }
        }),
        5,
    );
    merge_tool_call_delta(
        &mut states,
        0,
        &json!({
            "id": "call-zero",
            "function": {
                "name": "noop",
                "arguments": ""
            }
        }),
        0,
    );

    assert_eq!(states.len(), 2);
    assert_eq!(states[1].id, "call-real");
    assert_eq!(states[1].name, "lookup");
    assert_eq!(states[1].arguments, "{\"q\":\"rust\"}");
    assert_eq!(states[0].id, "call-zero");
    assert_eq!(states[0].name, "noop");
    assert_eq!(states[0].arguments, "");
}

#[test]
fn should_abort_reflects_watch_receiver_state() {
    let (tx, rx) = tokio::sync::watch::channel(false);

    assert!(!should_abort(None));
    assert!(!should_abort(Some(&rx)));

    tx.send(true).expect("send abort flag");

    assert!(should_abort(Some(&rx)));
}

#[test]
fn normalize_strict_object_required_preserves_primitive_schema_objects() {
    let mut schema = json!({
        "type": "string",
        "additionalProperties": true,
        "required": ["keep"],
        "items": 7,
        "anyOf": { "type": "object" }
    });
    let original = schema.clone();

    normalize_strict_object_required(&mut schema);

    assert_eq!(schema, original);
}

#[test]
fn assistant_text_with_reasoning_treats_blank_content_and_non_string_reasoning_as_empty() {
    let mut blank_content = serde_json::Map::new();
    blank_content.insert("content".to_string(), json!(" \n "));
    blank_content.insert("reasoning_content".to_string(), json!(" reason "));
    assert_eq!(assistant_text_with_reasoning(&blank_content), "[reasoning]\nreason");

    let mut non_string_reasoning = serde_json::Map::new();
    non_string_reasoning.insert("content".to_string(), json!("answer"));
    non_string_reasoning.insert("reasoning_content".to_string(), json!(["ignored"]));
    assert_eq!(assistant_text_with_reasoning(&non_string_reasoning), "answer");
}

#[test]
fn openai_messages_to_aisdk_messages_skips_malformed_calls_and_preserves_tool_outputs() {
    let model = test_model("openai", "gpt-test", "GPT Test");
    let messages = json!([
        {
            "role": "assistant",
            "content": "   ",
            "tool_calls": [
                { "id": "ignored", "function": null },
                {
                    "id": "call-text",
                    "function": { "name": "lookup", "arguments": 5 }
                }
            ]
        },
        { "role": "tool", "tool_call_id": "call-text", "content": "not-json" },
        { "role": "tool", "tool_call_id": "unknown", "content": "null" }
    ]);

    let converted = openai_messages_to_aisdk_messages(&messages, &model).expect("convert");

    assert_eq!(converted.len(), 3);
    assert!(matches!(
        &converted[0],
        Message::Assistant(assistant)
            if matches!(
                &assistant.content,
                LanguageModelResponseContentType::ToolCall(info)
                    if info.tool.id == "call-text"
                        && info.tool.name == "lookup"
                        && info.input == json!({})
            )
    ));
    assert!(matches!(
        &converted[1],
        Message::Tool(result)
            if result.tool.id == "call-text"
                && result.tool.name == "lookup"
                && result.output.as_ref().ok() == Some(&json!("not-json"))
    ));
    assert!(matches!(
        &converted[2],
        Message::Tool(result)
            if result.tool.id == "unknown"
                && result.tool.name.is_empty()
                && result.output.as_ref().ok() == Some(&Value::Null)
    ));
}

#[test]
fn openai_messages_to_aisdk_messages_detects_deepseek_provider_case_insensitively() {
    let model = test_model("DeEpSeEk Cloud", "chat-model", "DeepSeek Chat");
    let messages = json!([
        {
            "role": "assistant",
            "reasoning_content": "analysis",
            "tool_calls": [{
                "id": "call-1",
                "function": { "name": "lookup", "arguments": "{}" }
            }]
        },
        { "role": "tool", "tool_call_id": "call-1", "content": "result" }
    ]);

    let converted = openai_messages_to_aisdk_messages(&messages, &model).expect("convert");

    assert_eq!(converted.len(), 2);
    assert!(matches!(
        &converted[0],
        Message::Assistant(assistant)
            if matches!(
                &assistant.content,
                LanguageModelResponseContentType::Text(text)
                    if text == "[reasoning]\nanalysis"
            )
    ));
    assert_eq!(user_text(&converted[1]).as_deref(), Some("[tool_result:call-1]\nresult"));
}

#[test]
fn merge_tool_call_delta_ignores_blank_updates_and_keeps_fallback_id_stable() {
    let mut states = Vec::<StreamingToolCallState>::new();
    merge_tool_call_delta(
        &mut states,
        0,
        &json!({ "function": { "name": "lookup", "arguments": "{" } }),
        2,
    );

    merge_tool_call_delta(
        &mut states,
        0,
        &json!({
            "id": "   ",
            "function": { "name": " ", "arguments": "" }
        }),
        9,
    );
    merge_tool_call_delta(&mut states, 0, &Value::Null, 20);

    assert_eq!(states[0].id, "call_3");
    assert_eq!(states[0].name, "lookup");
    assert_eq!(states[0].arguments, "{");
}

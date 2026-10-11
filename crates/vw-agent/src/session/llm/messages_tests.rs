//! 验证系统消息构造、工具结果补全与供应商消息规范化。

use super::super::types::{AgentInfo, StreamInput};
use super::*;

use crate::app::agent::permission::next;
use crate::app::agent::provider::provider;
use crate::app::agent::session::message;
use crate::app::agent::tools::ToolSpec;
use serde_json::{Value, json};
use std::collections::HashMap;

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

fn test_input(model: provider::Model) -> StreamInput {
    StreamInput {
        user: message::UserInfo {
            id: "user-1".to_string(),
            session_id: "session-1".to_string(),
            time: message::UserTime { created: 123 },
            summary: None,
            agent: "build".to_string(),
            model: message::ModelRef {
                provider_id: model.provider_id.clone(),
                model_id: model.id.clone(),
            },
            system: Some("user system".to_string()),
            tools: None,
            variant: None,
        },
        session_id: "session-1".to_string(),
        model,
        agent: AgentInfo {
            name: "build".to_string(),
            mode: "build".to_string(),
            prompt: Some("agent prompt".to_string()),
            temperature: None,
            top_p: None,
            options: HashMap::new(),
            permission: Default::default(),
        },
        system: vec![" ".to_string(), "session system".to_string()],
        abort: None,
        messages: Vec::new(),
        small: false,
        tools: HashMap::new(),
        retries: 0,
    }
}

#[test]
fn build_system_messages_merges_non_empty_sources_in_order() {
    let model = test_model("test-provider", "custom-model", "openai-compatible");
    let input = test_input(model);

    let system = build_system_messages(&input, false);

    assert_eq!(system.len(), 1);
    let joined = &system[0];
    let agent_idx = joined.find("agent prompt").expect("agent prompt should be present");
    let session_idx = joined.find("session system").expect("session system should be present");
    let user_idx = joined.find("user system").expect("user system should be present");
    assert!(agent_idx < session_idx);
    assert!(session_idx < user_idx);
    assert!(!joined.contains("\n \n"));
}

#[test]
fn build_system_messages_does_not_duplicate_unified_prompt_for_codex() {
    // Stable marker from unified.txt; must appear once regardless of is_codex.
    const UNIFIED_MARKER: &str = crate::app::agent::session::system::UNIFIED_PROMPT_MARKER;

    let model = test_model("openai", "gpt-5-test", "openai");
    let input = test_input(model);

    let without_codex = build_system_messages(&input, false).join("\n");
    let with_codex = build_system_messages(&input, true).join("\n");

    let without_count = without_codex.matches(UNIFIED_MARKER).count();
    let with_count = with_codex.matches(UNIFIED_MARKER).count();
    eprintln!("unified marker count without_codex={without_count} with_codex={with_count}");
    eprintln!(
        "joined lengths without_codex={} with_codex={}",
        without_codex.len(),
        with_codex.len()
    );

    assert_eq!(without_count, 1, "non-codex path should include unified prompt once");
    assert_eq!(with_count, 1, "codex path must not duplicate unified prompt");
    assert!(with_codex.contains("agent prompt"));
    assert!(with_codex.contains("session system"));
}

#[test]
fn build_system_messages_keeps_pre_rendered_runtime_prompt_once() {
    let model = test_model("deepseek", "deepseek-v4-flash", "openai-compatible");
    let mut input = test_input(model);
    let rendered = crate::app::agent::session::system::render_unified_prompt(
        "deepseek/deepseek-v4-flash",
        "/workspace/project",
        &["TodoRead".into(), "TodoWrite".into(), "AgentTool".into()],
    );
    input.system = vec![rendered, "session system".to_string()];

    let joined = build_system_messages(&input, false).join("\n");

    assert_eq!(
        joined.matches(crate::app::agent::session::system::UNIFIED_PROMPT_MARKER).count(),
        1
    );
    assert!(joined.contains("deepseek/deepseek-v4-flash"));
    assert!(joined.contains("/workspace/project"));
    assert!(joined.contains("## Task Tracking"));
    assert!(joined.contains("## Subagents"));
}

#[test]
// 空的可选上下文不生成占位消息，供应商系统提示仍保持原顺序。
fn build_system_messages_filters_blank_optional_sources_and_keeps_provider_prompt() {
    let model = test_model("test-provider", "custom-model", "openai-compatible");
    let provider_prompt = crate::app::agent::session::system::provider(&model).join("\n");
    let mut input = test_input(model);
    input.agent.prompt = Some(" \t".to_string());
    input.system = vec![String::new(), " \n".to_string()];
    input.user.system = Some("\t".to_string());

    let system = build_system_messages(&input, false);

    assert_eq!(system, vec![provider_prompt]);
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

#[test]
fn resolve_tools_removes_user_disabled_and_permission_denied_tools() {
    let tools = HashMap::from([
        ("Bash".to_string(), ToolSpec::new("Bash", "run a command", json!({ "type": "object" }))),
        ("Read".to_string(), ToolSpec::new("Read", "read a file", json!({ "type": "object" }))),
        ("Write".to_string(), ToolSpec::new("Write", "write a file", json!({ "type": "object" }))),
    ]);
    let permission = next::from_config(&json!({ "Bash": "deny" }));
    let user = message::UserInfo {
        id: "user-1".to_string(),
        session_id: "session-1".to_string(),
        time: message::UserTime { created: 123 },
        summary: None,
        agent: "build".to_string(),
        model: message::ModelRef {
            provider_id: "provider".to_string(),
            model_id: "model".to_string(),
        },
        system: None,
        tools: Some(HashMap::from([("Write".to_string(), false)])),
        variant: None,
    };

    let resolved = resolve_tools(&tools, &permission, &user);

    assert!(!resolved.contains_key("Bash"));
    assert!(!resolved.contains_key("Write"));
    assert!(resolved.contains_key("Read"));
}

#[test]
fn resolve_tools_keeps_allowed_and_explicitly_enabled_tools_without_mutating_input() {
    let tools = HashMap::from([
        ("Read".to_string(), ToolSpec::new("Read", "read a file", json!({}))),
        ("Write".to_string(), ToolSpec::new("Write", "write a file", json!({}))),
    ]);
    let permission = next::from_config(&json!({}));
    let user = message::UserInfo {
        id: "user-1".to_string(),
        session_id: "session-1".to_string(),
        time: message::UserTime { created: 123 },
        summary: None,
        agent: "build".to_string(),
        model: message::ModelRef {
            provider_id: "provider".to_string(),
            model_id: "model".to_string(),
        },
        system: None,
        tools: Some(HashMap::from([("Write".to_string(), true)])),
        variant: None,
    };

    let resolved = resolve_tools(&tools, &permission, &user);

    assert_eq!(resolved.len(), 2);
    assert!(resolved.contains_key("Read"));
    assert!(resolved.contains_key("Write"));
    assert_eq!(tools.len(), 2);
}

#[test]
fn has_tool_calls_detects_all_supported_shapes() {
    assert!(has_tool_calls(&[json!({
        "role": "assistant",
        "tool_calls": [{ "id": "call-1" }]
    })]));
    assert!(has_tool_calls(&[json!({
        "role": "tool",
        "tool_call_id": "call-1",
        "content": "done"
    })]));
    assert!(has_tool_calls(&[json!({
        "role": "assistant",
        "content": [{ "type": "tool-call", "toolCallId": "call-1" }]
    })]));
    assert!(has_tool_calls(&[json!({
        "role": "user",
        "content": [{ "type": "tool-result", "toolCallId": "call-1" }]
    })]));
}

#[test]
fn has_tool_calls_ignores_empty_or_unrelated_messages() {
    let messages: Vec<Value> = vec![
        json!({ "role": "assistant", "tool_calls": [] }),
        json!({ "role": "tool", "content": "missing id" }),
        json!({ "role": "user", "content": [{ "type": "text", "text": "hello" }] }),
        json!("not an object"),
    ];

    assert!(!has_tool_calls(&messages));
}

#[test]
fn has_tool_calls_ignores_malformed_tool_shapes() {
    let messages = vec![
        json!({ "role": "assistant", "tool_calls": { "id": "call-1" } }),
        json!({ "role": "tool", "tool_call_id": 1 }),
        json!({ "role": "user", "content": "not an array" }),
        json!({ "role": "user", "content": [null, { "type": 1 }] }),
    ];

    assert!(!has_tool_calls(&messages));
}

//! 验证系统消息构造、工具结果补全与供应商消息规范化。

use super::super::types::{AgentInfo, StreamInput};
use super::*;

use crate::app::agent::permission::next;
use crate::app::agent::provider::provider;
use crate::app::agent::session::message;
use crate::app::agent::tools::ToolSpec;
use serde_json::{Value, json};
use std::collections::HashMap;

pub(super) fn test_model(provider_id: &str, api_id: &str, adapter: &str) -> provider::Model {
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

pub(super) fn test_input(model: provider::Model) -> StreamInput {
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
        retry_in_caller: false,
    }
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

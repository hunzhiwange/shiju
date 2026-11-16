//! Raw payload construction does not depend on host persistence.
use crate::{llm_step::LlmStep};
use model_types::ToolCall;
use session_port::ui_types as models;
use serde_json::{Value, json};
fn unique_id(label: &str) -> String { format!("raw-{label}-{}", uuid::Uuid::new_v4()) }
struct Request { stream: u64, session: String, root: Option<String>, model: Option<String>, options: Value, persist_app_session_artifacts: bool }
fn request(session: &str, stream: u64, persist: bool) -> Request {
    Request { stream, session: session.into(), root: Some("/tmp/project".into()), model: Some("test-model".into()), options: json!({"mode":"test"}), persist_app_session_artifacts: persist }
}
fn persist_llm_raw_step_payload(req: &Request, session: &str, index: u32, messages: &[Value], sources: &[Value], model: &str, system: &[String], tools: &[tool_types::ToolSpec], step: &LlmStep) -> Option<Value> {
    crate::artifacts::persist_llm_raw_step_payload(&crate::artifacts::ArtifactRequest { stream: req.stream, session: &req.session, root: req.root.as_deref(), model: req.model.as_deref(), options: &req.options, persist_app_session_artifacts: req.persist_app_session_artifacts }, session, index, messages, sources, model, system, tools, step)
}
fn usage(input: i64, output: i64, cached: i64, reasoning: i64) -> models::TokenUsage {
    models::TokenUsage {
        input_tokens: input,
        output_tokens: output,
        cached_tokens: cached,
        reasoning_tokens: reasoning,
    }
}

fn llm_step() -> LlmStep {
    LlmStep {
        usage: usage(10, 20, 30, 40),
        finish_reason: Some("stop".to_string()),
        reasoning_content: "reasoning".to_string(),
        text: "step answer".to_string(),
        tool_calls: vec![ToolCall {
            kind: Default::default(),
            namespace: None,
            id: "call-1".to_string(),
            name: "test_tool".to_string(),
            arguments: "{\"ok\":true}".to_string(),
        }],
        full_messages: vec![
            json!({"role": "system", "content": "system one"}),
            json!({"role": "user", "content": "hello"}),
            json!({"role": "system", "content": 42}),
        ],
    }
}

fn tool_spec() -> tool_types::ToolSpec {
    tool_types::ToolSpec::new(
        "shell",
        "Run a shell command",
        json!({
            "type": "object",
            "properties": {"command": {"type": "string"}},
            "required": ["command"],
        }),
    )
}

#[test]
fn raw_payload_builds_system_messages_output_and_tool_calls() {
    let session_id = unique_id("raw");
    let req = request(&session_id, 91, true);
    let step = llm_step();
    let llm_messages = vec![json!({"role": "user", "content": "hello"})];

    let payload = persist_llm_raw_step_payload(
        &req,
        &session_id,
        3,
        &llm_messages,
        &[],
        "test-model",
        &["system one".to_string()],
        &[],
        &step,
    )
    .expect("raw payload should build");
    assert_eq!(payload["session_id"], session_id);
    assert_eq!(payload["step_index"], 3);
    assert_eq!(payload["model"], "test-model");
    assert_eq!(payload["system"], json!(["system one"]));
    assert_eq!(payload["messages"], json!([{"role": "user", "content": "hello"}]));
    assert_eq!(payload["message_sources"], json!([null]));
    assert_eq!(payload["output"]["text"], "step answer");
    assert_eq!(payload["output"]["reasoning_content"], "reasoning");
    assert_eq!(payload["output"]["finish_reason"], "stop");
    assert_eq!(payload["output"]["tool_calls"][0]["id"], "call-1");
    assert_eq!(payload["output"]["tool_calls"][0]["type"], "function");
    assert_eq!(payload["output"]["tool_calls"][0]["function"]["name"], "test_tool");
    assert_eq!(payload["output"]["tool_calls"][0]["function"]["arguments"], "{\"ok\":true}");
    assert_eq!(payload["output"]["usage"]["reasoning_tokens"], 40);
}

#[test]
fn raw_payload_builds_message_sources_without_provider_metadata() {
    let session_id = unique_id("raw-message-sources");
    let mut req = request(&session_id, 98, true);
    req.options[crate::llm_messages::INTERNAL_INPUT_SOURCE_OPTION] = json!({
        "kind": "user",
        "rpcId": "desktop-request-98",
        "clientTimeZone": "Asia/Shanghai",
    });
    let llm_messages = vec![json!({"role": "user", "content": "hello"})];
    let message_sources = vec![
        req.options
            .get(crate::llm_messages::INTERNAL_INPUT_SOURCE_OPTION)
            .cloned()
            .expect("input source"),
    ];

    let payload = persist_llm_raw_step_payload(
        &req,
        &session_id,
        8,
        &llm_messages,
        &message_sources,
        "test-model",
        &[],
        &[],
        &llm_step(),
    )
    .expect("raw payload should build");
    assert_eq!(
        payload["message_sources"],
        json!([{
            "kind": "user",
            "rpcId": "desktop-request-98",
            "clientTimeZone": "Asia/Shanghai",
        }])
    );
    assert!(payload["messages"][0].get("_vw_source").is_none());
    assert!(payload["messages"][0].get("source").is_none());
    let provider_options = crate::runner_protocol::provider_request_options(&req.options);
    assert!(provider_options.get("_vw_input_source").is_none());
    assert!(provider_options.get("source").is_none());
}

#[test]
fn raw_payload_builds_request_snapshot_and_tool_catalog() {
    let session_id = unique_id("raw-request-snapshot");
    let mut req = request(&session_id, 99, true);
    req.options = json!({
        "reasoning_effort": "high",
        "temperature": 0.2,
        "top_p": 0.9,
        "max_tokens": 2048,
    });

    let payload = persist_llm_raw_step_payload(
        &req,
        &session_id,
        9,
        &[],
        &[],
        "resolved/test-model",
        &["final system prompt".to_string()],
        &[tool_spec()],
        &llm_step(),
    )
    .expect("raw payload should build");
    assert_eq!(
        payload["request"],
        json!({
            "config": {
                "model": "resolved/test-model",
                "reasoning_effort": "high",
                "temperature": 0.2,
                "top_p": 0.9,
                "max_tokens": 2048,
            },
            "tools": [{
                "id": "shell",
                "description": "Run a shell command",
                "input_schema": {
                    "type": "object",
                    "properties": {"command": {"type": "string"}},
                    "required": ["command"],
                },
            }],
        })
    );
    assert_eq!(payload["system"], json!(["final system prompt"]));
    assert_eq!(payload["output"]["usage"]["reasoning_tokens"], 40);
}

#[test]
// 原始工件只保存允许的选项字段，防止把认证或内部配置写入记录。
fn raw_payload_options_are_explicitly_allowlisted() {
    let session_id = unique_id("raw-options-allowlist");
    let mut req = request(&session_id, 100, true);
    req.options = json!({
        "reasoning_effort": "medium",
        "max_output_tokens": 1024,
        "api_key": "secret-key",
        "headers": {"Authorization": "Bearer secret-token"},
        "approval": {"mode": "bypass"},
        "private_option": "must-not-persist",
        "_vw_input_source": {"kind": "user"},
    });

    let payload = persist_llm_raw_step_payload(
        &req,
        &session_id,
        10,
        &[],
        &[],
        "resolved/test-model",
        &[],
        &[],
        &llm_step(),
    )
    .expect("raw payload should build");
    assert_eq!(
        payload["request"]["config"],
        json!({
            "model": "resolved/test-model",
            "reasoning_effort": "medium",
            "max_output_tokens": 1024,
        })
    );
    let serialized = serde_json::to_string(&payload).expect("payload should serialize");
    for secret in ["secret-key", "secret-token", "private_option", "_vw_input_source", "approval"] {
        assert!(!serialized.contains(secret), "raw payload leaked {secret}");
    }
}

#[test]
fn raw_payload_serializes_empty_optional_output() {
    let session_id = unique_id("raw-empty-output");
    let mut req = request(&session_id, 94, true);
    req.model = None;
    let step = LlmStep {
        usage: usage(0, 0, 0, 0),
        finish_reason: None,
        reasoning_content: String::new(),
        text: String::new(),
        tool_calls: Vec::new(),
        full_messages: vec![
            json!({"role": "system"}),
            json!({"role": "system", "content": null}),
            json!({"role": "assistant", "content": "ignored"}),
        ],
    };

    let payload =
        persist_llm_raw_step_payload(&req, &session_id, 6, &[], &[], "test-model", &[], &[], &step)
            .expect("raw payload should build");
    assert_eq!(payload["model"], Value::Null);
    assert_eq!(payload["system"], json!([]));
    assert_eq!(payload["messages"], json!([]));
    assert_eq!(payload["output"]["text"], "");
    assert_eq!(payload["output"]["reasoning_content"], "");
    assert_eq!(payload["output"]["finish_reason"], Value::Null);
    assert_eq!(payload["output"]["tool_calls"], json!([]));
    assert_eq!(payload["output"]["usage"], json!(usage(0, 0, 0, 0)));
}

#[test]
fn raw_payload_respects_persistence_guards() {
    let session_id = unique_id("raw-skip");
    let step = llm_step();
    let no_persist = request(&session_id, 92, false);

    let payload = persist_llm_raw_step_payload(
        &no_persist,
        &session_id,
        4,
        &[],
        &[],
        "test-model",
        &[],
        &[],
        &step,
    );
    assert!(payload.is_none());

    let empty_session = request("", 93, true);
    let payload = persist_llm_raw_step_payload(
        &empty_session,
        "",
        5,
        &[],
        &[],
        "test-model",
        &[],
        &[],
        &step,
    );
    assert!(payload.is_none());
}

#[test]
fn raw_payload_build_does_not_access_storage() {
    let session_id = unique_id("raw-error");
    let req = request(&session_id, 97, true);

    let payload = persist_llm_raw_step_payload(
        &req,
        &session_id,
        7,
        &[],
        &[],
        "test-model",
        &[],
        &[],
        &llm_step(),
    )
    .expect("raw construction must produce a payload");
    assert_eq!(payload["step_index"], 7);
}

//! 验证处理器消息与原始 AI 调用工件的构造和持久化守卫。

use crate::project;
use crate::session::llm::ToolCall;
use crate::session::processor::Request;
use crate::session::session::{Message, Role, Session};
use crate::session::ui_store;
use crate::session::ui_types as models;
use serde_json::{Value, json};
use uuid::Uuid;

use super::super::llm_runner::LlmStep;
use super::{
    persist_final_ai_call_payload, persist_llm_raw_step_payload, persist_step_ai_call_payload,
    session_messages_payload,
};

fn unique_id(label: &str) -> String {
    format!("artifacts-test-{label}-{}", Uuid::new_v4())
}

fn project_scope() -> (tempfile::TempDir, String) {
    let workspace = tempfile::tempdir().expect("project workspace should create");
    let tenant = crate::app::agent::project::default_tenant_scope();
    let (project, _) = super::super::prefetch::block_on(
        crate::app::agent::project::from_directory(&tenant, workspace.path()),
    )
    .expect("project scope should resolve");
    (workspace, project.id)
}

fn request(session_id: &str, stream: u64, persist: bool) -> Request {
    Request {
        stream,
        session: session_id.to_string(),
        query: "Explain the fixture".to_string(),
        root: Some("/tmp/project".to_string()),
        model: Some("test-model".to_string()),
        reliability: crate::app::agent::config::ReliabilityConfig::default(),
        options: json!({"mode": "test"}),
        approval: None,
        channel_name: None,
        non_cli_approval_context: None,
        assistant_message_id: None,
        history: Vec::new(),
        persist_app_session_artifacts: persist,
        app_session_scope: None,
        abort_token: None,
        session_tools: crate::session::runtime::SessionToolset::default(),
    }
}

fn usage(input: i64, output: i64, cached: i64, reasoning: i64) -> models::TokenUsage {
    models::TokenUsage {
        input_tokens: input,
        output_tokens: output,
        cached_tokens: cached,
        reasoning_tokens: reasoning,
    }
}

fn session_with_all_roles(session_id: &str) -> Session {
    Session {
        id: session_id.to_string(),
        messages: vec![
            Message { role: Role::System, content: "system".to_string() },
            Message { role: Role::User, content: "user".to_string() },
            Message { role: Role::Assistant, content: "assistant".to_string() },
            Message { role: Role::Tool, content: "tool".to_string() },
        ],
    }
}

fn llm_step() -> LlmStep {
    LlmStep {
        usage: usage(10, 20, 30, 40),
        finish_reason: Some("stop".to_string()),
        reasoning_content: "reasoning".to_string(),
        text: "step answer".to_string(),
        tool_calls: vec![ToolCall {
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

fn tool_spec() -> crate::app::agent::tools::ToolSpec {
    crate::app::agent::tools::ToolSpec::new(
        "shell",
        "Run a shell command",
        json!({
            "type": "object",
            "properties": {"command": {"type": "string"}},
            "required": ["command"],
        }),
    )
}

fn register_scope_project(scope: &str) -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("project tempdir");
    let info = project::Info {
        id: scope.to_string(),
        worktree: temp.path().to_string_lossy().to_string(),
        vcs: None,
        name: None,
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 10, updated: 20, initialized: None },
        sandboxes: Vec::new(),
    };
    project::store_local_test_project(&info).expect("project should write");
    temp
}

fn register_ui_session(session_id: &str, scope: &str) {
    let session = models::ChatSession {
        id: session_id.to_string(),
        title: "Raw payload fixture".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 1,
    };
    ui_store::save_session_scoped(&session, Some(scope)).expect("parent session should persist");
}

#[test]
fn session_messages_payload_returns_empty_array_for_empty_session() {
    let session = Session { id: unique_id("empty"), messages: Vec::new() };

    assert!(session_messages_payload(&session).is_empty());
}

#[test]
fn step_and_final_ai_call_payloads_persist_to_scoped_session() {
    let session_id = unique_id("ai-call");
    let scope = unique_id("scope");
    let _project = register_scope_project(&scope);
    let req = request(&session_id, 77, true);
    let session = session_with_all_roles(&session_id);
    let step = llm_step();
    ui_store::delete_session_scoped(&session_id, Some(&scope));

    persist_step_ai_call_payload(&req, &session, &usage(1, 2, 3, 4), &step, 5, Some(&scope));

    let loaded =
        ui_store::load_session_scoped(&session_id, Some(&scope)).expect("AI call should persist");
    assert_eq!(loaded.calls.len(), 1);
    let call = &loaded.calls[0];
    assert_eq!(call["session_id"], session_id);
    assert_eq!(call["stream_id"], 77);
    assert_eq!(call["step_index"], 5);
    assert_eq!(call["model"], "test-model");
    assert_eq!(call["root"], "/tmp/project");
    assert_eq!(call["usage"]["input_tokens"], 11);
    assert_eq!(call["usage"]["output_tokens"], 22);
    assert_eq!(call["usage"]["cached_tokens"], 33);
    assert_eq!(call["usage"]["reasoning_tokens"], 44);
    assert_eq!(call["answer"], "step answer");
    assert!(call["time"]["created_ms"].is_u64());
    assert!(call["prompt"].is_string());
    assert_eq!(
        call["messages"],
        json!([
            {"role": "system", "content": "system"},
            {"role": "user", "content": "user"},
            {"role": "assistant", "content": "assistant"},
            {"role": "tool", "content": "tool"},
        ])
    );

    persist_final_ai_call_payload(
        &req,
        &session,
        &usage(3, 4, 5, 6),
        "final answer",
        Some(&scope),
        true,
    );

    let loaded = ui_store::load_session_scoped(&session_id, Some(&scope))
        .expect("final AI call should replace matching stream");
    assert_eq!(loaded.calls.len(), 1);
    assert_eq!(loaded.calls[0]["answer"], "final answer");
    assert_eq!(loaded.calls[0]["usage"]["input_tokens"], 3);

    ui_store::delete_session_scoped(&session_id, Some(&scope));
}

#[test]
fn ai_call_payloads_compact_repeated_context_before_persisting() {
    let session_id = unique_id("compact-context");
    let scope = unique_id("scope");
    let _project = register_scope_project(&scope);
    let req = request(&session_id, 96, true);
    let session = Session {
        id: session_id.clone(),
        messages: vec![
            Message { role: Role::User, content: format!("old {}", "x".repeat(32 * 1024)) },
            Message { role: Role::Assistant, content: format!("recent {}", "y".repeat(32 * 1024)) },
        ],
    };
    let step = LlmStep { text: "z".repeat(32 * 1024), ..llm_step() };
    ui_store::delete_session_scoped(&session_id, Some(&scope));

    persist_step_ai_call_payload(&req, &session, &usage(0, 0, 0, 0), &step, 1, Some(&scope));

    let loaded = ui_store::load_session_scoped(&session_id, Some(&scope))
        .expect("compacted AI call should persist");
    let call = &loaded.calls[0];
    let serialized = serde_json::to_vec(call).expect("compacted call should serialize");
    assert!(serialized.len() < 20 * 1024, "call payload is too large: {} bytes", serialized.len());
    assert_eq!(call["context_compacted"], true);
    assert!(call.get("calls").is_none(), "UI call summaries must not carry raw call records");
    assert!(call["prompt"].as_str().unwrap_or_default().contains("context compacted"));
    assert!(
        call["messages"][0]["content"].as_str().unwrap_or_default().contains("context compacted")
    );
    assert!(call["answer"].as_str().unwrap_or_default().contains("context compacted"));

    ui_store::delete_session_scoped(&session_id, Some(&scope));
}

#[test]
fn ai_call_payloads_respect_persistence_guards() {
    let session_id = unique_id("skip");
    let scope = unique_id("scope");
    let no_persist = request(&session_id, 88, false);
    let session = session_with_all_roles(&session_id);
    let step = llm_step();
    ui_store::delete_session_scoped(&session_id, Some(&scope));

    persist_step_ai_call_payload(&no_persist, &session, &usage(0, 0, 0, 0), &step, 0, Some(&scope));
    assert!(ui_store::load_session_scoped(&session_id, Some(&scope)).is_none());

    let empty_session = request("", 89, true);
    persist_step_ai_call_payload(
        &empty_session,
        &session,
        &usage(0, 0, 0, 0),
        &step,
        0,
        Some(&scope),
    );
    assert!(ui_store::load_session_scoped("", Some(&scope)).is_none());

    let req = request(&session_id, 90, true);
    persist_final_ai_call_payload(
        &req,
        &session,
        &usage(1, 1, 1, 1),
        "ignored",
        Some(&scope),
        false,
    );
    assert!(ui_store::load_session_scoped(&session_id, Some(&scope)).is_none());
}

#[test]
fn final_ai_call_payload_uses_explicit_persist_decision() {
    let session_id = unique_id("final-explicit-persist");
    let scope = unique_id("scope");
    let _project = register_scope_project(&scope);
    let req = request(&session_id, 95, false);
    let session = session_with_all_roles(&session_id);
    ui_store::delete_session_scoped(&session_id, Some(&scope));

    persist_final_ai_call_payload(
        &req,
        &session,
        &usage(7, 8, 9, 10),
        "persisted by caller decision",
        Some(&scope),
        true,
    );

    let loaded = ui_store::load_session_scoped(&session_id, Some(&scope))
        .expect("explicit final persist decision should be authoritative");
    assert_eq!(loaded.calls.len(), 1);
    assert_eq!(loaded.calls[0]["answer"], "persisted by caller decision");
    assert_eq!(loaded.calls[0]["usage"]["reasoning_tokens"], 10);

    ui_store::delete_session_scoped(&session_id, Some(&scope));
}

#[test]
fn raw_payload_builds_system_messages_output_and_tool_calls() {
    let session_id = unique_id("raw");
    let scope = unique_id("scope");
    let _project = register_scope_project(&scope);
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
    let scope = unique_id("scope");
    let _project = register_scope_project(&scope);
    let mut req = request(&session_id, 98, true);
    req.options[super::super::llm_messages::INTERNAL_INPUT_SOURCE_OPTION] = json!({
        "kind": "user",
        "rpcId": "desktop-request-98",
        "clientTimeZone": "Asia/Shanghai",
    });
    let llm_messages = vec![json!({"role": "user", "content": "hello"})];
    let message_sources = vec![
        req.options
            .get(super::super::llm_messages::INTERNAL_INPUT_SOURCE_OPTION)
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
    let provider_options = super::super::runner::provider_request_options(&req.options);
    assert!(provider_options.get("_vw_input_source").is_none());
    assert!(provider_options.get("source").is_none());
}

#[test]
fn raw_payload_builds_request_snapshot_and_tool_catalog() {
    let session_id = unique_id("raw-request-snapshot");
    let scope = unique_id("scope");
    let _project = register_scope_project(&scope);
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
fn raw_payload_options_are_explicitly_allowlisted() {
    let session_id = unique_id("raw-options-allowlist");
    let scope = unique_id("scope");
    let _project = register_scope_project(&scope);
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
    let scope = unique_id("scope");
    let _project = register_scope_project(&scope);
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
    let scope = unique_id("scope");
    let _project = register_scope_project(&scope);
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

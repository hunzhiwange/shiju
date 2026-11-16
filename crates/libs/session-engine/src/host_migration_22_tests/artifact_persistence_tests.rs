//! 验证处理器消息与原始 AI 调用工件的构造和持久化守卫。

use agent_project as project;
use model_types::ToolCall;
use super::artifacts_tests::{Request, request};
use crate::session_memory::{Message, Role, Session};
use session_store::ui_store;
use session_port::ui_types as models;
use serde_json::json;
use uuid::Uuid;

use crate::llm_step::LlmStep;


fn unique_id(label: &str) -> String {
    format!("artifacts-test-{label}-{}", Uuid::new_v4())
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
// 持久化前压缩重复上下文，降低工件体积但保留调用语义。
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

struct StoreHost;
impl crate::artifacts::ArtifactHost for StoreHost {
    fn build_prompt(session: &Session, _: Option<&str>, _: Option<&str>) -> String {
        crate::prompting::build_prompt(session, "artifact storage contract", None)
    }
    fn persist_ai_call_payload(session: &str, stream: u64, payload: &serde_json::Value, scope: Option<&str>) {
        let _ = ui_store::persist_ai_call_payload(session, stream, payload, scope);
    }
}
fn projection(req: &Request) -> crate::artifacts::ArtifactRequest<'_> {
    crate::artifacts::ArtifactRequest { stream: req.stream, session: &req.session, root: req.root.as_deref(), model: req.model.as_deref(), options: &req.options, persist_app_session_artifacts: req.persist_app_session_artifacts }
}
fn persist_step_ai_call_payload(req: &Request, session: &Session, total: &models::TokenUsage, step: &LlmStep, index: u32, scope: Option<&str>) {
    crate::artifacts::persist_step_ai_call_payload::<StoreHost>(&projection(req), session, total, step, index, scope)
}
fn persist_final_ai_call_payload(req: &Request, session: &Session, total: &models::TokenUsage, answer: &str, scope: Option<&str>, persist: bool) {
    crate::artifacts::persist_final_ai_call_payload::<StoreHost>(&projection(req), session, total, answer, scope, persist)
}

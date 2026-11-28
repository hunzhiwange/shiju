#[cfg(not(feature = "test-support"))]
#[path = "test_support/provider.rs"]
mod provider;
#[cfg(not(feature = "test-support"))]
#[path = "test_support/binding.rs"]
mod binding;
#[cfg(not(feature = "test-support"))]
use {binding::binding_fixture as make_binding_fixture, provider::BindingFixtureProvider};
#[cfg(feature = "test-support")]
use crate::test_support::{binding_fixture as make_binding_fixture, BindingFixtureProvider};
use crate::*;
use crate::native_command::{BoundCommandRequest, CommandEvent as StreamEvent, NativeCommandHost};
use serde_json::json;
use std::sync::{Arc, Mutex};

struct Request { query: String, root: String, abort_token: Option<tokio_util::sync::CancellationToken> }
fn request(root: &std::path::Path, query: &str) -> Request {
    Request { query: query.into(), root: root.to_string_lossy().into_owned(), abort_token: None }
}
fn tenant() -> vw_api_transport::TenantScope {
    vw_api_transport::TenantScope::new("workflow-binding", "company").unwrap()
}
struct CommandHost;
impl NativeCommandHost for CommandHost {
    type Event = StreamEvent;
    fn event(event: StreamEvent) -> StreamEvent { event }
    fn now_ms() -> u64 { 1 }
    fn default_runtime(&self, _: tokio::sync::mpsc::UnboundedSender<StreamEvent>, cancellation: tokio_util::sync::CancellationToken) -> WorkflowRuntime {
        WorkflowRuntime {
            run_budget: crate::WorkflowRunBudget::default().with_cancellation(cancellation),
            provider: Arc::new(BindingFixtureProvider),
            model: "unused".into(), temperature: 0.0, tool_provider: None, agent_provider: None,
            knowledge_provider: None, document_extractor: None, pause_store: None,
        }
    }
}
fn run_bound_command(request: &Request, emit: &mut (impl FnMut(StreamEvent) -> bool + Send)) -> bool {
    tokio::runtime::Runtime::new().unwrap().block_on(crate::native_command::run_command(
        &CommandHost, BoundCommandRequest { query: &request.query, root: Some(&request.root), acp_test: false, current_tenant: tenant },
        request.abort_token.clone().unwrap_or_default(), emit,
    ))
}
fn binding_fixture() -> tempfile::TempDir {
    make_binding_fixture(tenant())
}

#[test]
fn native_workflow_bound_command_executes_graph_and_returns_answer() {
    let dir = binding_fixture();
    let request = request(dir.path(), "/shiju 摘要 sp7");
    let events = Arc::new(Mutex::new(Vec::new()));
    let recorded = events.clone();
    assert!(run_bound_command(&request, &mut move |event| {
        recorded.lock().unwrap().push(event);
        true
    }));
    let events = events.lock().unwrap();
    assert!(
        events.iter().any(|e| matches!(e, StreamEvent::Delta(text) if text == "verified preview"))
    );
    assert!(matches!(events.last(), Some(StreamEvent::Done)));
}

#[test]
fn native_workflow_unmatched_command_preserves_normal_session() {
    let dir = binding_fixture();
    for query in ["/shiju 登录", "/shijux 摘要 sp7", "普通消息", "/shiju 摘要器"] {
        let request = request(dir.path(), query);
        assert!(!run_bound_command(&request, &mut |_| panic!("unexpected dispatch")));
    }
}

#[test]
fn native_workflow_skill_palette_selection_dispatches_same_graph() {
    let dir = binding_fixture();
    let events = run_events(&request(
        dir.path(),
        "摘要 sp7\n\n<session_control_selection>\n用户选中了以下上下文。\n技能：shiju\n</session_control_selection>",
    ));
    assert!(
        events.iter().any(|e| matches!(e, StreamEvent::Delta(text) if text == "verified preview"))
    );
    assert!(matches!(events.last(), Some(StreamEvent::Done)));
}

fn run_events(request: &Request) -> Vec<StreamEvent> {
    let events = Arc::new(Mutex::new(vec![]));
    let recorded = events.clone();
    assert!(run_bound_command(request, &mut move |event| {
        recorded.lock().unwrap().push(event);
        true
    }));
    std::mem::take(&mut *events.lock().unwrap())
}

#[test]
fn native_workflow_rejects_cross_tenant_binding() {
    let dir = binding_fixture();
    let path = dir.path().join(".shiju/workflow_commands.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["company_id"] = json!("other-company");
    std::fs::write(path, value.to_string()).unwrap();
    let events = run_events(&request(dir.path(), "/shiju 摘要 sp7"));
    assert!(matches!(events.last(), Some(StreamEvent::Error(error)) if error.contains("租户")));
    assert!(!events.iter().any(|e| matches!(e, StreamEvent::Done)));
}

#[test]
fn native_workflow_rejects_file_outside_project() {
    let dir = binding_fixture();
    let outside = tempfile::NamedTempFile::new().unwrap();
    let path = dir.path().join(".shiju/workflow_commands.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["commands"][0]["workflow_file"] = json!(outside.path());
    std::fs::write(path, value.to_string()).unwrap();
    let events = run_events(&request(dir.path(), "/shiju 摘要 sp7"));
    assert!(matches!(events.last(), Some(StreamEvent::Error(error)) if error.contains("目录内")));
}

#[test]
fn native_workflow_failed_node_never_reports_success() {
    let dir = binding_fixture();
    let path = dir.path().join("preview.yaml");
    let yaml =
        std::fs::read_to_string(&path).unwrap().replace("type: answer", "type: unsupported-node");
    std::fs::write(path, yaml).unwrap();
    let events = run_events(&request(dir.path(), "/shiju 摘要 sp7"));
    assert!(matches!(events.last(), Some(StreamEvent::Error(_))));
    assert!(!events.iter().any(|e| matches!(e, StreamEvent::Delta(_) | StreamEvent::Done)));
}

#[test]
fn native_workflow_preserves_cancellation_and_tool_restrictions() {
    let dir = binding_fixture();
    let mut request = request(dir.path(), "/shiju 摘要 sp7");
    let token = tokio_util::sync::CancellationToken::new();
    token.cancel();
    request.abort_token = Some(token);
    let events = run_events(&request);
    assert!(matches!(events.last(), Some(StreamEvent::Error(error)) if error == "cancelled"));
    assert!(crate::check_allowed("bash", Some(&json!(["file_read"]))).is_err());
    assert!(crate::check_allowed("file_read", Some(&json!(["file_read"]))).is_ok());
}

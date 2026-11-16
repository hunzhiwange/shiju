//! Synthetic media receipts exercise the native gateway, never video performance.
use workflow_video_edit::test_support::scenario::{Cloud, Fault, Model, Scenario};
use super::{
    entry_test_transport,
};
use crate::gateway::api::handlers::session::stream::{
    GatewayChatStreamCollected, collect_gateway_chat_stream_with_observer,
};
use serde_json::{Value, json};
use workflow_video_edit::test_support::gateway_contracts::{source, captured};
use std::{
    path::Path,
    sync::{Arc, atomic::Ordering},
};
use {
    vw_api_chat::chat::GatewayChatStreamRequest,
    vw_api_transport::id::TenantScope,
};

fn external(session: &str, scenario: Arc<Scenario>) -> entry_test_transport::TransportGuard {
    entry_test_transport::install_external(
        session,
        Arc::new(move |context| {
            (
                Arc::new(Cloud {
                    context: Arc::new(context.clone()),
                    scenario: scenario.clone(),
                    writes: Default::default(),
                }),
                Arc::new(Model(scenario.clone())),
            )
        }),
    )
}
fn request(session: &str, path: &Path, full: bool) -> GatewayChatStreamRequest {
    GatewayChatStreamRequest {
        session_id: Some(session.to_owned().into()),
        messages: vec![
            json!({"role":"user","content":format!("/shiju 剪辑 \"{}\"",path.display())}),
        ],
        model: Some("model-recovery/original".into()),
        options: Some(json!({"full_access":full})),
        ..Default::default()
    }
}
async fn run(
    root: &Path,
    session: &str,
    path: &Path,
) -> (Result<GatewayChatStreamCollected, crate::gateway::ApiError>, Vec<Value>) {
    let mut events = Vec::new();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        collect_gateway_chat_stream_with_observer(
            root.to_string_lossy().into(),
            request(session, path, true),
            |e| events.push(e.clone()),
        ),
    )
    .await
    .expect("bounded synthetic gateway run");
    (result, events)
}
fn ledger(path: &Path) -> Value {
    let database = crate::global_database::GlobalDatabase::for_home(&crate::global::paths().home).unwrap();
    workflow_video_edit::test_support::gateway_contracts::ledger(&database, path)
}
fn failure(result: Result<GatewayChatStreamCollected, crate::gateway::ApiError>, events: &[Value]) {
    assert!(result.is_err(), "failed chain must reject collection");
    workflow_video_edit::test_support::gateway_contracts::terminal_failure(events);
}
fn success(
    root: &Path,
    session: &str,
    result: GatewayChatStreamCollected,
    events: &[Value],
    s: &Scenario,
) {
    workflow_video_edit::test_support::gateway_contracts::terminal_success(root, result.message_id.as_deref(), &result.output, events);
    let tenant = crate::session::ui_store::current_tenant_scope();
    let dir = root.to_string_lossy();
    let scope =
        crate::session::ui_store::resolve_session_scope_id_for_tenant(&tenant, Some(&dir), None)
            .unwrap();
    let snapshot =
        crate::session::ui_store::load_session_scoped_for_tenant(&tenant, session, Some(&scope))
            .unwrap();
    assert!(snapshot.messages.last().unwrap().content.contains("test-preview"));
    assert!(snapshot.steps[0].tool_timings.len() >= 10);
    assert!(snapshot.message_ids.iter().any(|id| id == &result.message_id));
    for chunk in captured(s, "model.start") {
        let event = events
            .iter()
            .find(|e| {
                e["type"] == "chat.tool"
                    && e["payload"]["chunk_id"] == chunk
                    && e["payload"]["node_id"] == "model"
                    && e["kind"] == "complete"
            })
            .expect("completed child event");
        let call_id = event["call_id"].as_str().unwrap();
        assert!(
            snapshot.steps[0].tool_timings.iter().any(|timing| timing.name == "workflow"
                && timing.call_id == call_id
                && timing.finished_ms.is_some()),
            "missing persisted child trace: {chunk}"
        );
    }
    assert_eq!(snapshot.steps[0].finish_reason.as_deref(), Some("workflow"));
    workflow_video_edit::test_support::gateway_contracts::shared_budget(s);
    let other = TenantScope::new("e2e-other-platform", "e2e-other-company").unwrap();
    assert!(
        crate::session::ui_store::load_session_scoped_for_tenant(&other, session, Some(&scope))
            .is_none()
    );
    let calls = s.capture.lock().unwrap();
    let capture = std::env::temp_dir().join(format!("shiju-task13-{session}.json"));
    std::fs::write(&capture, serde_json::to_vec_pretty(&*calls).unwrap()).unwrap();
    eprintln!(
        "synthetic duration={} shared_peak={} write_peak={} capture={}",
        s.duration(),
        s.peak.load(Ordering::SeqCst),
        s.write_peak.load(Ordering::SeqCst),
        capture.display()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn video_edit_e2e_long_transcripts_reach_done_and_reload() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    workflow_video_edit::test_support::gateway_acceptance::video_edit_e2e_long_transcripts_reach_done_and_reload(&NativeGatewayAcceptance).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn video_edit_e2e_quota_invalid_segment_and_fake_preview_never_done() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    workflow_video_edit::test_support::gateway_acceptance::video_edit_e2e_quota_invalid_segment_and_fake_preview_never_done(&NativeGatewayAcceptance).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn video_edit_e2e_accepted_timeout_reconciles_without_duplicate_post() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    workflow_video_edit::test_support::gateway_acceptance::video_edit_e2e_accepted_timeout_reconciles_without_duplicate_post(&NativeGatewayAcceptance).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn video_edit_e2e_independent_requests_have_distinct_run_budgets() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    workflow_video_edit::test_support::gateway_acceptance::video_edit_e2e_independent_requests_have_distinct_run_budgets(&NativeGatewayAcceptance).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn video_edit_e2e_native_permission_denial_prevents_external_io() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    crate::config::update_project(
        root.path(),
        json!({"autonomy":{"workspace_only":true,"allowed_roots":[]}}),
    )
    .await
    .unwrap();
    let path = source(outside.path(), "denied");
    let session = format!("ses_e2e_{}", uuid::Uuid::new_v4());
    let scenario = Scenario::new(600, Fault::None);
    let _external = external(&session, scenario.clone());
    let mut events = Vec::new();
    let result = collect_gateway_chat_stream_with_observer(
        root.path().to_string_lossy().into(),
        request(&session, &path, false),
        |e| events.push(e.clone()),
    )
    .await;
    failure(result, &events);
    assert!(scenario.capture.lock().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn video_edit_e2e_final_save_failure_never_done() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let root = tempfile::tempdir().unwrap();
    let path = source(root.path(), "save failure");
    let session = format!("ses_e2e_{}", uuid::Uuid::new_v4());
    let scenario = Scenario::new(600, Fault::None);
    let _external = external(&session, scenario.clone());
    let db = crate::global_database::GlobalDatabase::new(
        std::env::temp_dir()
            .join(format!("vw-agent-test-global-store-{}.sqlite3", std::process::id())),
    )
    .unwrap();
    let _store = crate::session::store::SessionStore::new(db.clone()).unwrap();
    let conn = db.connect_sqlite().unwrap();
    conn.execute_batch(&format!("CREATE TRIGGER task13_reject_final_save BEFORE INSERT ON session_messages WHEN NEW.session_id = '{session}' AND NEW.content LIKE '%云端剪辑已完成%' BEGIN SELECT RAISE(ABORT, 'synthetic final save failure'); END;")).unwrap();
    struct Trigger(rusqlite::Connection);
    impl Drop for Trigger {
        fn drop(&mut self) {
            self.0.execute_batch("DROP TRIGGER task13_reject_final_save").unwrap();
        }
    }
    let _trigger = Trigger(conn);
    let (result, events) = run(root.path(), &session, &path).await;
    failure(result, &events);
    assert!(!captured(&scenario, "project.write").is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn video_edit_e2e_cancelled_restart_reconciles_and_never_resubmits() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    workflow_video_edit::test_support::gateway_acceptance::video_edit_e2e_cancelled_restart_reconciles_and_never_resubmits(&NativeGatewayAcceptance).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn video_edit_e2e_template_edit_changes_next_native_model_request() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let database =
        crate::global_database::GlobalDatabase::for_home(&crate::global::paths().home).unwrap();
    let tenant = crate::session::ui_store::current_tenant_scope();
    let edit = workflow_video_edit::test_support::template_fixture::edit(&database, &tenant).await;
    let digest = edit.digest.clone();
    let root = tempfile::tempdir().unwrap();
    let path = source(root.path(), "template edit");
    let session = format!("ses_e2e_{}", uuid::Uuid::new_v4());
    let scenario = Scenario::new(300, Fault::None);
    let _external = external(&session, scenario.clone());
    let (result, events) = run(root.path(), &session, &path).await;
    edit.restore(database, tenant).await;
    success(root.path(), &session, result.unwrap(), &events, &scenario);
    assert!(captured(&scenario, "model.prompt").iter().any(|p| p.contains("E2E_TEMPLATE_EDIT")));
    assert_eq!(ledger(&path)["template_sha256"], digest);
}

struct NativeGatewayAcceptance;
#[async_trait::async_trait]
impl workflow_video_edit::test_support::gateway_acceptance::GatewayAcceptance for NativeGatewayAcceptance {
    fn install(&self, session: &str, scenario: Arc<Scenario>) -> Box<dyn Send> {
        Box::new(external(session, scenario))
    }
    fn ledger(&self, path: &Path) -> Value { ledger(path) }
    async fn run_cancelled(&self, root: &Path, session: &str, path: &Path, scenario: &Scenario) {
        use crate::gateway::api::handlers::session::stream::collect_video_edit_test_stream;
        let token = tokio_util::sync::CancellationToken::new();
        let mut events = Vec::new();
        let cancelled = async {
            tokio::time::timeout(std::time::Duration::from_secs(30), async {
                while captured(&scenario, "asset.done").len() < 5 {
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
            })
            .await
            .unwrap();
            token.cancel();
        };
        let stream = collect_video_edit_test_stream(
            root.to_string_lossy().into(),
            request(session, path, true),
            token.clone(),
            |e| events.push(e.clone()),
        );
        let (result, ()) = tokio::join!(stream, cancelled);
        failure(result, &events);
    }
    async fn run(&self, root: &Path, session: &str, path: &Path, scenario: &Scenario, succeeds: bool) {
        let (result, events) = run(root, session, path).await;
        if succeeds { success(root, session, result.unwrap(), &events, scenario); }
        else { failure(result, &events); }
    }
}

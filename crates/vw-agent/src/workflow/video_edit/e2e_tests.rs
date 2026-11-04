//! Synthetic media receipts exercise the native gateway, never video performance.
use super::{
    e2e_test_cloud::{Cloud, Fault, Model, Scenario},
    entry_test_transport,
};
use crate::gateway::api::handlers::session::stream::{
    GatewayChatStreamCollected, collect_gateway_chat_stream_with_observer,
};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{Arc, atomic::Ordering},
};
use vw_api_types::{chat::GatewayChatStreamRequest, id::TenantScope};

fn external(session: &str, scenario: Arc<Scenario>) -> entry_test_transport::TransportGuard {
    entry_test_transport::install_external(
        session,
        Arc::new(move |context| {
            (
                Arc::new(Cloud {
                    context: context.clone(),
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
fn source(root: &Path, tag: &str) -> std::path::PathBuf {
    let path = root.join(format!("中文 原片 {tag}.mp4"));
    std::fs::write(&path, format!("SYNTHETIC TEST DATA ONLY {tag}")).unwrap();
    path
}
fn runs() -> Vec<Value> {
    let database =
        crate::global_database::GlobalDatabase::for_home(&crate::global::paths().home).unwrap();
    let conn = database.connect_sqlite().unwrap();
    conn.prepare("SELECT payload FROM workflow_video_edit_runs")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| serde_json::from_str(&r.unwrap()).unwrap())
        .collect()
}
fn ledger(path: &Path) -> Value {
    runs()
        .into_iter()
        .find(|r| r["source"]["path"] == path.canonicalize().unwrap().to_string_lossy().as_ref())
        .unwrap()
}
fn captured(s: &Scenario, kind: &str) -> Vec<String> {
    s.capture.lock().unwrap().iter().filter(|(k, _)| k == kind).map(|(_, id)| id.clone()).collect()
}
fn failure(result: Result<GatewayChatStreamCollected, crate::gateway::ApiError>, events: &[Value]) {
    assert!(result.is_err(), "failed chain must reject collection");
    assert!(!events.iter().any(|e| e["type"] == "chat.done"));
    assert!(events.iter().any(|e| e["type"] == "chat.error"));
}
fn success(
    root: &Path,
    session: &str,
    result: GatewayChatStreamCollected,
    events: &[Value],
    s: &Scenario,
) {
    assert!(result.message_id.as_deref().unwrap().starts_with("msg_"));
    assert!(result.output.contains("test-preview"));
    assert_eq!(events.last().unwrap()["type"], "chat.done");
    assert!(!root.join(".shiju/workflow_commands.json").exists());
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
    assert_eq!(s.peak.load(Ordering::SeqCst), 5, "shared model/asset external peak");
    assert_eq!(s.write_peak.load(Ordering::SeqCst), 1);
    assert_eq!(s.active.load(Ordering::SeqCst), 0);
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
    let mut run_ids = Vec::new();
    for duration in [300, 600] {
        let root = tempfile::tempdir().unwrap();
        let path = source(root.path(), &duration.to_string());
        let session = format!("ses_e2e_{}", uuid::Uuid::new_v4());
        let scenario = Scenario::new(duration, Fault::None);
        let _transport = external(&session, scenario.clone());
        let (result, events) = run(root.path(), &session, &path).await;
        success(root.path(), &session, result.unwrap(), &events, &scenario);
        let row = ledger(&path);
        assert_eq!(row["transcript"]["Ready"]["duration_sec"].as_f64(), Some(f64::from(duration)));
        run_ids.push(row["run_id"].clone());
        let starts = captured(&scenario, "model.start");
        let done = captured(&scenario, "model.done");
        assert!(starts.len() >= 5);
        assert_eq!(done[..5], starts[..5].iter().rev().cloned().collect::<Vec<_>>());
        let jobs = row["asset_jobs"].as_array().unwrap();
        assert_eq!(jobs.len(), starts.len());
        assert!(jobs.iter().all(|j| j["state"] == "Succeeded" && j["attempts"] == 1));
        if duration == 600 {
            assert!(starts.len() > 5);
            assert!(jobs.len() > 5);
        }
        assert_eq!(captured(&scenario, "asset.submit").len(), jobs.len());
        let posts = captured(&scenario, "asset.submit");
        let ready = captured(&scenario, "asset.done");
        assert_eq!(ready[..5], posts[..5].iter().rev().cloned().collect::<Vec<_>>());
        assert_eq!(captured(&scenario, "project.create").len(), 1);
        assert_eq!(captured(&scenario, "upload.put").len(), 1);
        assert_eq!(captured(&scenario, "transcript.submit").len(), 1);
        assert_eq!(captured(&scenario, "preview.read").len(), 1);
        assert!(
            captured(&scenario, "project.write")
                .iter()
                .all(|w| !w.to_lowercase().contains("export"))
        );
    }
    assert_ne!(run_ids[0], run_ids[1]);
}

#[tokio::test(flavor = "multi_thread")]
async fn video_edit_e2e_quota_invalid_segment_and_fake_preview_never_done() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    for fault in [Fault::Quota, Fault::InvalidChunk, Fault::FakePreview] {
        let root = tempfile::tempdir().unwrap();
        let path = source(root.path(), &format!("{fault:?}"));
        let session = format!("ses_e2e_{}", uuid::Uuid::new_v4());
        let scenario = Scenario::new(600, fault);
        let _external = external(&session, scenario.clone());
        let (result, events) = run(root.path(), &session, &path).await;
        failure(result, &events);
        if fault != Fault::FakePreview {
            assert!(captured(&scenario, "project.write").is_empty());
            assert!(captured(&scenario, "asset.submit").is_empty());
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn video_edit_e2e_accepted_timeout_reconciles_without_duplicate_post() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let root = tempfile::tempdir().unwrap();
    let path = source(root.path(), "accepted timeout");
    let session = format!("ses_e2e_{}", uuid::Uuid::new_v4());
    let scenario = Scenario::new(600, Fault::AcceptedTimeout);
    let _external = external(&session, scenario.clone());
    let (result, events) = run(root.path(), &session, &path).await;
    success(root.path(), &session, result.unwrap(), &events, &scenario);
    let submitted = captured(&scenario, "asset.submit");
    let reconciled = captured(&scenario, "asset.reconcile");
    assert_eq!(submitted.len(), submitted.iter().collect::<std::collections::BTreeSet<_>>().len());
    assert_eq!(
        submitted.iter().collect::<std::collections::BTreeSet<_>>(),
        reconciled.iter().collect()
    );
    assert!(
        ledger(&path)["asset_jobs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|j| j["state"] == "Succeeded" && j["attempts"] == 1)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn video_edit_e2e_independent_requests_have_distinct_run_budgets() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let pa = source(a.path(), "parallel-a");
    let pb = source(b.path(), "parallel-b");
    std::fs::write(&pb, std::fs::read(&pa).unwrap()).unwrap();
    let sa = format!("ses_e2e_{}", uuid::Uuid::new_v4());
    let sb = format!("ses_e2e_{}", uuid::Uuid::new_v4());
    let ca = Scenario::new(600, Fault::None);
    let cb = Scenario::new(600, Fault::None);
    let _ta = external(&sa, ca.clone());
    let _tb = external(&sb, cb.clone());
    let ((ra, ea), (rb, eb)) = tokio::join!(run(a.path(), &sa, &pa), run(b.path(), &sb, &pb));
    success(a.path(), &sa, ra.unwrap(), &ea, &ca);
    success(b.path(), &sb, rb.unwrap(), &eb, &cb);
    assert_ne!(ledger(&pa)["run_id"], ledger(&pb)["run_id"]);
    assert_ne!(ledger(&pa)["project_id"], ledger(&pb)["project_id"]);
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
    use crate::gateway::api::handlers::session::stream::collect_video_edit_test_stream;
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let root = tempfile::tempdir().unwrap();
    let path = source(root.path(), "cancel restart");
    let session = format!("ses_e2e_{}", uuid::Uuid::new_v4());
    let scenario = Scenario::new(600, Fault::AcceptedTimeout);
    let _external = external(&session, scenario.clone());
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
        root.path().to_string_lossy().into(),
        request(&session, &path, true),
        token.clone(),
        |e| events.push(e.clone()),
    );
    let (result, ()) = tokio::join!(stream, cancelled);
    failure(result, &events);
    // The graph owns async node tasks; wait for their durable cancellation cleanup
    // before simulating the next process. A lost lease/save must time out here.
    let before = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let row = ledger(&path);
            if row["asset_cancelled"] == true {
                break row;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("cancelled graph must persist cleanup before restart");
    let planned = captured(&scenario, "model.start");
    let posted = captured(&scenario, "asset.submit");
    assert_eq!(posted.len(), 5);
    let restarted = format!("ses_e2e_{}", uuid::Uuid::new_v4());
    let _restart = external(&restarted, scenario.clone());
    let (result, events) = run(root.path(), &restarted, &path).await;
    failure(result, &events);
    assert_eq!(captured(&scenario, "asset.submit"), posted);
    assert_eq!(
        captured(&scenario, "model.start"),
        planned,
        "cancelled remote work must be reconciled before any new planning"
    );
    assert_eq!(scenario.peak.load(Ordering::SeqCst), 5);
    assert_eq!(captured(&scenario, "asset.reconcile").len(), 5);
    assert_eq!(ledger(&path)["run_id"], before["run_id"]);
    assert!(captured(&scenario, "project.write").is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn video_edit_e2e_template_edit_changes_next_native_model_request() {
    use crate::workflow::templates::*;
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let database =
        crate::global_database::GlobalDatabase::for_home(&crate::global::paths().home).unwrap();
    let tenant = crate::session::ui_store::current_tenant_scope();
    let seed = VideoEditTemplateSeed::production().unwrap();
    let original = seed.load(database.clone(), tenant.clone()).await.unwrap();
    let mut yaml: Value = serde_yaml::from_str(&original.record.workflow_yaml).unwrap();
    let brief = yaml["workflow"]["graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["id"] == "brief")
        .unwrap();
    let text = brief["data"]["prompt_template"][0]["text"].as_str().unwrap();
    brief["data"]["prompt_template"][0]["text"] = json!(format!("{text} E2E_TEMPLATE_EDIT"));
    let body = vw_api_types::workflow::WorkflowRecordUpsertBody {
        uuid: Some(VIDEO_EDIT_TEMPLATE_UUID.into()),
        name: VIDEO_EDIT_TEMPLATE_NAME.into(),
        description: String::new(),
        workflow_yaml: serde_json::to_string(&yaml).unwrap(),
    };
    let edited =
        save_template(database.clone(), tenant.clone(), body, original.record.updated_at_ms)
            .await
            .unwrap();
    let digest = seed.load(database.clone(), tenant.clone()).await.unwrap().digest;
    assert_ne!(digest, original.digest);
    let other = TenantScope::new(format!("e2e-{}", uuid::Uuid::new_v4()), "other-company").unwrap();
    assert!(
        !seed
            .load(database.clone(), other)
            .await
            .unwrap()
            .record
            .workflow_yaml
            .contains("E2E_TEMPLATE_EDIT")
    );
    let root = tempfile::tempdir().unwrap();
    let path = source(root.path(), "template edit");
    let session = format!("ses_e2e_{}", uuid::Uuid::new_v4());
    let scenario = Scenario::new(300, Fault::None);
    let _external = external(&session, scenario.clone());
    let (result, events) = run(root.path(), &session, &path).await;
    save_template(
        database,
        tenant,
        vw_api_types::workflow::WorkflowRecordUpsertBody {
            uuid: Some(VIDEO_EDIT_TEMPLATE_UUID.into()),
            name: VIDEO_EDIT_TEMPLATE_NAME.into(),
            description: String::new(),
            workflow_yaml: original.record.workflow_yaml,
        },
        edited.updated_at_ms,
    )
    .await
    .unwrap();
    success(root.path(), &session, result.unwrap(), &events, &scenario);
    assert!(captured(&scenario, "model.prompt").iter().any(|p| p.contains("E2E_TEMPLATE_EDIT")));
    assert_eq!(ledger(&path)["template_sha256"], digest);
}

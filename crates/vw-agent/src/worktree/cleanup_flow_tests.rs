use super::{cleanup, management, recovery, registry};
use axum::{
    Extension,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tower::ServiceExt;
use vw_api_types::{
    id::{ProjectScope, TenantScope},
    worktree::DeleteManagedWorktreeRequest,
};
use vw_config_types::worktree::WorktreeSettings;

// A process owns all lazy stores and environment variables. Never mutate a live
// test runner's environment or share tenant databases with a parallel fixture.
fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_WORKTREE_CLEANUP_FLOW_CHILD";
    if std::env::var(CHILD).as_deref() == Ok(test) {
        return true;
    }
    let root = tempfile::tempdir().unwrap();
    let name = format!("{}::{test}", module_path!().split_once("::").unwrap().1);
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &name, "--nocapture"])
        .env(CHILD, test)
        .env("VIBEWINDOW_TEST_ROOT", root.path())
        .env("VW_ACP_MOUNT_RECOVERY_DB_ROOT", root.path())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", root.path().join("no-global-gitconfig"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&result.stdout);
    print!("{stdout}");
    eprint!("{}", String::from_utf8_lossy(&result.stderr));
    assert!(stdout.contains("running 1 test"), "child must execute one real test: {test}");
    assert!(result.status.success(), "isolated cleanup flow failed: {test}");
    false
}

async fn refresh(
    scope: &ProjectScope,
    primary: &Path,
    settings: &WorktreeSettings,
) -> Result<cleanup::FlowStatus, super::Error> {
    cleanup::refresh(scope, primary, settings, &tokio_util::sync::CancellationToken::new()).await
}

fn git(path: &Path, args: &[&str]) {
    let output = std::process::Command::new("git").arg("-C").arg(path).args(args).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
struct Fixture {
    temp: tempfile::TempDir,
    primary: PathBuf,
    scope: ProjectScope,
    settings: WorktreeSettings,
}
impl Fixture {
    async fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let primary = temp.path().join("primary");
        std::fs::create_dir(&primary).unwrap();
        git(&primary, &["init", "-b", "main"]);
        git(
            &primary,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.test",
                "commit",
                "--allow-empty",
                "-m",
                "initial",
            ],
        );
        let tenant = TenantScope::new("cleanup-flow", uuid::Uuid::new_v4().to_string()).unwrap();
        let (project, _) = crate::project::from_directory(&tenant, &primary).await.unwrap();
        let scope = ProjectScope::new(tenant.clone(), project.id.into()).unwrap();
        let settings = WorktreeSettings {
            keep_count: 1,
            root: temp.path().join("restored").to_string_lossy().into_owned(),
            ..Default::default()
        };
        crate::config::update_global_for_tenant(&tenant, json!({"worktree_settings": settings}))
            .await
            .unwrap();
        Self { temp, primary, scope, settings }
    }
    async fn checkout(&self, name: &str) -> registry::Record {
        let path = self.temp.path().join(name);
        git(&self.primary, &["worktree", "add", "-b", name, path.to_str().unwrap()]);
        registry::register(&self.scope, &self.primary, &path).await.unwrap()
    }
    async fn call(&self, method: &str, suffix: &str, body: Value) -> (StatusCode, Value) {
        let app = crate::gateway::api::handlers::project::router::<()>()
            .layer(Extension(self.scope.tenant_scope().clone()));
        let response = app
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(format!("/projects/{}/{}", self.scope.project_id().as_ref(), suffix))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }
}
#[tokio::test]
async fn worktree_cleanup_flow_waits_for_confirmation() {
    if !isolated("worktree_cleanup_flow_waits_for_confirmation") {
        return;
    }
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    f.checkout("new").await;
    let report = refresh(&f.scope, &f.primary, &f.settings).await.unwrap();
    assert_eq!(report.proposal.candidates.len(), 1);
    assert!(report.deleted.is_empty());
    assert!(recovery::list(&f.scope, &f.primary).await.unwrap().is_empty());
    let target = format!("managed-worktrees/{}", old.id);
    let (code, p) = f.call("POST", &format!("{target}/deletion-preflight"), json!({})).await;
    assert_eq!(code, StatusCode::OK, "{p}");
    let (code, _) =
        f.call("DELETE", &target, json!({"fingerprint":p["fingerprint"],"confirmed":false})).await;
    assert!(!code.is_success());
    assert!(old.directory.exists());
    assert!(recovery::list(&f.scope, &f.primary).await.unwrap().is_empty());
    let mut disabled = f.settings.clone();
    disabled.auto_cleanup_enabled = false;
    assert!(refresh(&f.scope, &f.primary, &disabled).await.unwrap().proposal.candidates.is_empty());
}
#[tokio::test]
async fn worktree_cleanup_flow_rechecks_newly_busy_checkout() {
    if !isolated("worktree_cleanup_flow_rechecks_newly_busy_checkout") {
        return;
    }
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    f.checkout("new").await;
    let p = management::preflight(&f.scope, &f.primary, &old.id, &f.settings).await.unwrap();
    let lease = registry::use_lease(&old.directory).unwrap();
    let result = management::delete(
        &f.scope,
        &f.primary,
        &old.id,
        &f.settings,
        &DeleteManagedWorktreeRequest {
            fingerprint: p.fingerprint,
            confirmed: true,
            confirm_ignored_loss: true,
        },
    )
    .await;
    assert!(result.is_err(), "a newly running checkout must refuse deletion");
    assert!(old.directory.exists());
    let proposal = cleanup::propose(&f.scope, &f.primary, &f.settings).await.unwrap();
    assert!(
        proposal
            .protected
            .iter()
            .any(|p| p.id == old.id && p.reasons.contains(&cleanup::ProtectionReason::Busy))
    );
    drop(lease);
    let lifecycle = registry::lifecycle_lock(&f.primary).unwrap();
    assert!(registry::use_lease(&old.directory).is_err());
    drop(lifecycle);
    assert!(registry::use_lease(&old.directory).unwrap().is_some());
}
#[tokio::test]
async fn worktree_cleanup_flow_restore_is_visible() {
    if !isolated("worktree_cleanup_flow_restore_is_visible") {
        return;
    }
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    f.checkout("new").await;
    std::fs::write(old.directory.join("saved"), "recover me").unwrap();
    let target = format!("managed-worktrees/{}", old.id);
    let (_, p) = f.call("POST", &format!("{target}/deletion-preflight"), json!({})).await;
    let (code, deleted) =
        f.call("DELETE", &target, json!({"fingerprint":p["fingerprint"],"confirmed":true})).await;
    assert_eq!(code, StatusCode::OK, "{deleted}");
    let (_, list) = f.call("GET", "worktree-snapshots", json!({})).await;
    assert_eq!(list["items"][0]["id"], deleted["snapshot_id"]);
    let (code, restored) = f
        .call(
            "POST",
            &format!("worktree-snapshots/{}/restore", deleted["snapshot_id"].as_str().unwrap()),
            json!({}),
        )
        .await;
    assert_eq!(code, StatusCode::OK, "{restored}");
    let path = Path::new(restored["worktree"]["worktree"]["directory"].as_str().unwrap());
    assert_eq!(std::fs::read_to_string(path.join("saved")).unwrap(), "recover me");
    let (_, list) = f.call("GET", "managed-worktrees", json!({})).await;
    assert!(
        list["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["recovered_from"] == deleted["snapshot_id"])
    );
}
#[tokio::test]
async fn worktree_cleanup_flow_automatic_partial_failure_is_recoverable() {
    if !isolated("worktree_cleanup_flow_automatic_partial_failure_is_recoverable") {
        return;
    }
    let mut f = Fixture::new().await;
    let ignored = f.checkout("ignored-tree").await;
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let old = f.checkout("old").await;
    f.checkout("new").await;
    std::fs::write(f.primary.join(".git/info/exclude"), "ignored\n").unwrap();
    std::fs::write(ignored.directory.join("ignored"), "never discard automatically").unwrap();
    std::fs::write(old.directory.join("saved"), "automatic recovery").unwrap();
    f.settings.ask_before_delete = false;
    crate::config::update_global_for_tenant(
        f.scope.tenant_scope(),
        json!({"worktree_settings": f.settings}),
    )
    .await
    .unwrap();
    let report = refresh(&f.scope, &f.primary, &f.settings).await.unwrap();
    assert_eq!(report.deleted.len(), 1, "{report:?}");
    assert!(report.errors.contains_key(&ignored.id), "{report:?}");
    assert!(ignored.directory.exists());
    assert!(!old.directory.exists());
    let persisted = cleanup::flow_status(&f.scope).await.unwrap();
    assert_eq!(persisted.deleted, report.deleted);
    assert_eq!(persisted.errors, report.errors);
    let snapshot = recovery::list(&f.scope, &f.primary).await.unwrap();
    assert_eq!(snapshot.len(), 1);
    let restored = recovery::restore(&f.scope, &f.primary, &snapshot[0].id).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(restored.record.directory.join("saved")).unwrap(),
        "automatic recovery"
    );
    let (_, status) = f.call("GET", "worktree-cleanup-status", json!({})).await;
    assert_eq!(status["errors"][&ignored.id], report.errors[&ignored.id]);
}

#[tokio::test]
async fn worktree_cleanup_flow_snapshot_failure_keeps_checkout_and_can_retry() {
    if !isolated("worktree_cleanup_flow_snapshot_failure_keeps_checkout_and_can_retry") {
        return;
    }
    let mut f = Fixture::new().await;
    let old = f.checkout("old").await;
    f.checkout("new").await;
    f.settings.ask_before_delete = false;
    crate::config::update_global_for_tenant(
        f.scope.tenant_scope(),
        json!({"worktree_settings": f.settings}),
    )
    .await
    .unwrap();
    let blocker = f.primary.join(".git/refs/shiju-worktree-recovery");
    std::fs::write(&blocker, "block snapshot publication").unwrap();
    let report = refresh(&f.scope, &f.primary, &f.settings).await.unwrap();
    assert!(report.errors.contains_key(&old.id));
    assert!(old.directory.exists());
    assert!(report.deleted.is_empty());
    std::fs::remove_file(blocker).unwrap();
    assert_eq!(refresh(&f.scope, &f.primary, &f.settings).await.unwrap().deleted.len(), 1);
}

#[tokio::test]
async fn worktree_cleanup_flow_rejects_settings_changes_and_stale_candidates() {
    if !isolated("worktree_cleanup_flow_rejects_settings_changes_and_stale_candidates") {
        return;
    }
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    f.checkout("new").await;
    let preflight =
        management::preflight(&f.scope, &f.primary, &old.id, &f.settings).await.unwrap();
    let mut changed = f.settings.clone();
    changed.auto_cleanup_enabled = false;
    crate::config::update_global_for_tenant(
        f.scope.tenant_scope(),
        json!({"worktree_settings":changed}),
    )
    .await
    .unwrap();
    let request = DeleteManagedWorktreeRequest {
        fingerprint: preflight.fingerprint,
        confirmed: true,
        confirm_ignored_loss: true,
    };
    assert!(
        management::delete(&f.scope, &f.primary, &old.id, &f.settings, &request).await.is_err()
    );
    assert!(old.directory.exists());
    let (_, status) = f.call("GET", "worktree-cleanup-status", json!({})).await;
    assert_eq!(status["proposal"]["candidates"], json!([]));
}

fn request(root: &Path) -> crate::session::processor::Request {
    crate::session::processor::Request {
        stream: 1,
        session: uuid::Uuid::new_v4().to_string(),
        query: "hello".into(),
        root: Some(root.to_string_lossy().into_owned()),
        model: None,
        reliability: Default::default(),
        options: json!({}),
        approval: None,
        channel_name: None,
        non_cli_approval_context: None,
        assistant_message_id: None,
        history: vec![],
        persist_app_session_artifacts: false,
        app_session_scope: None,
        abort_token: None,
        session_tools: Default::default(),
        session_skills: Default::default(),
    }
}

#[tokio::test]
async fn worktree_cleanup_flow_runtime_and_detached_shell_hold_leases() {
    if !isolated("worktree_cleanup_flow_runtime_and_detached_shell_hold_leases") {
        return;
    }
    use crate::session::{
        processor::StreamEvent,
        runtime::{PromptEvent, start_prompt_with_runner},
    };
    use crate::tools::shell::processes::{ShellProcessScope, ShellProcesses};
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    let (started, ready) = std::sync::mpsc::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let turn = start_prompt_with_runner(request(&old.directory), move |_, mut emit| {
        started.send(()).unwrap();
        wait.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        emit(StreamEvent::Done(Default::default()));
    });
    ready.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
    assert!(registry::ensure_not_in_use(&old).is_err());
    release.send(()).unwrap();
    loop {
        if matches!(
            turn.events.recv_timeout(std::time::Duration::from_secs(10)).unwrap(),
            PromptEvent::Terminated(_)
        ) {
            break;
        }
    }
    for _ in 0..100 {
        if registry::ensure_not_in_use(&old).is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(registry::ensure_not_in_use(&old).is_ok());
    let host = ShellProcesses::new().unwrap();
    let scope = ShellProcessScope::new("fixture".into(), &f.primary, "detached".into()).unwrap();
    let cwd = old.directory.clone();
    let process = host
        .start(scope.clone(), move || {
            let mut command = crate::shell::tokio_system_command("git");
            command.args(["-c", "alias.hold=!sleep 30", "hold"]).current_dir(cwd);
            Ok(command)
        })
        .await
        .unwrap();
    assert!(registry::ensure_not_in_use(&old).is_err());
    host.cancel(&scope, &process.process_id).await.unwrap();
    host.shutdown().unwrap();
    for _ in 0..100 {
        if registry::ensure_not_in_use(&old).is_ok() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("execution lease was not released after runtime and child exit");
}

#[tokio::test]
async fn worktree_cleanup_flow_session_creation_cannot_cross_removal() {
    if !isolated("worktree_cleanup_flow_session_creation_cannot_cross_removal") {
        return;
    }
    use crate::session::runtime::{CreateSessionInput, SessionScope, create_session};
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    let scope = SessionScope::new(f.scope.tenant_scope().clone(), &old.directory).unwrap();
    let lock = registry::lifecycle_lock(&f.primary).unwrap();
    assert!(
        create_session(CreateSessionInput {
            scope,
            parent_id: None,
            title: None,
            permission: None
        })
        .await
        .is_err()
    );
    drop(lock);
    assert!(old.directory.exists());
}

#[tokio::test]
async fn worktree_cleanup_flow_host_refreshes_deduplicates_and_stops() {
    if !isolated("worktree_cleanup_flow_host_refreshes_deduplicates_and_stops") {
        return;
    }
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    f.checkout("new").await;
    let first = super::cleanup_host::HostCleanup::start().unwrap();
    let second = super::cleanup_host::HostCleanup::start().unwrap();
    let mut refreshed = false;
    for _ in 0..200 {
        let report = cleanup::flow_status(&f.scope).await.unwrap();
        if report.proposal.candidates.iter().any(|c| c.id == old.id) {
            refreshed = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(refreshed, "startup must enumerate the existing scoped registry");
    assert!(old.directory.exists(), "no desktop connection authorizes default deletion");
    let path = vw_config_types::paths::home_config_dir(&crate::global::paths().home)
        .join("storage/worktree-cleanup/host.lock");
    let lock = std::fs::OpenOptions::new().read(true).write(true).open(path).unwrap();
    assert!(lock.try_lock().is_err(), "host must own the deduplication lock");
    drop(first);
    drop(second);
    for _ in 0..200 {
        if lock.try_lock().is_ok() {
            assert!(registry::lifecycle_lock(&f.primary).is_ok());
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("stopped maintenance host did not release ownership");
}

#[tokio::test]
async fn worktree_cleanup_flow_reconciles_interrupted_execution() {
    if !isolated("worktree_cleanup_flow_reconciles_interrupted_execution") {
        return;
    }
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    f.checkout("new").await;
    let mut report = refresh(&f.scope, &f.primary, &f.settings).await.unwrap();
    report.running = Some(old.id.clone());
    crate::storage::write_atomic(&["worktree-cleanup", &registry::key(&f.scope)], &report)
        .await
        .unwrap();
    let recovered = cleanup::flow_status(&f.scope).await.unwrap();
    assert!(recovered.running.is_none());
    assert!(recovered.errors[&old.id].contains("interrupted"));
    assert!(old.directory.exists());
    assert!(!recovered.proposal.candidates.is_empty());
}

#[tokio::test]
async fn worktree_cleanup_flow_cancelled_host_does_not_start_deletion() {
    if !isolated("worktree_cleanup_flow_cancelled_host_does_not_start_deletion") {
        return;
    }
    let mut f = Fixture::new().await;
    let old = f.checkout("old").await;
    f.checkout("new").await;
    f.settings.ask_before_delete = false;
    crate::config::update_global_for_tenant(
        f.scope.tenant_scope(),
        json!({"worktree_settings": f.settings}),
    )
    .await
    .unwrap();
    let stop = tokio_util::sync::CancellationToken::new();
    stop.cancel();
    let report = cleanup::refresh(&f.scope, &f.primary, &f.settings, &stop).await.unwrap();
    assert!(report.deleted.is_empty());
    assert!(report.running.is_none());
    assert!(old.directory.exists());
    assert!(recovery::list(&f.scope, &f.primary).await.unwrap().is_empty());
}

struct LeaseProbeProvider(registry::Record);
#[async_trait::async_trait]
impl crate::providers::Provider for LeaseProbeProvider {
    async fn chat_with_system(
        &self,
        _system: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        assert!(
            registry::ensure_not_in_use(&self.0).is_err(),
            "legacy execution must hold its workspace lease"
        );
        Ok("done".into())
    }
}
async fn legacy_turn(record: &registry::Record) -> anyhow::Result<String> {
    let provider = LeaseProbeProvider(record.clone());
    let policy = std::sync::Arc::new(crate::security::SecurityPolicy {
        workspace_dir: record.directory.clone(),
        ..Default::default()
    });
    let mut history = vec![crate::providers::ChatMessage::user("hello")];
    crate::agent::loop_::core::run_tool_call_loop(
        &provider,
        &mut history,
        &[],
        &crate::observability::NoopObserver,
        "fixture",
        "fixture",
        0.0,
        true,
        None,
        "cli",
        &crate::config::MultimodalConfig::default(),
        1,
        None,
        None,
        None,
        Some(policy),
        &[],
    )
    .await
}
#[tokio::test]
async fn worktree_cleanup_flow_legacy_execution_cannot_cross_removal() {
    if !isolated("worktree_cleanup_flow_legacy_execution_cannot_cross_removal") {
        return;
    }
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    let removal = registry::lifecycle_lock(&f.primary).unwrap();
    let error = legacy_turn(&old).await.unwrap_err();
    assert!(error.to_string().contains("Worktree lifecycle is busy"), "{error}");
    drop(removal);
    assert_eq!(legacy_turn(&old).await.unwrap(), "done");
    assert!(registry::ensure_not_in_use(&old).is_ok());
}

#[tokio::test]
async fn worktree_cleanup_flow_binding_retains_lease_until_cleared() {
    if !isolated("worktree_cleanup_flow_binding_retains_lease_until_cleared") {
        return;
    }
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    let context = crate::tools::ToolUseContext::new(
        "binding",
        Some(f.primary.to_string_lossy().into_owned()),
    );
    context
        .bind_worktree(old.directory.to_string_lossy().into_owned(), "old".into(), "old".into())
        .unwrap();
    assert!(
        registry::ensure_not_in_use(&old).is_err(),
        "a bound worktree must remain protected between tool calls"
    );
    let metadata = context.workflow_state();
    let other = f.checkout("other").await;
    assert!(
        context
            .bind_worktree(
                other.directory.to_string_lossy().into_owned(),
                "other".into(),
                "other".into()
            )
            .is_err()
    );
    assert!(registry::ensure_not_in_use(&old).is_err());
    assert!(registry::ensure_not_in_use(&other).is_ok());
    context.clear_worktree_binding();
    assert!(
        registry::ensure_not_in_use(&old).is_ok(),
        "metadata snapshots must not retain live leases"
    );
    assert_eq!(metadata.worktree.directory.as_deref(), old.directory.to_str());
    assert_eq!(serde_json::to_value(metadata).unwrap().as_object().unwrap().len(), 1);
}

#[tokio::test]
async fn worktree_cleanup_flow_binding_rejects_removal_without_changing_context() {
    if !isolated("worktree_cleanup_flow_binding_rejects_removal_without_changing_context") {
        return;
    }
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    let context = crate::tools::ToolUseContext::new(
        "binding",
        Some(f.primary.to_string_lossy().into_owned()),
    );
    let removal = registry::lifecycle_lock(&f.primary).unwrap();
    assert!(
        context
            .bind_worktree(old.directory.to_string_lossy().into_owned(), "old".into(), "old".into())
            .is_err(),
        "binding must not cross an in-flight removal"
    );
    assert!(context.worktree_binding_state().directory.is_none());
    drop(removal);
    assert!(registry::ensure_not_in_use(&old).is_ok());
}

#[tokio::test]
async fn worktree_cleanup_flow_status_is_queryable_during_removal() {
    if !isolated("worktree_cleanup_flow_status_is_queryable_during_removal") {
        return;
    }
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    f.checkout("new").await;
    let mut report = refresh(&f.scope, &f.primary, &f.settings).await.unwrap();
    let root = vw_config_types::paths::home_config_dir(&crate::global::paths().home)
        .join("storage/worktree-cleanup");
    let pass = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join(format!("{}.lock", registry::key(&f.scope))))
        .unwrap();
    pass.try_lock().unwrap();
    report.running = Some(old.id.clone());
    crate::storage::write_atomic(&["worktree-cleanup", &registry::key(&f.scope)], &report)
        .await
        .unwrap();
    let removal = registry::lifecycle_lock(&f.primary).unwrap();
    let (code, body) = f.call("GET", "worktree-cleanup-status", json!({})).await;
    assert_eq!(
        code,
        StatusCode::OK,
        "in-flight status query must not acquire the removal lock: {body}"
    );
    assert_eq!(body["running"], old.id);
    assert_eq!(body["proposal"]["candidates"][0]["id"], old.id);
    assert!(old.directory.exists());
    drop(removal);
    drop(pass);
}

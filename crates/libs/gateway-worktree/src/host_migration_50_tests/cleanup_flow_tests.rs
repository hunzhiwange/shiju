use super::{cleanup, management, recovery, registry};
use axum::{
    Extension,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tower::ServiceExt;
use {
    vw_api_transport::id::ProjectScope,
    vw_api_transport::id::TenantScope,
    vw_api_worktree::DeleteManagedWorktreeRequest,
};
use config_worktree::WorktreeSettings;

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
        let (project, _) = agent_project::from_directory(&tenant, &primary).await.unwrap();
        let scope = ProjectScope::new(tenant.clone(), project.id.into()).unwrap();
        let settings = WorktreeSettings {
            keep_count: 1,
            root: temp.path().join("restored").to_string_lossy().into_owned(),
            ..Default::default()
        };
        agent_config::runtime::update_global_for_tenant(&tenant, json!({"worktree_settings": settings}))
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
        let app = crate::project_router::<()>()
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
    super::initialize_host();
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
async fn worktree_cleanup_flow_restore_is_visible() {
    super::initialize_host();
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
    super::initialize_host();
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
    agent_config::runtime::update_global_for_tenant(
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
async fn worktree_cleanup_flow_rejects_settings_changes_and_stale_candidates() {
    super::initialize_host();
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
    agent_config::runtime::update_global_for_tenant(
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

#[tokio::test]
async fn worktree_cleanup_flow_status_is_queryable_during_removal() {
    super::initialize_host();
    if !isolated("worktree_cleanup_flow_status_is_queryable_during_removal") {
        return;
    }
    let f = Fixture::new().await;
    let old = f.checkout("old").await;
    f.checkout("new").await;
    let mut report = refresh(&f.scope, &f.primary, &f.settings).await.unwrap();
    let root = config_runtime_paths::home_config_dir(&agent_paths::paths().home)
        .join("storage/worktree-cleanup");
    let pass = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join(format!("{}.lock", registry::key(&f.scope))))
        .unwrap();
    pass.try_lock().unwrap();
    report.running = Some(old.id.clone());
    agent_storage::write_atomic(&["worktree-cleanup", &registry::key(&f.scope)], &report)
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

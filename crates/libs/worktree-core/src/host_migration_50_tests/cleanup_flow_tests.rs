use super::{cleanup, management, recovery, registry};
use serde_json::json;
use std::path::{Path, PathBuf};
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
}

#[tokio::test]
async fn worktree_cleanup_flow_rechecks_newly_busy_checkout() {
    super::initialize_host();
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
async fn worktree_cleanup_flow_snapshot_failure_keeps_checkout_and_can_retry() {
    super::initialize_host();
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
async fn worktree_cleanup_flow_host_refreshes_deduplicates_and_stops() {
    super::initialize_host();
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
    let path = config_runtime_paths::home_config_dir(&crate::global::paths().home)
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
    super::initialize_host();
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
    super::initialize_host();
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

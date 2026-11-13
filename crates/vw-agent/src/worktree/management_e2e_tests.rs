use super::{recovery, registry};
use axum::{
    Extension,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tower::ServiceExt;
use vw_api_types::id::{ProjectScope, TenantScope};
use vw_config_types::worktree::WorktreeSettings;

// A process owns all lazy stores and environment variables. Never mutate a live
// test runner's environment or share tenant databases with a parallel fixture.
fn isolated(test: &str) -> bool {
    const CHILD: &str = "SHIJU_WORKTREE_E2E_CHILD";
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
        let tenant = TenantScope::default();
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
async fn worktree_management_e2e_create_chat_delete_restore() {
    if !isolated("worktree_management_e2e_create_chat_delete_restore") {
        return;
    }
    let mut f = Fixture::new().await;
    f.settings.fetch_before_create = true;
    let remote = f.temp.path().join("remote.git");
    git(f.temp.path(), &["init", "--bare", remote.to_str().unwrap()]);
    git(&f.primary, &["remote", "add", "origin", remote.to_str().unwrap()]);
    git(&f.primary, &["push", "-u", "origin", "main"]);
    crate::config::update_global_for_tenant(
        f.scope.tenant_scope(),
        json!({"worktree_settings": f.settings}),
    )
    .await
    .unwrap();
    let loaded = crate::config::get_global_for_tenant(f.scope.tenant_scope()).await.unwrap();
    assert_eq!(loaded.worktree_settings, f.settings);
    crate::project::update(
        &f.scope,
        crate::project::UpdateInput {
            project_id: f.scope.project_id().as_ref().into(),
            name: None,
            icon: None,
            commands: Some(crate::project::CommandsUpdate {
                start: Some(Some("echo configured > project-setup".into())),
            }),
        },
    )
    .await
    .unwrap();
    let project = crate::project::get(&f.scope).await.unwrap().unwrap();
    let root = super::creation_policy::prepare(&project, f.primary.to_str().unwrap(), &f.settings)
        .await
        .unwrap();
    let directory = root.join("中文工作树");
    git(&f.primary, &["worktree", "add", "-b", "e2e", directory.to_str().unwrap()]);
    let record = registry::register(&f.scope, &f.primary, &directory).await.unwrap();
    super::environment::prepare(directory.to_str().unwrap(), &project.id, "exit 23")
        .await
        .unwrap_err();
    assert!(super::environment::ensure_ready(directory.to_str().unwrap()).is_err());
    // A failed initializer is protected until a successful retry.
    let (_, items) = f.call("GET", "managed-worktrees", json!({})).await;
    assert_eq!(items["items"][0]["worktree"]["status"], "error");
    // Retry the same failed command must remain an error, never fake readiness.
    assert!(super::environment::retry(directory.to_str().unwrap(), &project.id).await.is_err());
    // A separate ready checkout exercises the confirmed deletion and durable restore path.
    let old = f.checkout("ready").await;
    super::environment::prepare(
        old.directory.to_str().unwrap(),
        &project.id,
        "echo ready > initialized",
    )
    .await
    .unwrap();
    super::environment::ensure_ready(old.directory.to_str().unwrap()).unwrap();
    assert_eq!(
        std::fs::read_to_string(old.directory.join("project-setup")).unwrap().trim(),
        "configured"
    );
    let (code, _) =
        f.call("POST", &format!("managed-worktrees/{}/record-use", old.id), json!({})).await;
    assert_eq!(code, StatusCode::OK);

    for id in ["first", "second"] {
        let session: crate::session::session::Info = serde_json::from_value(json!({
            "id":id,"slug":id,"projectID":f.scope.project_id().as_ref(),
            "directory":old.directory,"title":format!("中文聊天-{id}"),"version":"test",
            "time":{"created":1,"updated":2}
        }))
        .unwrap();
        crate::session::ui_store::save_agent_session_scoped_for_tenant(
            f.scope.tenant_scope(),
            &session,
            Some(f.scope.project_id().as_ref()),
        )
        .unwrap();
    }
    let (_, items) = f.call("GET", "managed-worktrees", json!({})).await;
    let item =
        items["items"].as_array().unwrap().iter().find(|i| i["worktree"]["id"] == old.id).unwrap();
    assert_eq!(item["sessions"].as_array().unwrap().len(), 2);
    crate::session::ui_store::set_thread_pinned(
        f.scope.tenant_scope(),
        f.scope.project_id().as_ref(),
        "first",
        true,
    )
    .unwrap();
    let (_, protected) = f
        .call("POST", &format!("managed-worktrees/{}/deletion-preflight", old.id), json!({}))
        .await;
    assert!(!protected["protection_reasons"].as_array().unwrap().is_empty());
    crate::session::ui_store::set_thread_pinned(
        f.scope.tenant_scope(),
        f.scope.project_id().as_ref(),
        "first",
        false,
    )
    .unwrap();
    std::fs::write(old.directory.join("中文二进制"), [0, 255, 10]).unwrap();
    let target = format!("managed-worktrees/{}", old.id);
    let (code, preflight) =
        f.call("POST", &format!("{target}/deletion-preflight"), json!({})).await;
    assert_eq!(code, StatusCode::OK, "{preflight}");
    let (code, _) = f
        .call("DELETE", &target, json!({"fingerprint":preflight["fingerprint"],"confirmed":false}))
        .await;
    assert!(!code.is_success());
    assert!(old.directory.exists());
    let (code, deleted) = f
        .call("DELETE", &target, json!({"fingerprint":preflight["fingerprint"],"confirmed":true}))
        .await;
    assert_eq!(code, StatusCode::OK, "{deleted}");
    assert!(!old.directory.exists());
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "worktree::worktree::management_e2e_tests::worktree_management_e2e_restart_restore",
            "--ignored",
            "--nocapture",
        ])
        .env("SHIJU_E2E_RESTORE_SCOPE", serde_json::to_string(&f.scope).unwrap())
        .env("SHIJU_E2E_RESTORE_PRIMARY", &f.primary)
        .env("SHIJU_E2E_RESTORE_ID", deleted["snapshot_id"].as_str().unwrap())
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{} {}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    assert!(String::from_utf8_lossy(&child.stdout).contains("1 passed"));
    assert!(record.directory.exists());
}

#[tokio::test]
#[ignore = "invoked by the parent with its persisted repository and tenant"]
async fn worktree_management_e2e_restart_restore() {
    let Ok(scope) = std::env::var("SHIJU_E2E_RESTORE_SCOPE") else {
        return;
    };
    let scope: ProjectScope = serde_json::from_str(&scope).unwrap();
    let primary = PathBuf::from(std::env::var_os("SHIJU_E2E_RESTORE_PRIMARY").unwrap());
    let id = std::env::var("SHIJU_E2E_RESTORE_ID").unwrap();
    let restored = recovery::restore(&scope, &primary, &id).await.unwrap();
    assert_eq!(std::fs::read(restored.record.directory.join("中文二进制")).unwrap(), [0, 255, 10]);
    assert!(registry::authorize(&scope, &primary, &restored.record.id).await.is_ok());
    assert!(recovery::restore(&scope, &primary, &id).await.is_err());
}

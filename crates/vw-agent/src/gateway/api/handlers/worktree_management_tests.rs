use super::*;
use crate::worktree::registry;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::path::Path as FsPath;
use tower::ServiceExt;
fn git(path: &FsPath, args: &[&str]) {
    let output = std::process::Command::new("git").arg("-C").arg(path).args(args).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
async fn call(
    tenant: Option<TenantScope>,
    method: &str,
    path: &str,
    body: Value,
) -> (StatusCode, Value) {
    let app = super::super::project::router::<()>();
    let app = if let Some(tenant) = tenant { app.layer(Extension(tenant)) } else { app };
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
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
#[tokio::test]
async fn worktree_management_rejects_cross_scope_operations() {
    for (method, path) in [
        ("GET", "/projects/unknown/managed-worktrees"),
        ("DELETE", "/projects/unknown/managed-worktrees/tree"),
        ("POST", "/projects/unknown/worktree-snapshots/snapshot/restore"),
    ] {
        let (status, _) = call(None, method, path, json!({"fingerprint":"x"})).await;
        assert!(!status.is_success());
    }
}
#[tokio::test]
async fn worktree_management_deletion_requires_valid_confirmation() {
    let temp = tempfile::tempdir().unwrap();
    let primary = temp.path().join("primary");
    std::fs::create_dir(&primary).unwrap();
    git(&primary, &["init", "-b", "main"]);
    std::fs::write(primary.join("tracked"), "base").unwrap();
    git(&primary, &["add", "."]);
    git(
        &primary,
        &["-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-m", "init"],
    );
    let secondary = temp.path().join("secondary");
    git(&primary, &["worktree", "add", "-b", "feature", secondary.to_str().unwrap()]);
    let tenant = TenantScope::default();
    let (project, _) = crate::project::from_directory(&tenant, &primary).await.unwrap();
    let scope = project_scope(tenant.clone(), project.id.clone()).unwrap();
    crate::config::update_global_for_tenant(
        &tenant,
        json!({"worktree_settings":{"root":temp.path().join("restored"),"ask_before_delete":true}}),
    )
    .await
    .unwrap();
    let record = registry::register(&scope, &primary, &secondary).await.unwrap();
    let base = format!("/projects/{}/managed-worktrees", project.id);
    let target = format!("{base}/{}", record.id);
    let preflight = format!("{target}/deletion-preflight");
    let request = |fingerprint: &Value, confirmed: bool, ignored: bool| json!({"fingerprint":fingerprint,"confirmed":confirmed,"confirm_ignored_loss":ignored});
    let (status, list) = call(Some(tenant.clone()), "GET", &base, json!({})).await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_eq!(list["items"][0]["worktree"]["id"], record.id);
    assert_eq!(list["items"][0]["worktree"]["created_at_ms"], record.created_at_ms);
    // Same registry ID and project ID never authorize another tenant or another project.
    let foreign = TenantScope::new("foreign", "tenant").unwrap();
    for (method, path) in [
        ("GET", base.clone()),
        ("POST", preflight.clone()),
        ("DELETE", target.clone()),
        ("POST", format!("{target}/retry-initialization")),
    ] {
        assert!(
            !call(Some(foreign.clone()), method, &path, request(&json!("x"), true, true))
                .await
                .0
                .is_success()
        );
    }
    let mut other_project = project.clone();
    other_project.id = uuid::Uuid::new_v4().to_string();
    crate::project::store_local_test_project(&other_project).unwrap();
    let other = format!(
        "/projects/{}/managed-worktrees/{}/deletion-preflight",
        other_project.id, record.id
    );
    assert!(!call(Some(tenant.clone()), "POST", &other, json!({})).await.0.is_success());
    // Encoded filesystem paths and primary paths are not managed IDs.
    for id in ["external", "primary"] {
        assert!(
            !call(
                Some(tenant.clone()),
                "POST",
                &format!("{base}/{id}/deletion-preflight"),
                json!({})
            )
            .await
            .0
            .is_success()
        );
    }
    let (status, p) = call(Some(tenant.clone()), "POST", &preflight, json!({})).await;
    assert_eq!(status, StatusCode::OK, "{p}");
    assert_eq!(p["protection_reasons"], json!([]));
    assert!(
        !call(Some(tenant.clone()), "DELETE", &target, request(&p["fingerprint"], false, false))
            .await
            .0
            .is_success()
    );
    assert!(
        !call(Some(tenant.clone()), "DELETE", &target, request(&json!("1:expired"), true, true))
            .await
            .0
            .is_success()
    );
    std::fs::write(secondary.join("untracked"), "new contents").unwrap();
    assert!(
        !call(Some(tenant.clone()), "DELETE", &target, request(&p["fingerprint"], true, true))
            .await
            .0
            .is_success()
    );
    assert!(secondary.exists());
    // Error and Creating come from checkout initialization metadata; retry uses saved commands.
    let environment = record.git_dir.join("shiju-environment.json");
    for (state, expected) in [("Pending", "creating"), ("Running", "creating"), ("Error", "error")]
    {
        std::fs::write(
            &environment,
            json!({"status":state,"project_done":false,"extra_done":false,"extra_command":""})
                .to_string(),
        )
        .unwrap();
        let (_, list) = call(Some(tenant.clone()), "GET", &base, json!({})).await;
        assert_eq!(list["items"][0]["worktree"]["status"], expected);
        if state == "Running" {
            let (_, p) = call(Some(tenant.clone()), "POST", &preflight, json!({})).await;
            assert!(
                !call(
                    Some(tenant.clone()),
                    "DELETE",
                    &target,
                    request(&p["fingerprint"], true, true)
                )
                .await
                .0
                .is_success()
            );
        }
    }
    let (status, body) =
        call(Some(tenant.clone()), "POST", &format!("{target}/retry-initialization"), json!({}))
            .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // A deterministic ref publication failure must leave the checkout intact.
    let barrier = primary.join(".git/refs/shiju-worktree-recovery");
    std::fs::write(&barrier, "disk boundary").unwrap();
    let (_, p) = call(Some(tenant.clone()), "POST", &preflight, json!({})).await;
    assert!(
        !call(Some(tenant.clone()), "DELETE", &target, request(&p["fingerprint"], true, true))
            .await
            .0
            .is_success()
    );
    assert!(secondary.join("untracked").exists());
    std::fs::remove_file(barrier).unwrap();
    std::fs::write(primary.join(".git/info/exclude"), "ignored\n").unwrap();
    std::fs::write(secondary.join("ignored"), "excluded").unwrap();
    let (_, p) = call(Some(tenant.clone()), "POST", &preflight, json!({})).await;
    assert_eq!(p["has_ignored_files"], true);
    assert!(
        !call(Some(tenant.clone()), "DELETE", &target, request(&p["fingerprint"], true, false))
            .await
            .0
            .is_success()
    );
    let (status, body) =
        call(Some(tenant.clone()), "DELETE", &target, request(&p["fingerprint"], true, true)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(!secondary.exists());
    assert!(
        git2::Repository::open(&primary)
            .unwrap()
            .find_branch("feature", git2::BranchType::Local)
            .is_ok()
    );
    let id = body["snapshot_id"].as_str().unwrap();
    let snapshots = format!("/projects/{}/worktree-snapshots", project.id);
    let (_, list) = call(Some(tenant.clone()), "GET", &snapshots, json!({})).await;
    assert_eq!(list["items"][0]["id"], id);
    let (status, restored) =
        call(Some(tenant.clone()), "POST", &format!("{snapshots}/{id}/restore"), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{restored}");
    let path = FsPath::new(restored["worktree"]["worktree"]["directory"].as_str().unwrap());
    assert_eq!(std::fs::read(path.join("untracked")).unwrap(), b"new contents");
    assert!(!path.join("ignored").exists());
}

#[tokio::test]
async fn worktree_managed_ui_records_ready_checkout_use_in_scope() {
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
    let checkout = temp.path().join("checkout");
    git(&primary, &["worktree", "add", "-b", "existing", checkout.to_str().unwrap()]);
    let tenant = TenantScope::default();
    let (project, _) = crate::project::from_directory(&tenant, &primary).await.unwrap();
    let scope = project_scope(tenant.clone(), project.id.clone()).unwrap();
    let record = registry::register(&scope, &primary, &checkout).await.unwrap();
    let target = format!("/projects/{}/managed-worktrees/{}/record-use", project.id, record.id);
    for other in [None, Some(TenantScope::new("foreign", "tenant").unwrap())] {
        assert!(!call(other, "POST", &target, json!({})).await.0.is_success());
    }
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let (status, _) = call(Some(tenant.clone()), "POST", &target, json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let updated = registry::authorize(&scope, &primary, &record.id).await.unwrap();
    assert!(updated.last_used_at_ms > record.last_used_at_ms);
    assert_eq!(updated.directory, record.directory);
    assert_eq!(git2::Repository::open(&primary).unwrap().worktrees().unwrap().len(), 1);
    assert!(
        !call(Some(tenant), "POST", &target.replace(&record.id, "unknown"), json!({}))
            .await
            .0
            .is_success()
    );
}

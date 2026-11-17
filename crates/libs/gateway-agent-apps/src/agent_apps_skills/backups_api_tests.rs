use agent_database::GlobalDatabase;
use axum::{
    Extension,
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt};
use tower::ServiceExt;
use vw_api_transport::id::TenantScope;

// Runtime paths are cached globally; each test needs its own process and home.
fn isolated_test() -> bool {
    const CHILD: &str = "SHIJU_SKILL_BACKUPS_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return false;
    }
    let root = tempfile::tempdir().unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", std::thread::current().name().unwrap(), "--nocapture"])
        .env(CHILD, "1")
        .env("VIBEWINDOW_TEST_ROOT", root.path().canonicalize().unwrap())
        .status()
        .unwrap();
    assert!(status.success(), "isolated skill backups child failed: {status}");
    true
}

async fn call(project_id: &str, company_id: &str) -> StatusCode {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let ssot = agent_paths::paths().home.join(".agents/skills");
    fs::create_dir_all(&ssot).unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let source = root.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("SKILL.md"), "---\nname: Tool\ndescription: Example\n---\nContent")
        .unwrap();
    let store =
        app_settings_store::agent_apps_skills_store::SkillInstallsStore::new(db.clone()).unwrap();
    let project =
        vw_api_transport::ProjectScope::new(TenantScope::new("p", "c").unwrap(), "project".into())
            .unwrap();
    store.install_local(&project, local_app_auth::HOST_ID, 0, &ssot, &source, "tool").unwrap();
    let backups =
        super::backups_api::backup_root(&ssot, &project, local_app_auth::HOST_ID).unwrap();
    fs::create_dir_all(&backups).unwrap();
    store
        .uninstall_skill(
            &project,
            local_app_auth::HOST_ID,
            1,
            &ssot,
            &backups,
            "tool",
            &std::collections::BTreeMap::new(),
        )
        .unwrap();
    let id =
        vw_agent_apps::skills::backup_catalog::list_backups(&backups).unwrap()[0].backup_id.clone();
    // A valid empty destination catalog makes a missing scoped backup a 404.
    let requested_project = vw_api_transport::ProjectScope::new(
        TenantScope::new("p", company_id).unwrap(),
        project_id.into(),
    )
    .unwrap();
    let requested_backups =
        super::backups_api::backup_root(&ssot, &requested_project, local_app_auth::HOST_ID)
            .unwrap();
    fs::create_dir_all(requested_backups).unwrap();
    let app = super::backups_api::router_with_policy(None)
        .layer(Extension(TenantScope::new("p", company_id).unwrap()))
        .layer(Extension(db));
    let response = app
        .oneshot(
            Request::post(format!("/agent-apps/skills/backups/{id}/restore"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"scope":{"projectId":project_id,"targetHostId":local_app_auth::HOST_ID},
            "expectedVersion":2,"apps":[]})
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    assert!(backups.join(&id).join("skill/SKILL.md").is_file());
    assert_eq!(ssot.join("tool").exists(), status == StatusCode::OK);
    assert_eq!(
        store.get(&project, local_app_auth::HOST_ID, &ssot).unwrap().version,
        if status == StatusCode::OK { 3 } else { 2 }
    );
    status
}
#[tokio::test]
async fn cp12_backups_api_success() {
    if isolated_test() {
        return;
    }
    assert_eq!(call("project", "c").await, StatusCode::OK);
}
#[tokio::test]
async fn cp12_backups_api_rejects_without_side_effects() {
    if isolated_test() {
        return;
    }
    // Backups are addressed within the complete tenant/project/host scope.
    // Gateway pairing authenticates callers; legacy per-app grants are ignored.
    assert_eq!(call("other", "c").await, StatusCode::NOT_FOUND);
    assert_eq!(call("project", "other-company").await, StatusCode::NOT_FOUND);
}

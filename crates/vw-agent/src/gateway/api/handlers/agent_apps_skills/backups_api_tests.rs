use axum::{Extension, body::Body, http::{Request, StatusCode}};
use serde_json::{json, Value};
use tower::ServiceExt;
use vw_api_types::id::TenantScope;
use crate::global_database::GlobalDatabase;
use std::{fs, os::unix::fs::PermissionsExt};
use sha2::{Digest, Sha256};

async fn call(allowed: bool, project_id: &str) -> StatusCode {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let ssot = root.path().join("ssot");
    fs::create_dir(&ssot).unwrap();
    let policy = root.path().join("access.json");
    fs::write(&policy, json!({"principals":[{"id":"reader",
        "token_sha256":hex::encode(Sha256::digest(b"test-token")),"grants":[{
            "platform_id":"p","company_id":"c","host_id":"h",
            "skill_inventory_read_projects":if allowed { vec!["project"] } else { vec![] },
            "skill_install_projects":if allowed { vec!["project"] } else { vec![] }}]}],
        "hosts":[{"id":"h","online":true,"binding":"local","home":root.path(),
            "applications":root.path(),"search_paths":[],"skills_root":ssot}]}).to_string()).unwrap();
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let source = root.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("SKILL.md"), "---\nname: Tool\ndescription: Example\n---\nContent").unwrap();
    let store = crate::gateway::agent_apps_skills_store::SkillInstallsStore::new(db.clone()).unwrap();
    let project = vw_api_types::id::ProjectScope::new(TenantScope::new("p", "c").unwrap(), "project".into()).unwrap();
    store.install_local(&project, "h", 0, &ssot, &source, "tool").unwrap();
    let backups = super::backups_api::backup_root(&ssot, &project, "h").unwrap();
    fs::create_dir(&backups).unwrap();
    store.uninstall_skill(&project, "h", 1, &ssot, &backups, "tool", &std::collections::BTreeMap::new()).unwrap();
    let id = vw_agent_apps::skills::backup_catalog::list_backups(&backups).unwrap()[0].backup_id.clone();
    let app = super::backups_api::router_with_policy(Some(policy))
        .layer(Extension(TenantScope::new("p", "c").unwrap())).layer(Extension(db));
    let response = app.oneshot(Request::post(format!("/agent-apps/skills/backups/{id}/restore"))
        .header("content-type", "application/json").header("x-agent-apps-token", "test-token")
        .body(Body::from(json!({"scope":{"projectId":project_id,"targetHostId":"h"},
            "expectedVersion":2,"apps":[]}).to_string())).unwrap()).await.unwrap();
    let status = response.status();
    assert!(backups.join(&id).join("skill/SKILL.md").is_file());
    assert_eq!(ssot.join("tool").exists(), status == StatusCode::OK);
    assert_eq!(store.get(&project, "h", &ssot).unwrap().version, if status == StatusCode::OK { 3 } else { 2 });
    status
}
#[tokio::test]
async fn cp12_backups_api_success() {
    assert_eq!(call(true, "project").await, StatusCode::OK);
}
#[tokio::test]
async fn cp12_backups_api_rejects_without_side_effects() {
    assert_eq!(call(false, "project").await, StatusCode::FORBIDDEN);
    assert_eq!(call(true, "other").await, StatusCode::FORBIDDEN);
}

use axum::{Extension, body::Body, http::{Request, StatusCode}};
use serde_json::{json, Value};
use tower::ServiceExt;
use vw_api_types::id::TenantScope;
use crate::global_database::GlobalDatabase;
use std::{fs, os::unix::fs::PermissionsExt};
use sha2::{Digest, Sha256};

async fn call(allowed: bool, path: &str, body: Value) -> (StatusCode, Value) {
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
    let app = super::uninstall_api::router_with_policy(Some(policy))
        .layer(Extension(TenantScope::new("p", "c").unwrap())).layer(Extension(db));
    let response = app.oneshot(Request::delete(path).header("content-type", "application/json")
        .header("x-agent-apps-token", "test-token").body(Body::from(body.to_string())).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 16384).await.unwrap();
    assert_eq!(ssot.join("tool").exists(), status != StatusCode::OK);
    if status != StatusCode::OK {
        assert_eq!(store.get(&project, "h", &ssot).unwrap().version, 1);
        assert!(!super::backups_api::backup_root(&ssot, &project, "h").unwrap().exists());
    } else {
        let result: Value = serde_json::from_slice(&bytes).unwrap();
        assert!(std::path::Path::new(result["backupPath"].as_str().unwrap()).join("skill/SKILL.md").is_file());
    }
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn cp12_uninstall_api_success() {
    let (status, result) = call(true, "/agent-apps/skills/tool",
        json!({"scope":{"projectId":"project","targetHostId":"h"},"expectedVersion":1})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["version"], 2);
    assert_eq!(result["cleanupIncomplete"], false);
    assert_eq!(result["preservedPaths"], json!([]));
}

#[tokio::test]
async fn cp12_uninstall_api_rejects_without_side_effects() {
    let request = json!({"scope":{"projectId":"project","targetHostId":"h"},"expectedVersion":1});
    assert_eq!(call(false, "/agent-apps/skills/tool", request.clone()).await.0, StatusCode::FORBIDDEN);
    assert_eq!(call(true, "/agent-apps/skills/missing", request.clone()).await.0, StatusCode::NOT_FOUND);
    let mut stale = request.clone(); stale["expectedVersion"] = json!(0);
    assert_eq!(call(true, "/agent-apps/skills/tool", stale).await.0, StatusCode::CONFLICT);
    let mut invalid = request; invalid["backupPath"] = json!("/tmp/client-path");
    assert_eq!(call(true, "/agent-apps/skills/tool", invalid).await.0, StatusCode::UNPROCESSABLE_ENTITY);
}

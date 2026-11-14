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
    let app = super::updates_api::router_with_policy(Some(policy))
        .layer(Extension(TenantScope::new("p", "c").unwrap())).layer(Extension(db));
    let response = app.oneshot(Request::post(path).header("content-type", "application/json")
        .header("x-agent-apps-token", "test-token").body(Body::from(body.to_string())).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 16384).await.unwrap();
    assert_eq!(fs::read_dir(ssot).unwrap().count(), 0);
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn cp12_updates_api_success() {
    let (status, response) = call(true, "/agent-apps/skills/check-updates",
        json!({"scope":{"projectId":"project","targetHostId":"h"}})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["updates"], json!([]));
    assert_eq!(response["version"], 0);
    assert_eq!(response["scope"]["targetHostId"], "h");
}

#[tokio::test]
async fn cp12_updates_api_rejects_without_side_effects() {
    let scope = json!({"projectId":"project","targetHostId":"h"});
    assert_eq!(call(false, "/agent-apps/skills/check-updates", json!({"scope":scope})).await.0, StatusCode::FORBIDDEN);
    assert_eq!(call(false, "/agent-apps/skills/tool/update", json!({"scope":scope,"expectedVersion":0,
        "expectedHash":"a".repeat(64)})).await.0, StatusCode::FORBIDDEN);
    assert_eq!(call(true, "/agent-apps/skills/tool/update", json!({"scope":scope,"expectedVersion":0,
        "expectedHash":"a".repeat(64),"url":"https://untrusted.invalid"})).await.0, StatusCode::UNPROCESSABLE_ENTITY);
}

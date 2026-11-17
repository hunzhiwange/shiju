use axum::{Extension, body::Body, http::{Request, StatusCode}};
use serde_json::{json, Value};
use tower::ServiceExt;
use vw_api_transport::id::TenantScope;
use agent_database::GlobalDatabase;
use std::{fs, os::unix::fs::PermissionsExt};
// Paths are cached globally; initialize each test in a fresh isolated process.
fn isolated_test() -> bool {
    const CHILD: &str = "SHIJU_UNINSTALL_API_CHILD";
    if std::env::var_os(CHILD).is_some() { return false; }
    let root = tempfile::tempdir().unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", std::thread::current().name().unwrap(), "--nocapture"])
        .env(CHILD, "1")
        .env("VIBEWINDOW_TEST_ROOT", root.path().canonicalize().unwrap())
        .status().unwrap();
    assert!(status.success(), "isolated uninstall child failed: {status}");
    true
}

async fn call(allowed: bool, path: &str, body: Value) -> (StatusCode, Value) {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let ssot = agent_paths::paths().home.join(".agents/skills");
    fs::create_dir_all(&ssot).unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let source = root.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("SKILL.md"), "---\nname: Tool\ndescription: Example\n---\nContent").unwrap();
    let store = app_settings_store::agent_apps_skills_store::SkillInstallsStore::new(db.clone()).unwrap();
    let project = vw_api_transport::ProjectScope::new(TenantScope::new("p", "c").unwrap(), "project".into()).unwrap();
    store.install_local(&project, "h", 0, &ssot, &source, "tool").unwrap();
    let app = super::uninstall_api::router_with_policy(None)
        .layer(Extension(TenantScope::new("p", "c").unwrap())).layer(Extension(db))
        .layer(axum::middleware::from_fn(|request, next| async move {
            agent_gateway::middleware::basic_auth_middleware_with_credentials(
                request, next, Some("test-password"), "test-user").await
        }));
    let response = app.oneshot(Request::delete(path).header("content-type", "application/json")
        .header("authorization", if allowed { "Basic dGVzdC11c2VyOnRlc3QtcGFzc3dvcmQ=" } else { "Basic invalid" }).body(Body::from(body.to_string())).unwrap()).await.unwrap();
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
    if ssot.join("tool").exists() { fs::remove_dir_all(ssot.join("tool")).unwrap(); }
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn cp12_uninstall_api_success() {
    if isolated_test() { return; }
    let (status, result) = call(true, "/agent-apps/skills/tool",
        json!({"scope":{"projectId":"project","targetHostId":"h"},"expectedVersion":1})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["version"], 2);
    assert_eq!(result["cleanupIncomplete"], false);
    assert_eq!(result["preservedPaths"], json!([]));
}

#[tokio::test]
async fn cp12_uninstall_api_rejects_without_side_effects() {
    if isolated_test() { return; }
    let request = json!({"scope":{"projectId":"project","targetHostId":"h"},"expectedVersion":1});
    assert_eq!(call(false, "/agent-apps/skills/tool", request.clone()).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(true, "/agent-apps/skills/missing", request.clone()).await.0, StatusCode::NOT_FOUND);
    let mut stale = request.clone(); stale["expectedVersion"] = json!(0);
    assert_eq!(call(true, "/agent-apps/skills/tool", stale).await.0, StatusCode::CONFLICT);
    let mut invalid = request; invalid["backupPath"] = json!("/tmp/client-path");
    assert_eq!(call(true, "/agent-apps/skills/tool", invalid).await.0, StatusCode::UNPROCESSABLE_ENTITY);
}

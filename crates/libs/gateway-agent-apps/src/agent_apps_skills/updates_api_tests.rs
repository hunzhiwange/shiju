use axum::{Extension, body::Body, http::{Request, StatusCode}};
use serde_json::{json, Value};
use tower::ServiceExt;
use vw_api_transport::id::TenantScope;
use agent_database::GlobalDatabase;
use std::fs;
use base64::Engine;

// Agent paths are process-global; never resolve them against the user's home.
fn isolated_test() -> bool {
    const CHILD: &str = "SHIJU_UPDATES_API_CHILD";
    if std::env::var_os(CHILD).is_some() { return false; }
    let root = tempfile::tempdir().unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", std::thread::current().name().unwrap(), "--nocapture"])
        .env(CHILD, "1")
        .env("VIBEWINDOW_TEST_ROOT", root.path().canonicalize().unwrap())
        .status().unwrap();
    assert!(status.success(), "isolated updates API child failed: {status}");
    true
}

async fn call(allowed: bool, path: &str, body: Value) -> (StatusCode, Value) {
    let root = tempfile::tempdir().unwrap();
    let ssot = agent_paths::paths().home.join(".agents/skills");
    fs::create_dir_all(&ssot).unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let app = super::updates_api::router_with_policy(None)
        .layer(Extension(TenantScope::new("p", "c").unwrap())).layer(Extension(db))
        .layer(axum::middleware::from_fn(|request, next| async move {
            agent_gateway::middleware::basic_auth_middleware_with_credentials(
                request, next, Some("updates-test-password"), "updates-test-user").await
        }));
    let credentials = base64::engine::general_purpose::STANDARD.encode(format!(
        "updates-test-user:{}", if allowed { "updates-test-password" } else { "incorrect" }));
    let response = app.oneshot(Request::post(path).header("content-type", "application/json")
        .header("authorization", format!("Basic {credentials}")).body(Body::from(body.to_string())).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 16384).await.unwrap();
    assert_eq!(fs::read_dir(ssot).unwrap().count(), 0);
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn cp12_updates_api_success() {
    if isolated_test() { return; }
    let (status, response) = call(true, "/agent-apps/skills/check-updates",
        json!({"scope":{"projectId":"project","targetHostId":"h"}})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["updates"], json!([]));
    assert_eq!(response["version"], 0);
    assert_eq!(response["scope"]["targetHostId"], "h");
}

#[tokio::test]
async fn cp12_updates_api_rejects_without_side_effects() {
    if isolated_test() { return; }
    let scope = json!({"projectId":"project","targetHostId":"h"});
    assert_eq!(call(false, "/agent-apps/skills/check-updates", json!({"scope":scope})).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(false, "/agent-apps/skills/tool/update", json!({"scope":scope,"expectedVersion":0,
        "expectedHash":"a".repeat(64)})).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(true, "/agent-apps/skills/tool/update", json!({"scope":scope,"expectedVersion":0,
        "expectedHash":"a".repeat(64),"url":"https://untrusted.invalid"})).await.0, StatusCode::UNPROCESSABLE_ENTITY);
}

use super::preferences;
use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    middleware,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::Path};
use tower::ServiceExt;
const A: &str = "fixture-preferences-principal-a-token";
const B: &str = "fixture-preferences-principal-b-token";
fn fixture(root: &Path) -> (std::path::PathBuf, GlobalDatabase) {
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.join("policy.json");
    let grants = json!([{"platform_id":"p","company_id":"c","host_id":"h","preferences_write":true},{"platform_id":"p","company_id":"c","host_id":"other"},{"platform_id":"p","company_id":"second","host_id":"h"}]);
    let hosts=["h","other"].map(|id| json!({"id":id,"online":true,"binding":"local","home":root,"search_paths":[],"applications":root}));
    std::fs::write(&path,serde_json::to_vec(&json!({"principals":[{"id":"a","token_sha256":hex::encode(Sha256::digest(A)),"grants":grants},{"id":"b","token_sha256":hex::encode(Sha256::digest(B)),"grants":grants}],"hosts":hosts})).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    (path, GlobalDatabase::for_home(root).unwrap())
}
fn router(path: &Path, db: GlobalDatabase) -> Router {
    preferences::router_with_policy(Some(path.to_owned()))
        .layer(Extension(db))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
}
async fn call(
    app: &Router,
    token: &str,
    company: &str,
    host: &str,
    patch: Option<Value>,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(if patch.is_some() { "PATCH" } else { "GET" })
        .uri(format!("/agent-apps/preferences?targetHostId={host}"))
        .header("x-agent-apps-token", token)
        .header("x-platform-id", "p")
        .header("x-company-id", company)
        .header("content-type", "application/json");
    let response = app
        .clone()
        .oneshot(req.body(Body::from(patch.map(|p| p.to_string()).unwrap_or_default())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}
#[tokio::test]
async fn cc_apps_preferences_api_scope_persists_and_isolates() {
    let root = tempfile::tempdir().unwrap();
    let (path, db) = fixture(root.path());
    let app = router(&path, db);
    let user = json!({"scope":"user","value":{"appOrder":["codex","codex","stale"],"visibleApps":["codex"]}});
    let (status, value) = call(&app, A, "c", "h", Some(user)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["user"]["appOrder"].as_array().unwrap().len(), 14);
    assert_eq!(value["restartRequired"], false);
    let terminal = if cfg!(target_os = "macos") { "terminal" } else { "konsole" };
    let (status, value) = call(
        &app,
        A,
        "c",
        "h",
        Some(json!({"scope":"host","value":{"preferredTerminal":terminal}})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["restartRequired"], true);
    let (_, second) = call(&app, A, "second", "h", None).await;
    assert_eq!(second["user"]["visibleApps"].as_array().unwrap().len(), 14);
    assert_eq!(second["host"]["preferredTerminal"], Value::Null);
    drop(app);
    let app = router(&path, GlobalDatabase::for_home(root.path()).unwrap());
    let (_, a) = call(&app, A, "c", "h", None).await;
    assert_eq!(a["user"]["visibleApps"], json!(["codex"]));
    assert_eq!(a["host"]["preferredTerminal"], terminal);
    assert_eq!(a["restartRequired"], false);
    let (_, b) = call(&app, B, "c", "h", None).await;
    assert_eq!(b["user"]["visibleApps"].as_array().unwrap().len(), 14);
    assert_eq!(b["host"], a["host"]);
    let (_, other) = call(&app, A, "c", "other", None).await;
    assert_eq!(other["user"], a["user"]);
    assert_eq!(other["host"]["preferredTerminal"], Value::Null);
    let (_, unchanged) = call(
        &app,
        A,
        "c",
        "h",
        Some(json!({"scope":"host","value":{"preferredTerminal":terminal}})),
    )
    .await;
    assert_eq!(unchanged["restartRequired"], false);
}
#[tokio::test]
async fn cc_apps_preferences_api_scope_rejects_without_writing() {
    let root = tempfile::tempdir().unwrap();
    let (path, db) = fixture(root.path());
    let app = router(&path, db);
    for (token, company, host, patch, status) in [
        ("invalid-token-with-sufficient-length", "c", "h", None, StatusCode::UNAUTHORIZED),
        (A, "other", "h", None, StatusCode::FORBIDDEN),
        (A, "c", "unknown", None, StatusCode::FORBIDDEN),
        (A, "c", "other", Some(json!({"scope":"host","value":{}})), StatusCode::FORBIDDEN),
        (
            A,
            "c",
            "h",
            Some(json!({"scope":"user","value":{"visibleApps":[]}})),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            A,
            "c",
            "h",
            Some(json!({"scope":"host","value":{"preferredTerminal":"arbitrary-command"}})),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            A,
            "c",
            "h",
            Some(json!({"scope":"user","value":{"visibleApps":["codex"]},"userId":"b"})),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        assert_eq!(call(&app, token, company, host, patch).await.0, status);
    }
    let (_, value) = call(&app, A, "c", "h", None).await;
    assert_eq!(value["user"]["visibleApps"].as_array().unwrap().len(), 14);
    assert_eq!(value["host"]["preferredTerminal"], Value::Null);
    std::fs::write(&path, b"{}").unwrap();
    assert_eq!(call(&app, A, "c", "h", None).await.0, StatusCode::SERVICE_UNAVAILABLE);
}

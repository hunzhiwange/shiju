use super::read;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    middleware,
};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::Path};
use tower::ServiceExt;

const TOKEN: &str = "fixture-token-for-a-single-explicit-principal";
fn router(path: &Path) -> Router {
    read::router_with_policy(Some(path.to_owned()))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
}
fn request(endpoint: &str, token: &str, company: &str, host: &str) -> Request<Body> {
    Request::builder()
        .uri(format!("/agent-apps/{endpoint}?targetHostId={host}&appId=claude"))
        .header("x-agent-apps-token", token)
        .header("x-platform-id", "p")
        .header("x-company-id", company)
        .body(Body::empty())
        .unwrap()
}
fn policy(root: &Path) -> std::path::PathBuf {
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.join("access.json");
    let value = serde_json::json!({
        "principals": [{"id":"reader", "token_sha256":hex::encode(Sha256::digest(TOKEN)),
            "grants":[{"platform_id":"p","company_id":"a","host_id":"host"},
                      {"platform_id":"p","company_id":"a","host_id":"offline"},
                      {"platform_id":"p","company_id":"a","host_id":"remote"}]}],
        "hosts": [{"id":"host","online":true,"binding":"local","home":root,
                   "search_paths":[root.join("bin")],"applications":root.join("SystemApplications")},
                  {"id":"offline","online":false,"binding":"local","home":root,
                   "search_paths":[],"applications":root.join("SystemApplications")},
                  {"id":"remote","online":true,"binding":"remote","home":root,
                   "search_paths":[],"applications":root.join("SystemApplications")}]
    });
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    path
}
fn executable(root: &Path) {
    std::fs::create_dir(root.join("bin")).unwrap();
    let script = format!(
        "#!/bin/sh\nprintf probe >> '{}'\nprintf 'claude 2.3.4\\n'\n",
        root.join("probed").display()
    );
    std::fs::write(root.join("bin/claude"), script).unwrap();
    std::fs::set_permissions(root.join("bin/claude"), std::fs::Permissions::from_mode(0o700))
        .unwrap();
}
#[tokio::test]
async fn cc_apps_read_api_authorization_production_registration_and_default_deny() {
    // Process-local environment injection avoids mutating global env in parallel tests.
    if let Some(path) = std::env::var_os("SHIJU_TASK33_CHILD_POLICY") {
        assert_eq!(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE"), Some(path.clone()));
        let app = crate::gateway::router::handler_router()
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
            .layer(middleware::from_fn(|req, next| async {
                crate::gateway::middleware::basic_auth_middleware_with_credentials(
                    req,
                    next,
                    Some("secret"),
                    "test",
                )
                .await
            }));
        let req = request("versions", TOKEN, "a", "host");
        assert_eq!(app.clone().oneshot(req).await.unwrap().status(), StatusCode::UNAUTHORIZED);
        let mut req = request("versions", TOKEN, "a", "host");
        req.headers_mut().insert("authorization", "Basic dGVzdDpzZWNyZXQ=".parse().unwrap());
        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let value: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
        assert_eq!(value["installations"][0]["version"], "2.3.4");
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let path = policy(root.path());
    executable(root.path());
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "gateway::api::handlers::agent_apps::read_tests::cc_apps_read_api_authorization_production_registration_and_default_deny", "--nocapture"])
        .env("SHIJU_TASK33_CHILD_POLICY", &path).env("SHIJU_AGENT_APPS_ACCESS_FILE", &path)
        .status().unwrap();
    assert!(status.success());
    assert_eq!(std::fs::read_to_string(root.path().join("probed")).unwrap(), "probe");
    let app = read::router_with_policy(None)
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    assert_eq!(
        app.oneshot(request("catalog", TOKEN, "a", "host")).await.unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}

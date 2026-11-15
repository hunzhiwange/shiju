use super::*;
use axum::{http::{HeaderMap, StatusCode}, body::{Body, to_bytes}};
use vw_api_transport::id::TenantScope;
use std::path::PathBuf;
use tower::ServiceExt;

async fn request(path: Option<PathBuf>, tenant: TenantScope, headers: HeaderMap, section: &str, host: &str, method: &str) -> axum::response::Response {
    let router = gateway_openclaw_settings::router_with_policy(path, authorization())
        .layer(axum::Extension(tenant));
    let mut request = axum::http::Request::builder().method(method)
        .uri(format!("/agent-apps/openclaw/settings/{section}?projectId=project&targetHostId={host}"))
        .body(Body::from("{}")).unwrap();
    *request.headers_mut() = headers;
    request.headers_mut().insert("content-type", "application/json".parse().unwrap());
    router.oneshot(request).await.unwrap()
}
async fn body(response: axum::response::Response) -> serde_json::Value {
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

#[tokio::test]
async fn openclaw_api_roundtrips_defaults_denies_before_io() {
    let tenant = TenantScope::new("platform", "company").unwrap();
    for method in ["GET", "PUT"] {
        let response = request(None, tenant.clone(), HeaderMap::new(), "env", "remote", method).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn openclaw_api_protects_sensitive_env() {
    use sha2::{Digest, Sha256};
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let path = root.path().join("policy.json");
    let policy = |sensitive: bool| serde_json::json!({
        "principals":[{"id":"member", "token_sha256":hex::encode(Sha256::digest("fixture-token")),
            "grants":[{"platform_id":"platform","company_id":"company","host_id":"host",
                "openclaw_settings_read":["project"],
                "openclaw_env_sensitive_read":if sensitive { vec!["project"] } else { vec![] }}]}],
        "hosts":[{"id":"host","binding":"local","online":true,"home":home,
            "applications":home,"search_paths":[]}]
    });
    std::fs::write(&path, policy(false).to_string()).unwrap();
    let mut headers = HeaderMap::new();
    headers.insert("x-agent-apps-token", "fixture-token".parse().unwrap());
    let tenant = TenantScope::new("platform", "company").unwrap();
    let result = request(Some(path.clone()), tenant.clone(), headers.clone(), "env", "host", "GET").await;
    assert_eq!(result.status(), StatusCode::FORBIDDEN);
    assert_eq!(std::fs::read_dir(&home).unwrap().count(), 0);
    std::fs::create_dir(home.join(".openclaw")).unwrap();
    let config = home.join(".openclaw/openclaw.json");
    let original = br#"{env:{vars:{TOKEN:'secret'}},tools:{profile:'private-profile'}}"#;
    std::fs::write(&config, original).unwrap();
    let health = body(request(Some(path.clone()), tenant.clone(), headers.clone(), "health", "host", "GET").await).await;
    assert!(!health.to_string().contains("secret"));
    assert!(!health.to_string().contains("private-profile"));
    assert_eq!(std::fs::read(&config).unwrap(), original);
    std::fs::write(&path, policy(true).to_string()).unwrap();
    let env = body(request(Some(path), tenant, headers, "env", "host", "GET").await).await;
    assert_eq!(env["vars"]["TOKEN"], "secret");
}

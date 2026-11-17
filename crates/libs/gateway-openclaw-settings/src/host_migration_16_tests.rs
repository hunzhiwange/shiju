use super::*;
use axum::{http::{HeaderMap, StatusCode}, body::{Body, to_bytes}};
use vw_api_transport::id::TenantScope;
use std::path::PathBuf;
use tower::ServiceExt;

async fn request(path: Option<PathBuf>, tenant: TenantScope, headers: HeaderMap, section: &str, host: &str, method: &str) -> axum::response::Response {
    request_using(path, tenant, headers, section, host, method, authorization()).await
}

async fn request_using(path: Option<PathBuf>, tenant: TenantScope, headers: HeaderMap, section: &str, host: &str, method: &str, authorization: crate::Authorization) -> axum::response::Response {
    let router = crate::router_with_policy(path, authorization)
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
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    std::fs::create_dir(&home).unwrap();
    // Gateway owns authentication now. Inject the route's authorization boundary
    // to verify sensitive reads independently of the retired app-token policy.
    let authorization = crate::Authorization {
        settings: |path, _, project, target, write| {
            assert_eq!(project.tenant_scope(), &TenantScope::new("platform", "company").unwrap());
            assert_eq!(project.project_id().as_ref(), "project");
            assert_eq!(target, "host");
            assert!(!write);
            Ok(crate::Host {
                id: target.into(), home: path.unwrap().to_path_buf(), credential_root: None,
            })
        },
        sensitive_env: |_, headers, project, target| {
            assert_eq!(project.project_id().as_ref(), "project");
            assert_eq!(target, "host");
            if headers.contains_key("x-test-sensitive-read") { Ok(()) }
            else { Err(StatusCode::FORBIDDEN) }
        },
    };
    let mut headers = HeaderMap::new();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let result = request_using(Some(home.clone()), tenant.clone(), headers.clone(), "env", "host", "GET", authorization).await;
    assert_eq!(result.status(), StatusCode::FORBIDDEN);
    assert_eq!(std::fs::read_dir(&home).unwrap().count(), 0);
    std::fs::create_dir(home.join(".openclaw")).unwrap();
    let config = home.join(".openclaw/openclaw.json");
    let original = br#"{env:{vars:{TOKEN:'secret'}},tools:{profile:'private-profile'}}"#;
    std::fs::write(&config, original).unwrap();
    let health = body(request_using(Some(home.clone()), tenant.clone(), headers.clone(), "health", "host", "GET", authorization).await).await;
    assert!(!health.to_string().contains("secret"));
    assert!(!health.to_string().contains("private-profile"));
    assert_eq!(std::fs::read(&config).unwrap(), original);
    headers.insert("x-test-sensitive-read", "true".parse().unwrap());
    let env = body(request_using(Some(home), tenant, headers, "env", "host", "GET", authorization).await).await;
    assert_eq!(env["vars"]["TOKEN"], "secret");
}

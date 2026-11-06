#![cfg(target_os = "macos")]
use axum::{Router, middleware};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::Path};
use vw_api_types::{agent_apps::AppId, app_providers::paths::*, id::TenantScope};
const TOKEN: &str = "fixture-config-reader-token-over-32-chars";
fn policy(root: &Path) -> std::path::PathBuf {
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.join("access.json");
    let value = serde_json::json!({
        "principals":[{"id":"reader","token_sha256":hex::encode(Sha256::digest(TOKEN)),
            "grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true},
                      {"platform_id":"p","company_id":"a","host_id":"metadata-only"},
                      {"platform_id":"p","company_id":"a","host_id":"offline","provider_config_read":true},
                      {"platform_id":"p","company_id":"a","host_id":"remote","provider_config_read":true}]}],
        "hosts":[{"id":"host","online":true,"binding":"local","home":root,"search_paths":[],"applications":root.join("Applications")},
                 {"id":"metadata-only","online":true,"binding":"local","home":root,"search_paths":[],"applications":root.join("Applications")},
                 {"id":"offline","online":false,"binding":"local","home":root,"search_paths":[],"applications":root.join("Applications")},
                 {"id":"remote","online":true,"binding":"remote","home":root,"search_paths":[],"applications":root.join("Applications")}]
    });
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    path
}

#[tokio::test]
async fn cc04_paths_sdk_api_execution_home_and_permission_boundary() {
    use vw_gateway_client::{GatewayClient, GatewayEndpoint};
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("custom")).unwrap();
    let path = root.path().join("custom/models.yaml");
    std::fs::write(&path, "fixture-secret").unwrap();
    let router = Router::new()
        .nest("/v1", super::paths::router_with_policy(Some(policy(root.path()))))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_tenant_scope(TenantScope::new("p", "a").unwrap()),
    )
    .unwrap();
    let mut request = ProviderPathsRequest {
        target_host_id: "host".into(),
        directory_override: Some("~/custom".into()),
    };
    let result = client.app_provider_paths(AppId::Omp, &request, TOKEN).await.unwrap();
    assert_eq!(result.directory, root.path().join("custom").to_str().unwrap());
    assert!(
        result
            .files
            .iter()
            .any(|p| p.path.ends_with("models.yaml") && p.state == ConfigPathState::Present)
    );
    assert!(!serde_json::to_string(&result).unwrap().contains("fixture-secret"));
    for (host, status) in
        [("metadata-only", 403), ("missing", 403), ("offline", 503), ("remote", 501)]
    {
        request.target_host_id = host.into();
        assert!(
            matches!(client.app_provider_paths(AppId::Omp, &request, TOKEN).await.unwrap_err(), vw_gateway_client::AgentAppsReadError::Http {status: actual, ..} if actual == status)
        );
    }
    request.target_host_id = "host".into();
    assert!(matches!(
        client.app_provider_paths(AppId::Omp, &request, "invalid").await.unwrap_err(),
        vw_gateway_client::AgentAppsReadError::Http { status: 401, .. }
    ));
    let foreign = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_tenant_scope(TenantScope::new("p", "b").unwrap()),
    )
    .unwrap();
    assert!(matches!(
        foreign.app_provider_paths(AppId::Omp, &request, TOKEN).await.unwrap_err(),
        vw_gateway_client::AgentAppsReadError::Http { status: 403, .. }
    ));
    request.directory_override = Some("/outside-authorization".into());
    assert!(matches!(
        client.app_provider_paths(AppId::Omp, &request, TOKEN).await.unwrap_err(),
        vw_gateway_client::AgentAppsReadError::Http { status: 400, .. }
    ));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "fixture-secret");
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn cc04_paths_registered_route_requires_gateway_authentication() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let app = crate::gateway::router::handler_router()
        .layer(axum::Extension(TenantScope::new("p", "a").unwrap()))
        .layer(middleware::from_fn(|req, next| async {
            crate::gateway::middleware::basic_auth_middleware_with_credentials(
                req,
                next,
                Some("secret"),
                "test",
            )
            .await
        }));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/app-providers/omp/config/paths")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"targetHostId":"host"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

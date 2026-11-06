use axum::{
    Extension,
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use vw_api_types::id::TenantScope;
#[tokio::test]
async fn cc04_catalog_api_lists_only_authenticated_tenant_and_application() {
    let root = tempfile::tempdir().unwrap();
    let database = crate::global_database::GlobalDatabase::for_home(root.path()).unwrap();
    let app = crate::gateway::router::handler_router()
        .layer(Extension(database))
        .layer(Extension(TenantScope::new("p", "a").unwrap()))
        .layer(axum::middleware::from_fn(|req, next| async {
            crate::gateway::middleware::basic_auth_middleware_with_credentials(
                req,
                next,
                Some("secret"),
                "test",
            )
            .await
        }));
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/app-providers/claude").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/app-providers/claude")
                .header("authorization", "Basic dGVzdDpzZWNyZXQ=")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[cfg(target_os = "macos")]
mod host_reads {
    use super::*;
    use axum::{Router, body::to_bytes, middleware};
    use sha2::{Digest, Sha256};
    use std::{os::unix::fs::PermissionsExt, path::Path};
    use vw_api_types::{
        agent_apps::AppId,
        app_providers::{AppProviderWrite, ProviderProtocol, catalog::*},
    };
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
    fn router(root: &Path) -> Router {
        let db = crate::global_database::GlobalDatabase::for_home(&root.join("database")).unwrap();
        let store = crate::app_providers::AppProviderStore::new(db.clone()).unwrap();
        for (company, app, id) in [
            ("a", AppId::Claude, "ours"),
            ("b", AppId::Claude, "other-tenant"),
            ("a", AppId::Codex, "other-app"),
        ] {
            store
                .save(
                    &TenantScope::new("p", company).unwrap(),
                    app,
                    id,
                    &AppProviderWrite {
                        name: id.into(),
                        protocol: if app == AppId::Claude {
                            ProviderProtocol::Anthropic
                        } else {
                            ProviderProtocol::OpenaiResponses
                        },
                        api_key: "fixture-private-key".into(),
                        base_url: "https://fixture.example.test".into(),
                        model: "fixture".into(),
                    },
                )
                .unwrap();
        }
        super::super::catalog::router_with_policy(Some(policy(root)))
            .layer(Extension(db))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
    }
    fn request(path: &str, company: &str, token: &str) -> Request<Body> {
        Request::builder()
            .uri(path)
            .header("x-platform-id", "p")
            .header("x-company-id", company)
            .header("x-agent-apps-token", token)
            .body(Body::empty())
            .unwrap()
    }
    #[tokio::test]
    async fn cc04_catalog_api_scopes_rows_and_denies_config_without_independent_grant() {
        let root = tempfile::tempdir().unwrap();
        let app = router(root.path());
        let response =
            app.clone().oneshot(request("/app-providers/claude", "a", TOKEN)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
        let rows: Vec<vw_api_types::app_providers::AppProviderSummary> =
            serde_json::from_slice(&bytes).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "ours");
        assert!(!String::from_utf8_lossy(&bytes).contains("fixture-private-key"));
        for (host, company, token, expected) in [
            ("host", "b", TOKEN, StatusCode::FORBIDDEN),
            ("metadata-only", "a", TOKEN, StatusCode::FORBIDDEN),
            ("missing", "a", TOKEN, StatusCode::FORBIDDEN),
            ("offline", "a", TOKEN, StatusCode::SERVICE_UNAVAILABLE),
            ("remote", "a", TOKEN, StatusCode::NOT_IMPLEMENTED),
            ("host", "a", "invalid", StatusCode::UNAUTHORIZED),
        ] {
            let response = app
                .clone()
                .oneshot(request(
                    &format!("/app-providers/kimi/catalog/state?targetHostId={host}"),
                    company,
                    token,
                ))
                .await
                .unwrap();
            assert_eq!(response.status(), expected);
            assert!(to_bytes(response.into_body(), 65536).await.unwrap().is_empty());
        }
        assert!(!root.path().join(".kimi-code").exists());
    }
    #[tokio::test]
    async fn cc04_catalog_sdk_http_reads_native_files_without_writes() {
        use vw_gateway_client::{GatewayClient, GatewayEndpoint};
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join(".kimi-code")).unwrap();
        let path = root.path().join(".kimi-code/config.toml");
        let content = "default_model = 'friendly-alias'\n[providers.native]\napi_key = 'fixture-native-secret'\n[models.friendly-alias]\nprovider = 'native'\n";
        std::fs::write(&path, content).unwrap();
        let router = Router::new().nest("/v1", router(root.path()));
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
        assert_eq!(client.app_provider_list(AppId::Claude).await.unwrap()[0].id, "ours");
        let result = client.app_provider_catalog(AppId::Kimi, "host", TOKEN).await.unwrap();
        assert_eq!(result.in_use, ProviderSelection::Selected { id: "native".into() });
        assert_eq!(
            result.direct,
            ProviderSelection::Unavailable { reason: ProviderReadUnavailable::NotImplemented }
        );
        assert!(!serde_json::to_string(&result).unwrap().contains("fixture-native-secret"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
        std::fs::write(&path, "broken = [").unwrap();
        assert_eq!(
            client.app_provider_catalog(AppId::Kimi, "host", TOKEN).await.unwrap().native_read,
            Some(ProviderReadUnavailable::InvalidConfiguration)
        );
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            client.app_provider_catalog(AppId::Kimi, "host", TOKEN).await.unwrap().in_use,
            ProviderSelection::Unconfigured
        );
        let error =
            client.app_provider_catalog(AppId::Kimi, "metadata-only", TOKEN).await.unwrap_err();
        assert!(matches!(error, vw_gateway_client::AgentAppsReadError::Http { status: 403, .. }));
        server.abort();
        let _ = server.await;
    }
}

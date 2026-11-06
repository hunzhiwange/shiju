use crate::{
    app_providers::AppProviderStore, gateway::settings_store::GatewaySettingsStore,
    global_database::GlobalDatabase,
};
use axum::{
    Extension,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use vw_api_types::{agent_apps::AppId, app_providers::universal_sync::*, id::TenantScope};
fn parent() -> String {
    serde_json::json!({"config":{"name":"Shared","providerType":"custom","apps":{"claude":true,"codex":true,"gemini":true},"baseUrl":"https://example.com","models":{"claude":{"model":"claude-main"},"codex":{"model":"codex-main"}}},"apiKey":"fixture-key"}).to_string()
}
#[tokio::test]
async fn cc04_universal_sync_sdk_api_persistence_and_partial_failure() {
    use futures_util::FutureExt;
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    settings.upsert(&tenant, "app_providers.universal", "shared", &parent()).unwrap();
    let live = root.path().join("native.json");
    std::fs::write(&live, b"native unchanged").unwrap();
    let app = axum::Router::new().nest(
        "/v1",
        crate::gateway::router::handler_router()
            .layer(Extension(db.clone()))
            .layer(axum::middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware)),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let endpoint = vw_gateway_client::GatewayEndpoint::new("127.0.0.1", addr.port());
    let client =
        vw_gateway_client::GatewayClient::new(endpoint.clone().with_tenant_scope(tenant.clone()))
            .unwrap();
    let result = std::panic::AssertUnwindSafe(async {
        let req = UniversalSyncRequest { apps: vec![AppId::Claude, AppId::Kimi, AppId::Codex] };
        let rows = client.universal_provider_sync("shared", &req).await.unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].catalog, UniversalCatalogResult::Updated);
        assert_eq!(
            rows[1].catalog,
            UniversalCatalogResult::Failed { code: UniversalSyncError::UnsupportedApp }
        );
        assert_eq!(rows[2].catalog, UniversalCatalogResult::Updated);
        assert!(rows.iter().all(|r| r.live == UniversalLiveResult::NotAttempted));
        assert!(!serde_json::to_string(&rows).unwrap().contains("fixture-key"));
        let key = "\"claude\":universal-claude-shared";
        let mut before: serde_json::Value = serde_json::from_str(
            &settings.get(&tenant, "app_providers.records", key).unwrap().unwrap(),
        )
        .unwrap();
        before["meta"] = serde_json::json!({"usage":{"apiKey":"local-secret"}});
        before["sortIndex"] = serde_json::json!(9);
        before["settingsConfig"]["env"]["APP_ONLY"] = serde_json::json!("keep");
        settings.upsert(&tenant, "app_providers.records", key, &before.to_string()).unwrap();
        // One corrupt child fails without preventing other selected apps from persisting.
        settings
            .upsert(
                &tenant,
                "app_providers.records",
                "\"codex\":universal-codex-shared",
                "bad-json",
            )
            .unwrap();
        let rows = client.universal_provider_sync("shared", &req).await.unwrap();
        assert_eq!(rows[0].catalog, UniversalCatalogResult::Updated);
        assert_eq!(
            rows[2].catalog,
            UniversalCatalogResult::Failed { code: UniversalSyncError::InvalidProfile }
        );
        let reopened =
            GatewaySettingsStore::new(GlobalDatabase::for_home(root.path()).unwrap()).unwrap();
        let after: serde_json::Value = serde_json::from_str(
            &reopened.get(&tenant, "app_providers.records", key).unwrap().unwrap(),
        )
        .unwrap();
        assert_eq!(after["meta"], before["meta"]);
        assert_eq!(after["sortIndex"], 9);
        assert_eq!(after["settingsConfig"]["env"]["APP_ONLY"], "keep");
        assert_eq!(after["apiKey"], "fixture-key");
        assert!(
            settings
                .get(&tenant, "app_providers.records", "\"gemini\":universal-gemini-shared")
                .unwrap()
                .is_none()
        );
        for scope in [TenantScope::new("p", "b").unwrap(), TenantScope::new("q", "a").unwrap()] {
            let other = vw_gateway_client::GatewayClient::new(
                endpoint.clone().with_tenant_scope(scope.clone()),
            )
            .unwrap();
            assert!(
                other.universal_provider_sync("shared", &req).await.unwrap_err().contains("404")
            );
            assert!(settings.list(&scope, "app_providers.records").unwrap().is_empty());
        }
        assert_eq!(
            settings.get(&tenant, "app_providers.universal", "shared").unwrap().unwrap(),
            parent()
        );
        assert_eq!(std::fs::read(live).unwrap(), b"native unchanged");
    })
    .catch_unwind()
    .await;
    server.abort();
    let _ = server.await;
    result.unwrap();
}
#[tokio::test]
async fn cc04_universal_sync_api_denial_has_no_side_effects() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    settings.upsert(&tenant, "app_providers.universal", "shared", &parent()).unwrap();
    let router = crate::gateway::router::handler_router()
        .layer(Extension(db.clone()))
        .layer(Extension(tenant.clone()))
        .layer(axum::middleware::from_fn(|req, next| async {
            crate::gateway::middleware::basic_auth_middleware_with_credentials(
                req,
                next,
                Some("secret"),
                "test",
            )
            .await
        }));
    for (auth, body, status) in [
        (false, serde_json::json!({"apps":["claude"]}), StatusCode::UNAUTHORIZED),
        (true, serde_json::json!({"apps":[]}), StatusCode::BAD_REQUEST),
        (true, serde_json::json!({"apps":["claude","claude"]}), StatusCode::BAD_REQUEST),
        (true, serde_json::json!({"apps":["claude"],"activate":true}), StatusCode::BAD_REQUEST),
    ] {
        let mut request = Request::builder()
            .method("POST")
            .uri("/universal-providers/shared/sync")
            .header("content-type", "application/json");
        if auth {
            request = request.header("authorization", "Basic dGVzdDpzZWNyZXQ=");
        }
        let response = router
            .clone()
            .oneshot(request.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        let bytes = to_bytes(response.into_body(), 8192).await.unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("fixture-key"));
        assert!(settings.list(&tenant, "app_providers.records").unwrap().is_empty());
    }
    assert_eq!(
        AppProviderStore::new(db).unwrap().universal_get(&tenant, "shared").unwrap().config.name,
        "Shared"
    );
}

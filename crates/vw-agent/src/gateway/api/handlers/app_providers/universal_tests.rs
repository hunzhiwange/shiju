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
use vw_api_types::{
    app_providers::{create::ProviderSecretWrite, universal::UniversalProviderWrite},
    id::TenantScope,
};
fn request() -> UniversalProviderWrite {
    serde_json::from_value(serde_json::json!({"config":{"name":"共享","providerType":"custom","apps":{"claude":true,"codex":true},"baseUrl":"https://example.com","models":{"claude":{"haikuModel":"haiku"},"codex":{"reasoningEffort":"high"},"gemini":{"model":"gemini"}}},"apiKey":{"action":"replace","value":"fixture-secret"}})).unwrap()
}
#[tokio::test]
async fn cc04_universal_sdk_api_sqlite_crud_and_isolation() {
    use futures_util::FutureExt;
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    let other = TenantScope::new("p", "b").unwrap();
    let platform = TenantScope::new("q", "a").unwrap();
    let store = AppProviderStore::new(db.clone()).unwrap();
    // Seed other scopes directly, so red evidence reaches the actual SDK business assertion.
    let seed = serde_json::json!({"config":request().config,"apiKey":"other-secret"}).to_string();
    for scope in [&other, &platform] {
        settings.upsert(scope, "app_providers.universal", "shared", &seed).unwrap();
    }
    settings
        .upsert(
            &tenant,
            "app_providers.records",
            "\"claude\":shared",
            "{\"apiKey\":\"child-secret\"}",
        )
        .unwrap();
    let live = root.path().join("native.json");
    std::fs::write(&live, b"native fixture").unwrap();
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
    let client = vw_gateway_client::GatewayClient::new(
        vw_gateway_client::GatewayEndpoint::new("127.0.0.1", addr.port())
            .with_tenant_scope(tenant.clone()),
    )
    .unwrap();
    let result = std::panic::AssertUnwindSafe(async {
        let mut req = request();
        let saved = client.universal_provider_save("shared", &req).await.unwrap();
        assert_eq!(saved.config, req.config);
        assert!(saved.configured);
        let raw = settings.get(&tenant, "app_providers.universal", "shared").unwrap().unwrap();
        let mut raw: serde_json::Value = serde_json::from_str(&raw).unwrap();
        raw["meta"] = serde_json::json!({"token":"hidden-token"});
        raw["future"] = serde_json::json!(42);
        settings.upsert(&tenant, "app_providers.universal", "shared", &raw.to_string()).unwrap();
        req.config.name = "Changed".into();
        req.api_key = ProviderSecretWrite::Keep;
        assert_eq!(
            client.universal_provider_save("shared", &req).await.unwrap().config,
            req.config
        );
        assert_eq!(client.universal_provider_get("shared").await.unwrap().config, req.config);
        assert_eq!(client.universal_providers_list().await.unwrap().len(), 1);
        let reopened =
            AppProviderStore::new(GlobalDatabase::for_home(root.path()).unwrap()).unwrap();
        assert_eq!(reopened.universal_get(&tenant, "shared").unwrap().config, req.config);
        let disk: serde_json::Value = serde_json::from_str(
            &settings.get(&tenant, "app_providers.universal", "shared").unwrap().unwrap(),
        )
        .unwrap();
        assert_eq!(disk["apiKey"], "fixture-secret");
        assert_eq!(disk["meta"], raw["meta"]);
        assert_eq!(disk["future"], 42);
        req.api_key = ProviderSecretWrite::Replace { value: "replacement-secret".into() };
        client.universal_provider_save("shared", &req).await.unwrap();
        req.api_key = ProviderSecretWrite::Clear;
        assert!(client.universal_provider_save("shared", &req).await.unwrap_err().contains("400"));
        assert!(client.universal_provider_delete("shared").await.unwrap());
        assert!(!client.universal_provider_delete("shared").await.unwrap());
        assert!(client.universal_provider_get("shared").await.unwrap_err().contains("404"));
        assert!(client.universal_providers_list().await.unwrap().is_empty());
        for scope in [&other, &platform] {
            assert_eq!(
                settings.get(scope, "app_providers.universal", "shared").unwrap().unwrap(),
                seed
            );
            assert!(store.universal_get(scope, "shared").unwrap().configured);
        }
        assert_eq!(
            settings.get(&tenant, "app_providers.records", "\"claude\":shared").unwrap().unwrap(),
            "{\"apiKey\":\"child-secret\"}"
        );
        assert_eq!(std::fs::read(live).unwrap(), b"native fixture");
    })
    .catch_unwind()
    .await;
    server.abort();
    let _ = server.await;
    result.unwrap();
}
#[tokio::test]
async fn cc04_universal_api_denial_corruption_and_redaction() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    let raw=serde_json::json!({"config":request().config,"apiKey":"fixture-secret","meta":{"token":"hidden-token"}}).to_string();
    settings.upsert(&tenant, "app_providers.universal", "shared", &raw).unwrap();
    let router = crate::gateway::router::handler_router()
        .layer(Extension(db))
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
    for (auth, method, path, body, status) in [
        (
            false,
            "PUT",
            "/universal-providers/shared",
            serde_json::to_value(request()).unwrap(),
            StatusCode::UNAUTHORIZED,
        ),
        (
            false,
            "DELETE",
            "/universal-providers/shared",
            serde_json::json!(null),
            StatusCode::UNAUTHORIZED,
        ),
        (
            true,
            "PUT",
            "/universal-providers/shared",
            serde_json::json!({"path":"/private/fixture-secret"}),
            StatusCode::BAD_REQUEST,
        ),
        (true, "GET", "/universal-providers/shared", serde_json::json!(null), StatusCode::OK),
        (true, "GET", "/universal-providers", serde_json::json!(null), StatusCode::OK),
        (
            true,
            "GET",
            "/universal-providers/missing",
            serde_json::json!(null),
            StatusCode::NOT_FOUND,
        ),
    ] {
        let mut req =
            Request::builder().method(method).uri(path).header("content-type", "application/json");
        if auth {
            req = req.header("authorization", "Basic dGVzdDpzZWNyZXQ=");
        }
        let response =
            router.clone().oneshot(req.body(Body::from(body.to_string())).unwrap()).await.unwrap();
        assert_eq!(response.status(), status);
        let bytes = to_bytes(response.into_body(), 262144).await.unwrap();
        let text = String::from_utf8_lossy(&bytes);
        for secret in ["fixture-secret", "hidden-token", "/private/"] {
            assert!(!text.contains(secret));
        }
        assert_eq!(
            settings.get(&tenant, "app_providers.universal", "shared").unwrap().unwrap(),
            raw
        );
    }
    settings
        .upsert(&tenant, "app_providers.universal", "shared", "{\"apiKey\":\"fixture-secret\"}")
        .unwrap();
    for path in ["/universal-providers/shared", "/universal-providers"] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("authorization", "Basic dGVzdDpzZWNyZXQ=")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}

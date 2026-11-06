use crate::{
    app_providers::AppProviderStore, gateway::settings_store::GatewaySettingsStore,
    global_database::GlobalDatabase,
};
use axum::{
    Extension,
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::endpoints::{
        AppProviderEndpointChange as Change, AppProviderEndpointKind as Kind,
    },
    id::TenantScope,
};
fn seed(db: &GlobalDatabase) {
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    for (platform, company, app, id) in [
        ("p", "a", "claude", "a"),
        ("p", "a", "claude", "z"),
        ("p", "a", "claude", "presentation"),
        ("p", "a", "codex", "a"),
        ("p", "b", "claude", "a"),
        ("q", "a", "claude", "a"),
    ] {
        settings.upsert(&TenantScope::new(platform,company).unwrap(),"app_providers.records",&format!("\"{app}\":{id}"),&serde_json::json!({"apiKey":"fixture-secret","name":"fixture","protocol":"anthropic","baseUrl":"https://fixture.example.test","model":"fixture-model","meta":{"binding":"fixture-account"},"current":true}).to_string()).unwrap();
    }
}
fn input() -> Change {
    Change::Add { url: "https://relay.example.test/v1".into(), kind: Kind::BaseUrl }
}
fn router(db: GlobalDatabase) -> axum::Router {
    crate::gateway::router::handler_router()
        .layer(Extension(db))
        .layer(Extension(TenantScope::new("p", "a").unwrap()))
        .layer(axum::middleware::from_fn(|req, next| async {
            crate::gateway::middleware::basic_auth_middleware_with_credentials(
                req,
                next,
                Some("secret"),
                "test",
            )
            .await
        }))
}
#[tokio::test]
async fn cc04_endpoints_api_rejects_without_side_effects() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    seed(&db);
    let tenant = TenantScope::new("p", "a").unwrap();
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    let before = settings.list(&tenant, "app_providers.records").unwrap();
    for (auth, id, body, status) in [
        (false, "a", serde_json::to_value(input()).unwrap(), StatusCode::UNAUTHORIZED),
        (true, "missing", serde_json::to_value(input()).unwrap(), StatusCode::NOT_FOUND),
        (
            true,
            "a",
            serde_json::json!({"action":"add","url":"invalid","kind":"baseUrl"}),
            StatusCode::BAD_REQUEST,
        ),
        (
            true,
            "a",
            serde_json::json!({"action":"add","url":"https://relay.test","kind":"baseUrl","current":true}),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let mut req = Request::builder()
            .method("POST")
            .uri(format!("/app-providers/claude/{id}/endpoints"))
            .header("content-type", "application/json");
        if auth {
            req = req.header("authorization", "Basic dGVzdDpzZWNyZXQ=");
        }
        let response = router(db.clone())
            .oneshot(req.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(settings.list(&tenant, "app_providers.records").unwrap(), before);
    }
}
#[tokio::test]
async fn cc04_endpoints_sdk_api_persistence_and_isolation() {
    let root = tempfile::tempdir().unwrap();
    let live = root.path().join("native.json");
    std::fs::write(&live, b"native fixture").unwrap();
    let dbhome = root.path().join("db");
    let db = GlobalDatabase::for_home(&dbhome).unwrap();
    seed(&db);
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let scopes =
        [tenant.clone(), TenantScope::new("p", "b").unwrap(), TenantScope::new("q", "a").unwrap()];
    let before: Vec<_> =
        scopes.iter().map(|t| settings.list(t, "app_providers.records").unwrap()).collect();
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
    let added = client.app_provider_endpoint_change(AppId::Claude, "a", &input()).await;
    let used = client
        .app_provider_endpoint_change(
            AppId::Claude,
            "a",
            &Change::MarkUsed { url: "https://relay.example.test/v1/".into(), kind: Kind::BaseUrl },
        )
        .await;
    let full_url = "https://relay.example.test/custom/messages/";
    let full = client
        .app_provider_endpoint_change(
            AppId::Claude,
            "a",
            &Change::Add { url: full_url.into(), kind: Kind::FullUrl },
        )
        .await;
    let full_removed = client
        .app_provider_endpoint_change(
            AppId::Claude,
            "a",
            &Change::Remove { url: full_url.into(), kind: Kind::FullUrl },
        )
        .await;
    let listed = client.app_provider_endpoints(AppId::Claude, "a").await;
    let existing = client.app_provider_get(AppId::Claude, "presentation").await;
    server.abort();
    let _ = server.await;
    let full = full.unwrap();
    assert!(full.iter().any(|e| e.url == full_url && e.kind == Kind::FullUrl));
    assert_eq!(full_removed.unwrap().len(), 1);
    let added = added.unwrap();
    assert_eq!(added.len(), 1);
    assert!(added[0].added_at > 0);
    assert_eq!(added[0].last_used, None);
    let used = used.unwrap();
    assert!(used[0].last_used.unwrap() >= added[0].added_at);
    assert_eq!(listed.unwrap(), used);
    assert_eq!(existing.unwrap().id, "presentation");
    let store = AppProviderStore::new(GlobalDatabase::for_home(&dbhome).unwrap()).unwrap();
    assert_eq!(store.list_endpoints(&tenant, AppId::Claude, "a").unwrap(), used);
    let removed = store
        .change_endpoint(
            &tenant,
            AppId::Claude,
            "a",
            &Change::Remove { url: used[0].url.clone(), kind: Kind::BaseUrl },
        )
        .unwrap();
    assert!(removed.is_empty());
    for (scope, rows) in scopes.iter().zip(before) {
        for row in rows {
            let now =
                settings.get(scope, "app_providers.records", &row.config_key).unwrap().unwrap();
            if scope == &tenant && row.config_key == "\"claude\":a" {
                let mut value: serde_json::Value = serde_json::from_str(&now).unwrap();
                value.as_object_mut().unwrap().remove("customEndpoints");
                assert_eq!(
                    value,
                    serde_json::from_str::<serde_json::Value>(&row.payload_json).unwrap()
                );
            } else {
                assert_eq!(now, row.payload_json);
            }
        }
    }
    assert_eq!(std::fs::read(live).unwrap(), b"native fixture");
}

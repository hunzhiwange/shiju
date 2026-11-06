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
    agent_apps::AppId, app_providers::presentation::AppProviderPresentation, id::TenantScope,
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
fn input() -> AppProviderPresentation {
    AppProviderPresentation {
        sort_index: Some(0),
        category: Some("分组".into()),
        notes: Some("备注".into()),
    }
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
async fn cc04_presentation_api_denies_unauthenticated_and_invalid_without_writes() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    seed(&db);
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let before = settings.list(&tenant, "app_providers.records").unwrap();
    for (auth, id, body, status) in [
        (false, "a", serde_json::json!({}), StatusCode::UNAUTHORIZED),
        (true, "missing", serde_json::json!({}), StatusCode::NOT_FOUND),
        (true, "a", serde_json::json!({"current":true}), StatusCode::BAD_REQUEST),
    ] {
        let mut req = Request::builder()
            .method("PUT")
            .uri(format!("/app-providers/claude/{id}/presentation"))
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
async fn cc04_presentation_sdk_api_reload_isolated_and_native_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let live = root.path().join("settings.json");
    std::fs::write(&live, b"native fixture").unwrap();
    let db = GlobalDatabase::for_home(&root.path().join("db")).unwrap();
    seed(&db);
    let tenant = TenantScope::new("p", "a").unwrap();
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    let before = settings.list(&tenant, "app_providers.records").unwrap();
    let other_scopes = [TenantScope::new("p", "b").unwrap(), TenantScope::new("q", "a").unwrap()];
    let other_before: Vec<_> =
        other_scopes.iter().map(|t| settings.list(t, "app_providers.records").unwrap()).collect();
    let app = axum::Router::new().nest(
        "/v1",
        crate::gateway::router::handler_router()
            .layer(Extension(db.clone()))
            .layer(axum::middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware)),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = vw_gateway_client::GatewayClient::new(
        vw_gateway_client::GatewayEndpoint::new("127.0.0.1", address.port())
            .with_tenant_scope(tenant.clone()),
    )
    .unwrap();
    let result = client.app_provider_presentation_set(AppId::Claude, "z", &input()).await;
    let listed = client.app_provider_presentation_list(AppId::Claude).await;
    let existing_id = client.app_provider_get(AppId::Claude, "presentation").await;
    server.abort();
    let _ = server.await;
    assert_eq!(result.unwrap(), input());
    assert_eq!(listed.unwrap()[0].id, "z");
    assert_eq!(existing_id.unwrap().id, "presentation");
    let reopened =
        AppProviderStore::new(GlobalDatabase::for_home(&root.path().join("db")).unwrap()).unwrap();
    let rows = reopened.list_presentation(&tenant, AppId::Claude).unwrap();
    assert_eq!(rows[0].id, "z");
    assert_eq!(rows[0].presentation, input());
    let changed: serde_json::Value = serde_json::from_str(
        &settings.get(&tenant, "app_providers.records", "\"claude\":z").unwrap().unwrap(),
    )
    .unwrap();
    assert_eq!(changed["apiKey"], "fixture-secret");
    assert_eq!(changed["meta"], serde_json::json!({"binding":"fixture-account"}));
    assert_eq!(changed["current"], true);
    for row in before.iter().filter(|r| r.config_key != "\"claude\":z") {
        assert_eq!(
            settings.get(&tenant, "app_providers.records", &row.config_key).unwrap().as_deref(),
            Some(row.payload_json.as_str())
        );
    }
    for (t, expected) in other_scopes.iter().zip(other_before) {
        assert_eq!(settings.list(t, "app_providers.records").unwrap(), expected);
    }
    assert_eq!(std::fs::read(live).unwrap(), b"native fixture");
}

#[test]
fn cc04_presentation_ties_clear_and_max_index_are_deterministic() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    seed(&db);
    let tenant = TenantScope::new("p", "a").unwrap();
    let store = AppProviderStore::new(db).unwrap();
    let value = AppProviderPresentation { sort_index: Some(u32::MAX), ..input() };
    for id in ["z", "a"] {
        store.set_presentation(&tenant, AppId::Claude, id, &value).unwrap();
    }
    let rows = store.list_presentation(&tenant, AppId::Claude).unwrap();
    assert_eq!(rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["a", "z", "presentation"]);
    store
        .set_presentation(&tenant, AppId::Claude, "a", &AppProviderPresentation::default())
        .unwrap();
    let rows = store.list_presentation(&tenant, AppId::Claude).unwrap();
    assert_eq!(rows[0].id, "z");
    assert_eq!(
        rows.iter().find(|r| r.id == "a").unwrap().presentation,
        AppProviderPresentation::default()
    );
}

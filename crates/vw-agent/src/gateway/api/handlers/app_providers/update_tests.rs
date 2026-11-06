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
use vw_api_types::{agent_apps::AppId, app_providers::update::AppProviderUpdate, id::TenantScope};
const NS: &str = "app_providers.records";
const KEY: &str = "\"claude\":fixture";
fn tenant(company: &str) -> TenantScope {
    TenantScope::new("p", company).unwrap()
}
fn seed(db: &GlobalDatabase) -> serde_json::Value {
    let value = serde_json::json!({"name":"fixture", "protocol":"anthropic", "apiKey":"fixture-secret", "baseUrl":"https://fixture.example.test", "model":"fixture-model", "meta":{"notes":"keep","sortIndex":7}, "createdAt":123});
    GatewaySettingsStore::new(db.clone())
        .unwrap()
        .upsert(&tenant("a"), NS, KEY, &value.to_string())
        .unwrap();
    value
}
fn raw(db: &GlobalDatabase) -> String {
    GatewaySettingsStore::new(db.clone()).unwrap().get(&tenant("a"), NS, KEY).unwrap().unwrap()
}
fn router(db: GlobalDatabase, company: &str) -> axum::Router {
    crate::gateway::router::handler_router()
        .layer(Extension(db))
        .layer(Extension(tenant(company)))
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
fn http(value: serde_json::Value, auth: bool, path: &str) -> Request<Body> {
    let mut b =
        Request::builder().method("PATCH").uri(path).header("content-type", "application/json");
    if auth {
        b = b.header("authorization", "Basic dGVzdDpzZWNyZXQ=");
    }
    b.body(Body::from(value.to_string())).unwrap()
}
#[tokio::test]
async fn cc04_update_api_denials_and_invalid_inputs_have_no_side_effects() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    seed(&db);
    let before = raw(&db);
    let path = "/app-providers/claude/fixture";
    assert_eq!(
        router(db.clone(), "a")
            .oneshot(http(serde_json::json!({"name":"edited"}), false, path))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    for (company, path) in
        [("b", path), ("a", "/app-providers/codex/fixture"), ("a", "/app-providers/claude/missing")]
    {
        assert_eq!(
            router(db.clone(), company)
                .oneshot(http(serde_json::json!({"name":"edited"}), true, path))
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    for body in [
        serde_json::json!({"apiKey":{"action":"replace","value":"********"}}),
        serde_json::json!({"apiKey":{"action":"clear"}}),
        serde_json::json!({"apiKey":{"action":"replace","value":""}}),
        serde_json::json!({"model":""}),
        serde_json::json!({"name":null}),
        serde_json::json!({"companyId":"b"}),
    ] {
        assert_eq!(
            router(db.clone(), "a").oneshot(http(body, true, path)).await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(raw(&db), before);
    }
    assert_eq!(raw(&db), before);
    assert!(
        AppProviderStore::new(db).unwrap().list(&tenant("b"), AppId::Claude).unwrap().is_empty()
    );
}
#[tokio::test]
async fn cc04_update_sdk_api_preserves_metadata_and_native_files() {
    let root = tempfile::tempdir().unwrap();
    let native = root.path().join(".claude");
    std::fs::create_dir(&native).unwrap();
    let live = native.join("settings.json");
    let native_before = br#"{"env":{"unmanaged":"keep"}}"#;
    std::fs::write(&live, native_before).unwrap();
    let db = GlobalDatabase::for_home(&root.path().join("database")).unwrap();
    let mut expected = seed(&db);
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
            .with_tenant_scope(tenant("a")),
    )
    .unwrap();
    let summary = client
        .app_provider_update(
            AppId::Claude,
            "fixture",
            &AppProviderUpdate { name: Some("edited".into()), ..Default::default() },
        )
        .await
        .unwrap();
    assert_eq!(summary.name, "edited");
    assert!(!serde_json::to_string(&summary).unwrap().contains("fixture-secret"));
    expected["name"] = "edited".into();
    assert_eq!(serde_json::from_str::<serde_json::Value>(&raw(&db)).unwrap(), expected);
    assert_eq!(client.app_provider_get(AppId::Claude, "fixture").await.unwrap(), summary);
    let rotated: AppProviderUpdate = serde_json::from_value(serde_json::json!({"apiKey":{"action":"replace","value":"fixture-rotated"},"model":"rotated-model"})).unwrap();
    client.app_provider_update(AppId::Claude, "fixture", &rotated).await.unwrap();
    expected["apiKey"] = "fixture-rotated".into();
    expected["model"] = "rotated-model".into();
    assert_eq!(serde_json::from_str::<serde_json::Value>(&raw(&db)).unwrap(), expected);
    client
        .app_provider_update(AppId::Claude, "fixture", &AppProviderUpdate::default())
        .await
        .unwrap();
    assert_eq!(serde_json::from_str::<serde_json::Value>(&raw(&db)).unwrap(), expected);
    assert_eq!(std::fs::read(&live).unwrap(), native_before);
    assert_eq!(std::fs::read_dir(native).unwrap().count(), 1);
    server.abort();
    let _ = server.await;
}

#[test]
fn cc04_update_conditional_sqlite_write_rejects_stale_and_cross_scope_payloads() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    seed(&db);
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    let old = raw(&db);
    let mut newest: serde_json::Value = serde_json::from_str(&old).unwrap();
    newest["model"] = "new-model".into();
    assert!(!settings.compare_exchange(&tenant("b"), NS, KEY, &old, &newest.to_string()).unwrap());
    assert!(
        !settings
            .compare_exchange(
                &TenantScope::new("other", "a").unwrap(),
                NS,
                KEY,
                &old,
                &newest.to_string()
            )
            .unwrap()
    );
    assert!(
        !settings
            .compare_exchange(&tenant("a"), NS, "\"codex\":fixture", &old, &newest.to_string())
            .unwrap()
    );
    assert_eq!(raw(&db), old);
    assert!(settings.compare_exchange(&tenant("a"), NS, KEY, &old, &newest.to_string()).unwrap());
    let timestamp = || {
        db.connect_sqlite()
            .unwrap()
            .query_row("SELECT updated_at_ms FROM gateway_settings", [], |r| r.get::<_, i64>(0))
            .unwrap()
    };
    let before = timestamp();
    assert!(!settings.compare_exchange(&tenant("a"), NS, KEY, &old, &old).unwrap());
    assert_eq!(raw(&db), newest.to_string());
    assert_eq!(timestamp(), before);
    let store = AppProviderStore::new(db.clone()).unwrap();
    let summary = store
        .update(
            &tenant("a"),
            AppId::Claude,
            "fixture",
            &AppProviderUpdate { name: Some("renamed".into()), ..Default::default() },
        )
        .unwrap();
    assert_eq!(summary.name, "renamed");
    newest["name"] = "renamed".into();
    assert_eq!(serde_json::from_str::<serde_json::Value>(&raw(&db)).unwrap(), newest);
}

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
    agent_apps::AppId, app_providers::duplicate::AppProviderDuplicate, id::TenantScope,
};
fn seed(db: &GlobalDatabase) {
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    settings.upsert(&TenantScope::new("p","a").unwrap(),"app_providers.records","\"claude\":source",&serde_json::json!({"name":"fixture","protocol":"anthropic","apiKey":"fixture-secret","baseUrl":"https://fixture.example.test","model":"fixture-model","notes":"description","meta":{"notes":"keep","authBinding":{"accountId":"fixture-account"},"liveConfigManaged":true},"current":true,"createdAt":123}).to_string()).unwrap();
}
fn database_snapshot(db: &GlobalDatabase) -> Vec<(String, String, String, String, String, i64)> {
    let connection = db.connect_sqlite().unwrap();
    let mut statement = connection.prepare("SELECT platform_id, company_id, namespace, config_key, payload_json, updated_at_ms FROM gateway_settings ORDER BY platform_id, company_id, namespace, config_key").unwrap();
    statement
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
}
fn router(db: GlobalDatabase, company: &str) -> axum::Router {
    crate::gateway::router::handler_router()
        .layer(Extension(db))
        .layer(Extension(TenantScope::new("p", company).unwrap()))
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
fn http(auth: bool, app: &str, body: serde_json::Value) -> Request<Body> {
    let mut b = Request::builder()
        .method("POST")
        .uri(format!("/app-providers/{app}/source/duplicate"))
        .header("content-type", "application/json");
    if auth {
        b = b.header("authorization", "Basic dGVzdDpzZWNyZXQ=");
    }
    b.body(Body::from(body.to_string())).unwrap()
}
#[tokio::test]
async fn cc04_duplicate_api_auth_isolation_collision_and_no_side_effects() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    seed(&db);
    let tenant = TenantScope::new("p", "a").unwrap();
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    let snapshot = || settings.list(&tenant, "app_providers.records").unwrap();
    let before = snapshot();
    let database_before = database_snapshot(&db);
    let body = serde_json::json!({"newId":"copy","name":"fixture copy"});
    for (company, auth, app, status) in [
        ("a", false, "claude", StatusCode::UNAUTHORIZED),
        ("b", true, "claude", StatusCode::NOT_FOUND),
        ("a", true, "codex", StatusCode::NOT_FOUND),
    ] {
        assert_eq!(
            router(db.clone(), company)
                .oneshot(http(auth, app, body.clone()))
                .await
                .unwrap()
                .status(),
            status
        );
        assert_eq!(snapshot(), before);
        assert_eq!(database_snapshot(&db), database_before);
    }
    let response =
        router(db.clone(), "a").oneshot(http(true, "claude", body.clone())).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("fixture-secret"));
    let copied: serde_json::Value = serde_json::from_str(
        &settings.get(&tenant, "app_providers.records", "\"claude\":copy").unwrap().unwrap(),
    )
    .unwrap();
    assert_eq!(copied["meta"], serde_json::json!({"notes":"keep"}));
    assert!(copied.get("current").is_none());
    let after = snapshot();
    let database_after = database_snapshot(&db);
    assert_eq!(database_after.iter().find(|r| r.3 == "\"claude\":source"), database_before.first());
    assert_eq!(after.iter().find(|r| r.config_key == "\"claude\":source"), before.first());
    assert_eq!(
        router(db.clone(), "a").oneshot(http(true, "claude", body)).await.unwrap().status(),
        StatusCode::CONFLICT
    );
    for invalid in [
        serde_json::json!({"newId":"source","name":"copy"}),
        serde_json::json!({"newId":"../bad","name":"copy"}),
        serde_json::json!({"newId":"bad","name":"copy","addToLive":true}),
    ] {
        let status =
            router(db.clone(), "a").oneshot(http(true, "claude", invalid)).await.unwrap().status();
        assert!(status == StatusCode::BAD_REQUEST || status == StatusCode::CONFLICT);
    }
    assert_eq!(snapshot(), after);
    assert_eq!(database_snapshot(&db), database_after);
}
#[tokio::test]
async fn cc04_duplicate_sdk_api_sqlite_preserves_source_and_native_files() {
    let root = tempfile::tempdir().unwrap();
    let native = root.path().join("settings.json");
    std::fs::write(&native, b"unmanaged native fixture").unwrap();
    let db = GlobalDatabase::for_home(&root.path().join("db")).unwrap();
    seed(&db);
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
    let tenant = TenantScope::new("p", "a").unwrap();
    let client = vw_gateway_client::GatewayClient::new(
        vw_gateway_client::GatewayEndpoint::new("127.0.0.1", address.port())
            .with_tenant_scope(tenant.clone()),
    )
    .unwrap();
    let request = AppProviderDuplicate { new_id: "copy".into(), name: "fixture copy".into() };
    assert_eq!(
        client.app_provider_duplicate(AppId::Claude, "source", &request).await.unwrap().id,
        "copy"
    );
    assert_eq!(client.app_provider_get(AppId::Claude, "copy").await.unwrap().name, "fixture copy");
    assert!(
        client
            .app_provider_duplicate(AppId::Claude, "source", &request)
            .await
            .unwrap_err()
            .contains("409")
    );
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    let value: serde_json::Value = serde_json::from_str(
        &settings.get(&tenant, "app_providers.records", "\"claude\":copy").unwrap().unwrap(),
    )
    .unwrap();
    assert_eq!(value["apiKey"], "fixture-secret");
    assert_eq!(value["notes"], "description");
    assert_eq!(value["meta"], serde_json::json!({"notes":"keep"}));
    assert_eq!(
        AppProviderStore::new(db)
            .unwrap()
            .get_summary(&tenant, AppId::Claude, "source")
            .unwrap()
            .name,
        "fixture"
    );
    assert_eq!(std::fs::read(native).unwrap(), b"unmanaged native fixture");
    server.abort();
    let _ = server.await;
}

#[test]
fn cc04_duplicate_concurrent_sqlite_copy_has_one_winner() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    seed(&db);
    let tenant = TenantScope::new("p", "a").unwrap();
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    let original = settings.get(&tenant, "app_providers.records", "\"claude\":source").unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(6));
    let results = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for n in 0..6 {
            let db = db.clone();
            let barrier = barrier.clone();
            let tenant = tenant.clone();
            handles.push(scope.spawn(move || {
                let store = AppProviderStore::new(db).unwrap();
                let request =
                    AppProviderDuplicate { new_id: "race".into(), name: format!("copy-{n}") };
                barrier.wait();
                store.duplicate(&tenant, AppId::Claude, "source", &request)
            }));
        }
        handles.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    for result in results.iter().filter(|r| r.is_err()) {
        assert_eq!(result.as_ref().unwrap_err(), "application provider already exists");
    }
    let winner = results.into_iter().find_map(Result::ok).unwrap();
    assert_eq!(
        AppProviderStore::new(db).unwrap().get_summary(&tenant, AppId::Claude, "race").unwrap(),
        winner
    );
    assert_eq!(
        settings.get(&tenant, "app_providers.records", "\"claude\":source").unwrap(),
        original
    );
}

use vw_api_types::{
    agent_apps::AppId,
    app_providers::{ProviderProtocol, create::*},
};
fn request() -> AppProviderCreate {
    AppProviderCreate {
        name: "fixture".into(),
        protocol: ProviderProtocol::Anthropic,
        api_key: ProviderSecretWrite::Replace { value: "fixture-private-key".into() },
        base_url: "https://fixture.example.test".into(),
        model: "fixture-model".into(),
        add_to_live: false,
    }
}
use crate::{app_providers::AppProviderStore, global_database::GlobalDatabase};
use axum::{
    Extension,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use vw_api_types::id::TenantScope;
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
fn http(input: &AppProviderCreate, auth: bool) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/app-providers/claude/new")
        .header("content-type", "application/json");
    if auth {
        builder = builder.header("authorization", "Basic dGVzdDpzZWNyZXQ=");
    }
    builder.body(Body::from(serde_json::to_vec(input).unwrap())).unwrap()
}
#[tokio::test]
async fn cc04_create_api_rejects_unauthorized_and_duplicates_without_mutation() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let store = AppProviderStore::new(db.clone()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let app = router(db.clone(), "a");
    assert_eq!(
        app.clone().oneshot(http(&request(), false)).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    assert!(store.list(&tenant, AppId::Claude).unwrap().is_empty());
    let response = app.clone().oneshot(http(&request(), true)).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("fixture-private-key"));
    let settings = crate::gateway::settings_store::GatewaySettingsStore::new(db.clone()).unwrap();
    let before = settings.list(&tenant, "app_providers.records").unwrap();
    let snapshot = || {
        let connection = db.connect_sqlite().unwrap();
        let mut statement = connection.prepare("SELECT platform_id, company_id, namespace, config_key, payload_json, updated_at_ms FROM gateway_settings ORDER BY platform_id, company_id, namespace, config_key").unwrap();
        statement
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i64>(5)?,
                ))
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
    };
    let database_before = snapshot();
    let mut changed = request();
    changed.name = "replacement".into();
    let response = app.oneshot(http(&changed, true)).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(settings.list(&tenant, "app_providers.records").unwrap(), before);
    assert_eq!(snapshot(), database_before);
    assert_eq!(
        router(db, "b").oneshot(http(&changed, true)).await.unwrap().status(),
        StatusCode::CREATED
    );
    assert_eq!(store.get_summary(&tenant, AppId::Claude, "new").unwrap().name, "fixture");
    assert!(store.list(&tenant, AppId::Codex).unwrap().is_empty());
}
#[tokio::test]
async fn cc04_create_sdk_api_persists_without_native_file_changes() {
    let root = tempfile::tempdir().unwrap();
    let native = root.path().join(".claude");
    std::fs::create_dir(&native).unwrap();
    let live = native.join("settings.json");
    std::fs::write(&live, b"{\"env\":{\"unmanaged\":\"keep\"}}").unwrap();
    let before = std::fs::read(&live).unwrap();
    let db = GlobalDatabase::for_home(&root.path().join("database")).unwrap();
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
    let endpoint = vw_gateway_client::GatewayEndpoint::new("127.0.0.1", address.port())
        .with_tenant_scope(TenantScope::new("p", "a").unwrap());
    let client = vw_gateway_client::GatewayClient::new(endpoint).unwrap();
    assert_eq!(
        client.app_provider_create(AppId::Claude, "new", &request()).await.unwrap().id,
        "new"
    );
    assert_eq!(client.app_provider_get(AppId::Claude, "new").await.unwrap().name, "fixture");
    assert!(
        client
            .app_provider_create(AppId::Claude, "new", &request())
            .await
            .unwrap_err()
            .contains("409")
    );
    let mut input = request();
    input.add_to_live = true;
    assert!(client.app_provider_create(AppId::Claude, "live", &input).await.is_err());
    assert!(
        AppProviderStore::new(db)
            .unwrap()
            .get_summary(&TenantScope::new("p", "a").unwrap(), AppId::Claude, "live")
            .is_err()
    );
    assert_eq!(std::fs::read(&live).unwrap(), before);
    assert_eq!(std::fs::read_dir(native).unwrap().count(), 1);
    server.abort();
    let _ = server.await;
}

#[test]
fn cc04_create_sqlite_concurrent_duplicate_has_one_winner() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let store = AppProviderStore::new(db.clone()).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let results = std::thread::scope(|scope| {
        let mut handles = vec![];
        for n in 0..8 {
            let db = db.clone();
            let barrier = barrier.clone();
            handles.push(scope.spawn(move || {
                let store = AppProviderStore::new(db).unwrap();
                let mut input = request();
                input.name = format!("candidate-{n}");
                barrier.wait();
                store.create(&TenantScope::new("p", "a").unwrap(), AppId::Claude, "race", &input)
            }));
        }
        handles.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    for result in results.iter().filter(|r| r.is_err()) {
        assert_eq!(result.as_ref().unwrap_err(), "application provider already exists");
    }
    let winner = results.into_iter().find_map(Result::ok).unwrap();
    let rows = store.list(&TenantScope::new("p", "a").unwrap(), AppId::Claude).unwrap();
    assert_eq!(rows, vec![winner]);
}

#[tokio::test]
async fn cc04_create_api_rejects_live_and_missing_secret_without_records() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let app = router(db.clone(), "a");
    let mut input = request();
    input.add_to_live = true;
    assert_eq!(
        app.clone().oneshot(http(&input, true)).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
    for secret in [
        ProviderSecretWrite::Keep,
        ProviderSecretWrite::Clear,
        ProviderSecretWrite::Replace { value: String::new() },
    ] {
        let mut input = request();
        input.api_key = secret;
        assert_eq!(
            app.clone().oneshot(http(&input, true)).await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert!(
        AppProviderStore::new(db)
            .unwrap()
            .list(&TenantScope::new("p", "a").unwrap(), AppId::Claude)
            .unwrap()
            .is_empty()
    );
}

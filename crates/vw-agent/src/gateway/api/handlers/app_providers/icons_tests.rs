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
    agent_apps::{AddCustomIconRequest, AppId, RenameCustomIconRequest},
    app_providers::icons::AppProviderIconChange as Change,
    id::TenantScope,
};
const IMAGE: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
#[tokio::test]
async fn cc04_icons_api_denial_and_safe_read() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    let profile =
        serde_json::json!({"icon":"/private/fixture.png","apiKey":"fixture-secret"}).to_string();
    settings.upsert(&tenant, "app_providers.records", "\"claude\":fixture", &profile).unwrap();
    settings.upsert(&tenant,"agent_apps.custom_icons","default",&serde_json::json!([{"id":"fixture","name":"Icon","data":"/private/fixture.png","createdAt":1}]).to_string()).unwrap();
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
            "/app-providers/claude/fixture/icon",
            serde_json::json!({"action":"clear"}),
            StatusCode::UNAUTHORIZED,
        ),
        (
            true,
            "PUT",
            "/app-providers/claude/fixture/icon",
            serde_json::json!({"action":"select","iconId":"fixture"}),
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        (
            true,
            "PUT",
            "/app-providers/claude/fixture/icon",
            serde_json::json!({"action":"select","iconId":"fixture","path":"/private/fixture.png"}),
            StatusCode::BAD_REQUEST,
        ),
        (
            true,
            "GET",
            "/app-providers/claude/fixture/icon",
            serde_json::json!(null),
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        (
            true,
            "PATCH",
            "/agent-apps/custom-icons/fixture",
            serde_json::json!({"name":"New"}),
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        (
            true,
            "GET",
            "/agent-apps/custom-icons",
            serde_json::json!(null),
            StatusCode::SERVICE_UNAVAILABLE,
        ),
    ] {
        let mut request =
            Request::builder().method(method).uri(path).header("content-type", "application/json");
        if auth {
            request = request.header("authorization", "Basic dGVzdDpzZWNyZXQ=");
        }
        let response = router
            .clone()
            .oneshot(request.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains("/private/fixture.png"));
        assert!(!text.contains("fixture-secret"));
        assert_eq!(
            settings.get(&tenant, "app_providers.records", "\"claude\":fixture").unwrap().unwrap(),
            profile
        );
    }
}
#[tokio::test]
async fn cc04_icons_sdk_api_persistence_and_reference_copies() {
    use futures_util::FutureExt;
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    let profile = serde_json::json!({"apiKey":"fixture-secret","future":true}).to_string();
    for (scope, app) in [
        (tenant.clone(), "claude"),
        (tenant.clone(), "codex"),
        (TenantScope::new("p", "b").unwrap(), "claude"),
        (TenantScope::new("q", "a").unwrap(), "claude"),
    ] {
        settings
            .upsert(&scope, "app_providers.records", &format!("\"{app}\":fixture"), &profile)
            .unwrap();
    }
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
        let added = client
            .custom_icon_add(&AddCustomIconRequest { name: "  Icon  ".into(), data: IMAGE.into() })
            .await
            .unwrap();
        assert_eq!(added.icon.name, "Icon");
        assert!(!added.existed);
        assert!(
            client
                .custom_icon_add(&AddCustomIconRequest { name: "Other".into(), data: IMAGE.into() })
                .await
                .unwrap()
                .existed
        );
        let selected = client
            .app_provider_icon_change(
                AppId::Claude,
                "fixture",
                &Change::Select { icon_id: added.icon.id.clone() },
            )
            .await
            .unwrap();
        assert_eq!(selected.data.as_deref(), Some(IMAGE));
        let other = vw_gateway_client::GatewayClient::new(
            vw_gateway_client::GatewayEndpoint::new("127.0.0.1", addr.port())
                .with_tenant_scope(TenantScope::new("p", "b").unwrap()),
        )
        .unwrap();
        assert!(
            other
                .app_provider_icon_change(
                    AppId::Claude,
                    "fixture",
                    &Change::Select { icon_id: added.icon.id.clone() }
                )
                .await
                .unwrap_err()
                .contains("404")
        );

        assert_eq!(
            client
                .custom_icon_rename(&added.icon.id, &RenameCustomIconRequest { name: "New".into() })
                .await
                .unwrap()
                .name,
            "New"
        );
        client.custom_icon_delete(&added.icon.id).await.unwrap();
        assert!(client.custom_icons_list().await.unwrap().is_empty());
        assert_eq!(client.app_provider_icon(AppId::Claude, "fixture").await.unwrap(), selected);
        let store = AppProviderStore::new(GlobalDatabase::for_home(root.path()).unwrap()).unwrap();
        assert_eq!(store.get_icon(&tenant, AppId::Claude, "fixture").unwrap(), selected);
        assert!(
            client
                .app_provider_icon_change(
                    AppId::Claude,
                    "fixture",
                    &Change::Select { icon_id: added.icon.id }
                )
                .await
                .unwrap_err()
                .contains("404")
        );
        let cleared = client
            .app_provider_icon_change(AppId::Claude, "fixture", &Change::Clear)
            .await
            .unwrap();
        assert!(cleared.data.is_none());
        for (scope, app) in [
            (tenant.clone(), "claude"),
            (tenant.clone(), "codex"),
            (TenantScope::new("p", "b").unwrap(), "claude"),
            (TenantScope::new("q", "a").unwrap(), "claude"),
        ] {
            assert_eq!(
                settings
                    .get(&scope, "app_providers.records", &format!("\"{app}\":fixture"))
                    .unwrap()
                    .unwrap(),
                profile
            );
        }
        assert_eq!(std::fs::read(live).unwrap(), b"native fixture");
    })
    .catch_unwind()
    .await;
    server.abort();
    let _ = server.await;
    result.unwrap();
}

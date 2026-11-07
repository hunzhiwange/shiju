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
    app_providers::{ProviderProtocol, basic::*, create::*},
    id::TenantScope,
};
fn input() -> ProviderBasicWrite {
    ProviderBasicWrite {
        fields: ProviderBasic {
            name: "中文供应商".into(),
            category: Some("工作".into()),
            notes: Some("动态备注".into()),
            website_url: Some("https://website.example.test/about".into()),
        },
        api_key: ProviderSecretWrite::Replace { value: "fixture-secret".into() },
        action: BasicOperation::Create {
            draft: Box::new(AppProviderCreate {
                name: "draft".into(),
                protocol: ProviderProtocol::Anthropic,
                api_key: ProviderSecretWrite::Keep,
                base_url: "https://api.example.test".into(),
                model: "fixture-model".into(),
                request_options: None,
                add_to_live: false,
            }),
        },
    }
}
fn router(db: GlobalDatabase) -> axum::Router {
    crate::gateway::router::handler_router()
        .layer(Extension(db))
        .layer(axum::middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
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
async fn cc04_ui_edit_sdk_basic_create_update_copy_preserves_executor_fields() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let routes = crate::gateway::router::handler_router()
        .layer(Extension(db.clone()))
        .layer(axum::middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    let server = tokio::spawn(async move {
        axum::serve(listener, axum::Router::new().nest("/v1", routes)).await.unwrap();
    });
    let endpoint = vw_gateway_client::GatewayEndpoint::new("127.0.0.1", address.port())
        .with_tenant_scope(tenant.clone());
    let client = vw_gateway_client::GatewayClient::new(endpoint).unwrap();
    let mut request = input();
    let created = client.app_provider_basic_write(AppId::Claude, "one", &request).await.unwrap();
    assert_eq!(created, request.fields);
    let raw = settings.get(&tenant, "app_providers.records", "\"claude\":one").unwrap().unwrap();
    let mut stored: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(stored["baseUrl"], "https://api.example.test");
    assert_eq!(stored["websiteUrl"], "https://website.example.test/about");
    stored["meta"] = serde_json::json!({"unknownApplicationField": {"account":"fixture-account"}});
    settings
        .upsert(&tenant, "app_providers.records", "\"claude\":one", &stored.to_string())
        .unwrap();
    request.action = BasicOperation::Update;
    request.api_key = ProviderSecretWrite::Keep;
    request.fields.name = "更新中文".into();
    client.app_provider_basic_write(AppId::Claude, "one", &request).await.unwrap();
    let reopened = AppProviderStore::new(GlobalDatabase::for_home(root.path()).unwrap()).unwrap();
    assert_eq!(reopened.basic(&tenant, AppId::Claude, "one").unwrap().name, "更新中文");
    let updated: serde_json::Value = serde_json::from_str(
        &settings.get(&tenant, "app_providers.records", "\"claude\":one").unwrap().unwrap(),
    )
    .unwrap();
    assert_eq!(updated["apiKey"], "fixture-secret");
    assert_eq!(updated["meta"], stored["meta"]);
    assert_eq!(updated["model"], "fixture-model");
    let view = client.app_provider_basic(AppId::Claude, "one").await.unwrap();
    assert!(!serde_json::to_string(&view).unwrap().contains("fixture-secret"));
    request.action = BasicOperation::Copy { source_id: "one".into() };
    request.fields.name = "复制中文".into();
    client.app_provider_basic_write(AppId::Claude, "copy", &request).await.unwrap();
    assert_eq!(client.app_provider_basic(AppId::Claude, "copy").await.unwrap().name, "复制中文");
    let copied: serde_json::Value = serde_json::from_str(
        &settings.get(&tenant, "app_providers.records", "\"claude\":copy").unwrap().unwrap(),
    )
    .unwrap();
    assert_eq!(copied["apiKey"], "fixture-secret");
    assert!(copied.get("meta").is_none());
    server.abort();
    let _ = server.await;
}
#[tokio::test]
async fn cc04_ui_edit_denied_invalid_and_foreign_scope_do_not_write() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let store = AppProviderStore::new(db.clone()).unwrap();
    store.write_basic(&tenant, AppId::Claude, "one", &input()).unwrap();
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    let before = settings.list(&tenant, "app_providers.records").unwrap();
    let mut marker = input();
    marker.api_key = ProviderSecretWrite::Replace { value: "[REDACTED]".into() };
    assert!(store.write_basic(&tenant, AppId::Claude, "marker", &marker).is_err());
    assert_eq!(settings.list(&tenant, "app_providers.records").unwrap(), before);
    for (auth, company, body, expected) in [
        (false, "a", serde_json::to_value(input()).unwrap(), StatusCode::UNAUTHORIZED),
        (
            true,
            "b",
            serde_json::json!({"fields":input().fields,"apiKey":{"action":"keep"},"action":{"operation":"update"}}),
            StatusCode::NOT_FOUND,
        ),
        (
            true,
            "a",
            serde_json::json!({"fields":input().fields,"apiKey":{"action":"replace","value":"[redacted]"},"action":{"operation":"update"}}),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let mut req = Request::builder()
            .method("PUT")
            .uri("/app-providers/claude/one/basic")
            .header("content-type", "application/json")
            .header("x-platform-id", "p")
            .header("x-company-id", company);
        if auth {
            req = req.header("authorization", "Basic dGVzdDpzZWNyZXQ=");
        }
        assert_eq!(
            router(db.clone())
                .oneshot(req.body(Body::from(body.to_string())).unwrap())
                .await
                .unwrap()
                .status(),
            expected
        );
        assert_eq!(settings.list(&tenant, "app_providers.records").unwrap(), before);
    }
}

#[test]
fn cc04_ui_edit_native_metadata_keep_preserves_nested_configuration_and_rejects_legacy_secret_write()
 {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    let stored = serde_json::json!({"name":"原生中文","claude":{"name":"原生中文","config":{"kind":"third_party","base_url":"https://native.example.test","api_key":"native-fixture-secret","api_key_header":true},"model":null},"sortIndex":9,"unknown":{"nested":"retained"}});
    settings
        .upsert(&tenant, "app_providers.records", "\"claude\":native", &stored.to_string())
        .unwrap();
    let store = AppProviderStore::new(db).unwrap();
    let mut request = input();
    request.action = BasicOperation::Update;
    request.api_key = ProviderSecretWrite::Keep;
    store.write_basic(&tenant, AppId::Claude, "native", &request).unwrap();
    let after =
        settings.get(&tenant, "app_providers.records", "\"claude\":native").unwrap().unwrap();
    let value: serde_json::Value = serde_json::from_str(&after).unwrap();
    assert_eq!(value["claude"]["config"], stored["claude"]["config"]);
    assert_eq!(value["claude"]["name"], request.fields.name);
    assert_eq!(
        store.get_summary(&tenant, AppId::Claude, "native").unwrap().name,
        request.fields.name
    );
    assert_eq!(value["unknown"], stored["unknown"]);
    assert_eq!(value["sortIndex"], 9);
    request.api_key = ProviderSecretWrite::Replace { value: "another-fixture".into() };
    assert!(store.write_basic(&tenant, AppId::Claude, "native", &request).is_err());
    assert_eq!(
        settings.get(&tenant, "app_providers.records", "\"claude\":native").unwrap().unwrap(),
        after
    );
}

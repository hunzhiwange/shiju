use crate::{
    app_providers::AppProviderStore, gateway::settings_store::GatewaySettingsStore,
    global_database::GlobalDatabase,
};
use axum::Extension;
use serde_json::json;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderWrite, create::AppProviderCreate, update::AppProviderUpdate},
    id::TenantScope,
};
#[tokio::test]
async fn cc04_validation_sdk_api_persists_options_and_rejects_bypass_without_mutation() {
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let app = axum::Router::new().nest(
        "/v1",
        crate::gateway::router::handler_router()
            .layer(Extension(db.clone()))
            .layer(axum::middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware)),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = vw_gateway_client::GatewayClient::new(
        vw_gateway_client::GatewayEndpoint::new("127.0.0.1", address.port())
            .with_tenant_scope(tenant.clone()),
    )
    .unwrap();
    let mut wire = json!({"name":"fixture","protocol":"anthropic","apiKey":{"action":"replace","value":"fixture-secret"},"baseUrl":"https://fixture.test","model":"fixture","addToLive":false,"requestOptions":{"requestHeaders":{"Authorization":"fixture-header-secret"},"apiKeyField":"ANTHROPIC_API_KEY","localProxyRequestOverrides":{"headers":{"X-Test":"ok"},"body":{"temperature":0.2}}}});
    let request: AppProviderCreate = serde_json::from_value(wire.clone()).unwrap();
    let summary = client.app_provider_create(AppId::Claude, "fixture", &request).await.unwrap();
    assert!(!serde_json::to_string(&summary).unwrap().contains("secret"));
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    let key = "\"claude\":fixture";
    let before = settings.get(&tenant, "app_providers.records", key).unwrap().unwrap();
    let stored: serde_json::Value = serde_json::from_str(&before).unwrap();
    assert_eq!(
        stored["requestOptions"]["requestHeaders"]["authorization"],
        "fixture-header-secret"
    );
    assert_eq!(stored["requestOptions"]["localProxyRequestOverrides"]["headers"]["x-test"], "ok");
    assert_eq!(stored["requestOptions"]["apiKeyField"], "ANTHROPIC_API_KEY");
    wire["requestOptions"] =
        json!({"localProxyRequestOverrides":{"headers":{"Authorization":"fixture-header-secret"}}});
    let bad: AppProviderCreate = serde_json::from_value(wire.clone()).unwrap();
    assert!(
        client.app_provider_create(AppId::Claude, "bad", &bad).await.unwrap_err().contains("400")
    );
    let bad: AppProviderUpdate =
        serde_json::from_value(json!({"requestOptions":wire["requestOptions"]})).unwrap();
    assert!(
        client
            .app_provider_update(AppId::Claude, "fixture", &bad)
            .await
            .unwrap_err()
            .contains("400")
    );
    let bad:AppProviderWrite=serde_json::from_value(json!({"name":"fixture","protocol":"anthropic","apiKey":"fixture-secret","baseUrl":"https://fixture.test","model":"fixture","requestOptions":wire["requestOptions"]})).unwrap();
    assert!(
        client.app_provider_save(AppId::Claude, "fixture", &bad).await.unwrap_err().contains("400")
    );
    assert_eq!(settings.get(&tenant, "app_providers.records", key).unwrap().unwrap(), before);
    assert_eq!(AppProviderStore::new(db).unwrap().list(&tenant, AppId::Claude).unwrap().len(), 1);
    // Seed application-owned fields through the trusted store, not a generic public writer.
    let mut original: serde_json::Value = serde_json::from_str(&before).unwrap();
    original["settingsConfig"] = json!({"env":{"APP_PRIVATE":"keep"}});
    original["meta"] = json!({"unknown":{"nested":42}});
    settings.upsert(&tenant, "app_providers.records", key, &original.to_string()).unwrap();
    let patch:AppProviderUpdate=serde_json::from_value(json!({"name":"edited","requestOptions":{"localProxyRequestOverrides":{"body":{"temperature":0.8}}}})).unwrap();
    client.app_provider_update(AppId::Claude, "fixture", &patch).await.unwrap();
    let mut put = bad;
    put.request_options = None;
    client.app_provider_save(AppId::Claude, "fixture", &put).await.unwrap();
    let kept: serde_json::Value = serde_json::from_str(
        &settings.get(&tenant, "app_providers.records", key).unwrap().unwrap(),
    )
    .unwrap();
    assert_eq!(kept["settingsConfig"], original["settingsConfig"]);
    assert_eq!(kept["meta"], original["meta"]);
    assert_eq!(
        kept["requestOptions"]["requestHeaders"],
        original["requestOptions"]["requestHeaders"]
    );
    assert_eq!(kept["requestOptions"]["apiKeyField"], original["requestOptions"]["apiKeyField"]);
    assert_eq!(kept["requestOptions"]["localProxyRequestOverrides"]["body"]["temperature"], 0.8);
    let clear: AppProviderUpdate = serde_json::from_value(
        json!({"requestOptions":{"requestHeaders":{},"localProxyRequestOverrides":{}}}),
    )
    .unwrap();
    client.app_provider_update(AppId::Claude, "fixture", &clear).await.unwrap();
    let cleared: serde_json::Value = serde_json::from_str(
        &settings.get(&tenant, "app_providers.records", key).unwrap().unwrap(),
    )
    .unwrap();
    assert_eq!(cleared["requestOptions"]["requestHeaders"], json!({}));
    assert_eq!(cleared["requestOptions"]["localProxyRequestOverrides"], json!({}));
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn cc04_validation_api_auth_and_invalid_wire_inputs_have_no_side_effects() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
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
    for method in ["POST", "PUT", "PATCH"] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/app-providers/claude/fixture")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    for options in [
        json!({"requestHeaders":{"X":1}}),
        json!({"apiKeyField":"ANY_ENV"}),
        json!({"localProxyRequestOverrides":{"body":[]}}),
        json!({"localProxyRequestOverrides":{"headers":{"X-Test":"a\rb"}}}),
        json!({"unexpected":"fixture-secret"}),
    ] {
        let input = json!({"name":"fixture","protocol":"anthropic","apiKey":{"action":"replace","value":"fixture-secret"},"baseUrl":"https://fixture.test","model":"fixture","addToLive":false,"requestOptions":options});
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/app-providers/claude/fixture")
                    .header("content-type", "application/json")
                    .header("authorization", "Basic dGVzdDpzZWNyZXQ=")
                    .body(Body::from(input.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), 65536).await.unwrap();
        assert!(!String::from_utf8_lossy(&body).contains("fixture-secret"));
    }
    assert!(
        GatewaySettingsStore::new(db)
            .unwrap()
            .list(&tenant, "app_providers.records")
            .unwrap()
            .is_empty()
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn cc04_validation_preview_checks_permission_before_options_and_never_writes() {
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::PermissionsExt;
    const TOKEN: &str = "fixture-validation-token-over-32-chars";
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let policy = root.path().join("access.json");
    let value = json!({"principals":[{"id":"reader","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"search_paths":[],"applications":root.path().join("Applications")}]});
    std::fs::write(&policy, value.to_string()).unwrap();
    std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
    let app = axum::Router::new()
        .nest("/v1", super::preview::router_with_policy(Some(policy.clone())))
        .layer(axum::middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = vw_gateway_client::GatewayClient::new(
        vw_gateway_client::GatewayEndpoint::new("127.0.0.1", address.port())
            .with_tenant_scope(TenantScope::new("p", "a").unwrap()),
    )
    .unwrap();
    let request=serde_json::from_value(json!({"targetHostId":"host","settings":{"env":{}},"requestOptions":{"localProxyRequestOverrides":{"headers":{"authorization":"fixture-secret"}}}})).unwrap();
    for (token, status) in [("invalid", 401), (TOKEN, 400)] {
        let error = client.app_provider_preview(AppId::Claude, &request, token).await.unwrap_err();
        assert!(
            matches!(error,vw_gateway_client::AgentAppsReadError::Http{status:actual,body} if actual==status && body.is_null())
        );
    }
    assert_eq!(std::fs::read_to_string(&policy).unwrap(), value.to_string());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    server.abort();
    let _ = server.await;
}

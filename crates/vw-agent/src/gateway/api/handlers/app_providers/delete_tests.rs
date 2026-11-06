use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{
    Extension, Router,
    body::Body,
    http::{Request, StatusCode},
};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::Path};
use tower::ServiceExt;
use vw_api_types::{agent_apps::AppId, app_providers::delete::AppProviderDelete, id::TenantScope};
const TOKEN: &str = "fixture-delete-reader-token-over-32-chars";
fn tenant(company: &str) -> TenantScope {
    TenantScope::new("p", company).unwrap()
}
fn setup(root: &Path) -> (Router, GlobalDatabase) {
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let policy = root.join("access.json");
    let value = serde_json::json!({"principals":[{"id":"reader","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true},{"platform_id":"p","company_id":"a","host_id":"metadata-only"}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root,"search_paths":[],"applications":root.join("Applications")},{"id":"metadata-only","online":true,"binding":"local","home":root,"search_paths":[],"applications":root.join("Applications")}]});
    std::fs::write(&policy, value.to_string()).unwrap();
    std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
    let db = GlobalDatabase::for_home(&root.join("database")).unwrap();
    let settings = GatewaySettingsStore::new(db.clone()).unwrap();
    for app in ["kimi", "opencode", "claude"] {
        for id in ["used", "unused"] {
            // These catalog records may precede provider-session profile support.
            settings
                .upsert(
                    &tenant("a"),
                    "app_providers.records",
                    &format!("\"{app}\":{id}"),
                    r#"{"name":"fixture","protocol":"openai_responses","baseUrl":"https://fixture.example.test","model":"fixture-model","apiKey":"fixture-secret"}"#,
                )
                .unwrap();
        }
    }
    let routes = super::delete::router_with_policy(Some(policy));
    let app = Router::new()
        .merge(routes)
        .layer(Extension(db.clone()))
        .layer(axum::middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    (app, db)
}
fn rows(db: &GlobalDatabase) -> Vec<crate::gateway::settings_store::GatewaySetting> {
    GatewaySettingsStore::new(db.clone())
        .unwrap()
        .list(&tenant("a"), "app_providers.records")
        .unwrap()
}
#[tokio::test]
async fn cc04_delete_sdk_http_rejects_members_and_deletes_only_unused_rows() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join(".kimi-code")).unwrap();
    std::fs::create_dir_all(root.path().join(".config/opencode")).unwrap();
    let kimi = root.path().join(".kimi-code/config.toml");
    let opencode = root.path().join(".config/opencode/opencode.json");
    let kimi_bytes = "default_model='alias'\n[providers.used]\napi_key='fixture-secret'\n[models.alias]\nprovider='used'\n";
    let open_bytes = r#"{"provider":{"used":{"apiKey":"fixture-secret"}}}"#;
    std::fs::write(&kimi, kimi_bytes).unwrap();
    std::fs::write(&opencode, open_bytes).unwrap();
    let (app, db) = setup(root.path());
    // Current native persistence has only catalog records: no managed direct/route writer.
    let namespaces: Vec<String> = db
        .connect_sqlite()
        .unwrap()
        .prepare("SELECT DISTINCT namespace FROM gateway_settings")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(namespaces, vec!["app_providers.records"]);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(listener, Router::new().nest("/v1", app)).await.unwrap();
    });
    let client = vw_gateway_client::GatewayClient::new(
        vw_gateway_client::GatewayEndpoint::new("127.0.0.1", port).with_tenant_scope(tenant("a")),
    )
    .unwrap();
    let request = AppProviderDelete { target_host_id: "host".into() };
    for app in [AppId::Kimi, AppId::OpenCode] {
        let before = rows(&db);
        let error = client.app_provider_delete(app, "used", &request, TOKEN).await.unwrap_err();
        assert!(error.contains("409"));
        assert!(!error.contains("fixture-secret"));
        assert_eq!(rows(&db), before);
        assert!(client.app_provider_delete(app, "unused", &request, TOKEN).await.unwrap().deleted);
        assert_eq!(rows(&db).len(), before.len() - 1);
        assert!(
            client
                .app_provider_delete(app, "unused", &request, TOKEN)
                .await
                .unwrap_err()
                .contains("404")
        );
    }
    assert_eq!(std::fs::read_to_string(kimi).unwrap(), kimi_bytes);
    assert_eq!(std::fs::read_to_string(opencode).unwrap(), open_bytes);
    server.abort();
    let _ = server.await;
}
#[tokio::test]
async fn cc04_delete_api_permission_scope_and_unknown_config_denials_preserve_rows() {
    let root = tempfile::tempdir().unwrap();
    let (app, db) = setup(root.path());
    let app = app.layer(axum::middleware::from_fn(|req, next| async {
        crate::gateway::middleware::basic_auth_middleware_with_credentials(
            req,
            next,
            Some("secret"),
            "test",
        )
        .await
    }));
    let before = rows(&db);
    for (auth, company, host, token, path, expected) in [
        (false, "a", "host", TOKEN, "opencode/unused", StatusCode::UNAUTHORIZED),
        (true, "b", "host", TOKEN, "opencode/unused", StatusCode::FORBIDDEN),
        (true, "a", "metadata-only", TOKEN, "opencode/unused", StatusCode::FORBIDDEN),
        (true, "a", "host", "invalid", "opencode/unused", StatusCode::UNAUTHORIZED),
        (true, "a", "host", TOKEN, "claude/unused", StatusCode::CONFLICT),
        (true, "a", "host", TOKEN, "opencode/missing", StatusCode::NOT_FOUND),
    ] {
        let mut req = Request::builder()
            .method("DELETE")
            .uri(format!("/app-providers/{path}"))
            .header("content-type", "application/json")
            .header("x-platform-id", "p")
            .header("x-company-id", company)
            .header("x-agent-apps-token", token);
        if auth {
            req = req.header("authorization", "Basic dGVzdDpzZWNyZXQ=");
        }
        let response = app
            .clone()
            .oneshot(
                req.body(Body::from(serde_json::json!({"targetHostId":host}).to_string())).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        assert_eq!(rows(&db), before);
    }
}

#[test]
fn cc04_delete_store_preserves_other_tenants_and_applications() {
    let root = tempfile::tempdir().unwrap();
    let (_, db) = setup(root.path());
    let store = crate::app_providers::AppProviderStore::new(db.clone()).unwrap();
    let settings = GatewaySettingsStore::new(db).unwrap();
    let other = TenantScope::new("other-platform", "a").unwrap();
    let key = "\"opencode\":unused";
    for scope in [tenant("b"), other.clone()] {
        settings.upsert(&scope, "app_providers.records", key, "{}").unwrap();
    }
    assert!(store.delete(&tenant("b"), AppId::OpenCode, "missing", root.path()).is_err());
    store.delete(&tenant("a"), AppId::OpenCode, "unused", root.path()).unwrap();
    assert_eq!(settings.get(&tenant("a"), "app_providers.records", key).unwrap(), None);
    for scope in [tenant("b"), other] {
        assert_eq!(settings.get(&scope, "app_providers.records", key).unwrap(), Some("{}".into()));
    }
    assert!(
        settings.get(&tenant("a"), "app_providers.records", "\"kimi\":unused").unwrap().is_some()
    );
}

#[tokio::test]
async fn cc04_delete_is_registered_in_formal_handler_router() {
    let root = tempfile::tempdir().unwrap();
    let (_, db) = setup(root.path());
    let app = crate::gateway::router::handler_router()
        .layer(Extension(db.clone()))
        .layer(Extension(tenant("a")));
    let before = rows(&db);
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/app-providers/opencode/unused")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(rows(&db), before);
}

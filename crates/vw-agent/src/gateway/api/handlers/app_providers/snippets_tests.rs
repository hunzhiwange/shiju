use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use vw_api_types::{agent_apps::AppId, app_providers::snippets::*, id::TenantScope};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-snippet-token-at-least-32-chars";
struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    client: GatewayClient,
    server: tokio::task::JoinHandle<()>,
    port: u16,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::create_dir(root.path().join(".claude")).unwrap();
        std::fs::write(
            root.path().join(".claude/settings.json"),
            br#"{ "hooks": {"keep":true}, "model":"current" }"#,
        )
        .unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let tenant = TenantScope::new("p", "a").unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"snippets","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true},{"platform_id":"p","company_id":"b","host_id":"host","provider_config_read":true,"provider_config_write":true},{"platform_id":"q","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true},{"platform_id":"p","company_id":"read","host_id":"host","provider_config_read":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let router = Router::new()
            .nest("/v1", super::snippets::router_with_policy(Some(policy)))
            .layer(Extension(db.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client =
            GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port).with_tenant_scope(tenant))
                .unwrap();
        Self { root, db, client, server, port }
    }
    fn scoped(&self, p: &str, c: &str) -> GatewayClient {
        GatewayClient::new(
            GatewayEndpoint::new("127.0.0.1", self.port)
                .with_tenant_scope(TenantScope::new(p, c).unwrap()),
        )
        .unwrap()
    }
    fn raw(&self) -> Option<String> {
        GatewaySettingsStore::new(self.db.clone())
            .unwrap()
            .get(&TenantScope::new("p", "a").unwrap(), "app_providers.snippets", "\"claude\"")
            .unwrap()
    }
}
#[tokio::test]
async fn cc04_snippets_sdk_api_sqlite_sanitizes_and_preserves_live() {
    let f = Fixture::new().await;
    let file = f.root.path().join(".claude/settings.json");
    let live = std::fs::read(&file).unwrap();
    let read = SnippetRead { target_host_id: "host".into() };
    assert!(
        f.client
            .app_provider_snippet_get(AppId::Claude, &read, TOKEN)
            .await
            .unwrap()
            .content
            .is_none()
    );
    let settings = json!({"env":{"ANTHROPIC_API_KEY":"fixture-key","AWS_REGION":"fixture-region","DISABLE_TELEMETRY":"1"},"model":"old","hooks":{"keep":false}});
    let extracted = f
        .client
        .app_provider_snippet_extract(
            AppId::Claude,
            &SnippetExtract {
                target_host_id: "host".into(),
                source: SnippetSource::Draft { settings: settings.clone() },
            },
            TOKEN,
        )
        .await
        .unwrap();
    let expected = json!({"env":{"DISABLE_TELEMETRY":"1"},"hooks":{"keep":false}});
    assert_eq!(serde_json::from_str::<Value>(&extracted.content.unwrap()).unwrap(), expected);
    let saved = f
        .client
        .app_provider_snippet_save(
            AppId::Claude,
            &SnippetWrite { target_host_id: "host".into(), content: settings.to_string() },
            TOKEN,
        )
        .await
        .unwrap();
    assert_eq!(serde_json::from_str::<Value>(saved.content.as_ref().unwrap()).unwrap(), expected);
    assert!(!f.raw().unwrap().contains("fixture-key"));
    assert_eq!(
        f.client.app_provider_snippet_get(AppId::Claude, &read, TOKEN).await.unwrap(),
        saved
    );
    let reopened = crate::app_providers::AppProviderStore::new(
        GlobalDatabase::for_home(f.root.path()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        reopened.snippet_get(&TenantScope::new("p", "a").unwrap(), AppId::Claude).unwrap(),
        saved
    );
    for (p, c) in [("p", "b"), ("q", "a")] {
        let other = f.scoped(p, c);
        assert!(
            other
                .app_provider_snippet_get(AppId::Claude, &read, TOKEN)
                .await
                .unwrap()
                .content
                .is_none()
        );
        other
            .app_provider_snippet_save(
                AppId::Claude,
                &SnippetWrite { target_host_id: "host".into(), content: "{\"other\":true}".into() },
                TOKEN,
            )
            .await
            .unwrap();
        assert_eq!(
            f.client.app_provider_snippet_get(AppId::Claude, &read, TOKEN).await.unwrap(),
            saved
        );
    }
    let store = GatewaySettingsStore::new(f.db.clone()).unwrap();
    store
        .upsert(
            &TenantScope::new("p", "a").unwrap(),
            "app_providers.records",
            "\"claude\":saved",
            &json!({"settingsConfig":settings}).to_string(),
        )
        .unwrap();
    let from_saved = f
        .client
        .app_provider_snippet_extract(
            AppId::Claude,
            &SnippetExtract {
                target_host_id: "host".into(),
                source: SnippetSource::Saved { provider_id: "saved".into() },
            },
            TOKEN,
        )
        .await
        .unwrap();
    assert_eq!(from_saved, saved);
    assert!(
        f.scoped("p", "b")
            .app_provider_snippet_extract(
                AppId::Claude,
                &SnippetExtract {
                    target_host_id: "host".into(),
                    source: SnippetSource::Saved { provider_id: "saved".into() }
                },
                TOKEN
            )
            .await
            .is_err()
    );
    let cleared = f
        .client
        .app_provider_snippet_save(
            AppId::Claude,
            &SnippetWrite { target_host_id: "host".into(), content: "  ".into() },
            TOKEN,
        )
        .await
        .unwrap();
    assert!(cleared.content.is_none());
    assert_eq!(std::fs::read(&file).unwrap(), live);
    assert_eq!(std::fs::read_dir(file.parent().unwrap()).unwrap().count(), 1);
}
#[tokio::test]
async fn cc04_snippets_denial_invalid_input_and_corruption_have_no_side_effects() {
    let f = Fixture::new().await;
    let read = SnippetRead { target_host_id: "host".into() };
    f.client
        .app_provider_snippet_save(
            AppId::Claude,
            &SnippetWrite { target_host_id: "host".into(), content: "{\"keep\":true}".into() },
            TOKEN,
        )
        .await
        .unwrap();
    let before = f.raw();
    for content in [
        json!({"targetHostId":"host","content":"{}","path":"/tmp/fixture-key"}),
        json!({"targetHostId":"host","content":null}),
    ] {
        let response = reqwest::Client::new()
            .put(format!("http://127.0.0.1:{}/v1/app-providers/claude/config/snippet", f.port))
            .header(vw_api_types::id::PLATFORM_ID_HEADER, "p")
            .header(vw_api_types::id::COMPANY_ID_HEADER, "a")
            .header("x-agent-apps-token", TOKEN)
            .json(&content)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 400);
    }

    let live = std::fs::read(f.root.path().join(".claude/settings.json")).unwrap();
    for (client, token, content) in
        [(&f.client, "bad", "{}"), (&f.client, TOKEN, "[broken"), (&f.client, TOKEN, "[]")]
    {
        assert!(
            client
                .app_provider_snippet_save(
                    AppId::Claude,
                    &SnippetWrite { target_host_id: "host".into(), content: content.into() },
                    token
                )
                .await
                .is_err()
        );
        assert_eq!(f.raw(), before);
    }
    assert!(
        f.scoped("p", "read")
            .app_provider_snippet_save(
                AppId::Claude,
                &SnippetWrite { target_host_id: "host".into(), content: "{}".into() },
                TOKEN
            )
            .await
            .is_err()
    );
    assert!(
        f.scoped("p", "none").app_provider_snippet_get(AppId::Claude, &read, TOKEN).await.is_err()
    );
    assert_eq!(f.raw(), before);
    let store = GatewaySettingsStore::new(f.db.clone()).unwrap();
    store
        .upsert(
            &TenantScope::new("p", "a").unwrap(),
            "app_providers.snippets",
            "\"claude\"",
            &json!({"content":"{\"env\":{\"ANTHROPIC_API_KEY\":\"fixture-key\"},\"keep\":true}"})
                .to_string(),
        )
        .unwrap();
    let safe = f.client.app_provider_snippet_get(AppId::Claude, &read, TOKEN).await.unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&safe.content.unwrap()).unwrap(),
        json!({"keep":true})
    );
    store
        .upsert(
            &TenantScope::new("p", "a").unwrap(),
            "app_providers.snippets",
            "\"claude\"",
            "fixture-key-broken",
        )
        .unwrap();
    let err = f
        .client
        .app_provider_snippet_get(AppId::Claude, &read, TOKEN)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("503"));
    assert!(!err.contains("fixture-key"));
    assert_eq!(std::fs::read(f.root.path().join(".claude/settings.json")).unwrap(), live);
}

#[tokio::test]
async fn cc04_snippets_registered_gateway_rejects_unauthenticated_writes() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let f = Fixture::new().await;
    let live = std::fs::read(f.root.path().join(".claude/settings.json")).unwrap();
    let router = crate::gateway::router::handler_router()
        .layer(Extension(f.db.clone()))
        .layer(Extension(TenantScope::new("p", "a").unwrap()))
        .layer(middleware::from_fn(|request, next| async {
            crate::gateway::middleware::basic_auth_middleware_with_credentials(
                request,
                next,
                Some("fixture-gateway-password"),
                "fixture",
            )
            .await
        }));
    for (method, path) in [
        ("GET", "/app-providers/claude/config/snippet?targetHostId=host"),
        ("PUT", "/app-providers/claude/config/snippet"),
        ("POST", "/app-providers/claude/config/snippet/extract"),
        ("POST", "/app-providers/claude/config/snippet/preview"),
    ] {
        let result = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"targetHostId":"host","content":"{}"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result.status(), StatusCode::UNAUTHORIZED);
    }
    assert!(f.raw().is_none());
    assert_eq!(std::fs::read(f.root.path().join(".claude/settings.json")).unwrap(), live);
}

#[tokio::test]
async fn cc04_ui_snippets_preview_uses_read_grant_and_leaves_storage_untouched() {
    let f = Fixture::new().await;
    let file = f.root.path().join(".claude/settings.json");
    let live = std::fs::read(&file).unwrap();
    let read_only = f.scoped("p", "read");
    let request = SnippetWrite {
        target_host_id: "host".into(),
        content: json!({"theme":"中文主题","model":"fixture-model","env":{"ANTHROPIC_API_KEY":"fake-key"}}).to_string(),
    };
    let preview =
        read_only.app_provider_snippet_preview(AppId::Claude, &request, TOKEN).await.unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&preview.content.unwrap()).unwrap(),
        json!({"theme":"中文主题"})
    );
    let invalid = SnippetWrite { content: "{ broken".into(), ..request.clone() };
    for (client, payload, status) in
        [(&read_only, &invalid, 400), (&f.scoped("p", "none"), &request, 403)]
    {
        assert!(matches!(
            client.app_provider_snippet_preview(AppId::Claude, payload, TOKEN).await,
            Err(vw_gateway_client::AgentAppsReadError::Http {
                status: actual, body: Value::Null
            }) if actual == status
        ));
    }
    let read = SnippetRead { target_host_id: "host".into() };
    assert!(
        read_only
            .app_provider_snippet_get(AppId::Claude, &read, TOKEN)
            .await
            .unwrap()
            .content
            .is_none()
    );
    assert!(f.raw().is_none());
    assert_eq!(std::fs::read(file).unwrap(), live);
}

use crate::{
    app_providers::AppProviderStore, gateway::settings_store::GatewaySettingsStore,
    global_database::GlobalDatabase,
};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{claude::*, codex_catalog::*, request_options::*},
    id::TenantScope,
};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-options-token-with-at-least-32-characters";
struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    client: GatewayClient,
    server: tokio::task::JoinHandle<()>,
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
        let private = root.path().join("private");
        std::fs::create_dir(&private).unwrap();
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"options","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let store = AppProviderStore::new(db.clone()).unwrap();
        store
            .claude_save(
                &tenant(),
                "fixture",
                &ClaudeProfile {
                    name: "中文档案".into(),
                    config: ClaudeConfig::ThirdParty {
                        base_url: "https://fixture.test".into(),
                        api_key: "fake-secret".into(),
                        api_key_header: false,
                    },
                    model: None,
                },
                None,
                true,
            )
            .unwrap();
        store.codex_save(&tenant(),"fixture",&CodexProfile{name:"中文档案".into(),config:"model_provider = \"vendor\"\n[model_providers.vendor]\nname = \"Vendor\"\nbase_url = \"https://fixture.test/v1\"\nwire_api = \"responses\"\n".into(),api_key:Some("fake-secret".into()),preserve_login:false},None,None).unwrap();
        let router = super::request_options::router_with_policy(Some(policy.clone()))
            .merge(super::endpoints::router())
            .merge(super::claude::router_with_policy(Some(policy.clone())))
            .merge(super::codex_catalog::router_with_policy(Some(policy)))
            .layer(Extension(db.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            axum::serve(listener, Router::new().nest("/v1", router)).await.unwrap()
        });
        let client =
            GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port).with_tenant_scope(tenant()))
                .unwrap();
        Self { root, db, client, server }
    }
    fn row(&self, app: AppId) -> String {
        GatewaySettingsStore::new(self.db.clone())
            .unwrap()
            .get(
                &tenant(),
                "app_providers.records",
                &format!("{}:fixture", serde_json::to_string(&app).unwrap()),
            )
            .unwrap()
            .unwrap()
    }
}
fn tenant() -> TenantScope {
    TenantScope::new("p", "a").unwrap()
}
fn target() -> RequestOptionsTarget {
    RequestOptionsTarget { target_host_id: "host".into() }
}
fn request() -> RequestOptionsWrite {
    serde_json::from_value(json!({"targetHostId":"host","options":{"requestHeaders":{"Authorization":"fake-sensitive-header","X-Title":"fixture"},"localProxyRequestOverrides":{"headers":{"X-Proxy":"fixture"},"body":{"temperature":0.2}}}})).unwrap()
}
#[tokio::test]
async fn cc04_ui_endpoints_options_survive_native_save_and_never_return_secrets() {
    let f = Fixture::new().await;
    for app in [AppId::Claude, AppId::Codex] {
        let result = f
            .client
            .app_provider_request_options_save(app, "fixture", &request(), TOKEN)
            .await
            .unwrap();
        assert!(result.headers_configured && result.overrides_configured);
        assert_eq!(result.application, RequestOptionsApplication::NotApplied);
        let public = serde_json::to_string(&result).unwrap();
        assert!(!public.contains("fake-sensitive") && !public.contains("Authorization"));
        let before: Value = serde_json::from_str(&f.row(app)).unwrap();
        if app == AppId::Claude {
            let body=serde_json::from_value(json!({"targetHostId":"host","fields":{"name":"再次原生保存","category":null,"notes":null,"websiteUrl":null},"config":{"kind":"third_party","baseUrl":"https://fixture.test","apiKeyHeader":false,"apiKey":{"action":"keep"}},"model":null,"apply":false})).unwrap();
            let result = f.client.claude_form_update("fixture", &body, TOKEN).await.unwrap();
            assert_eq!(result.live, ClaudeLiveResult::NotAttempted);
        } else {
            let mut profile: CodexProfile =
                serde_json::from_value(before["codex"].clone()).unwrap();
            profile.name = "再次原生保存".into();
            let result = f
                .client
                .codex_provider_save(
                    "fixture",
                    &CodexWrite {
                        target_host_id: "host".into(),
                        profile,
                        catalog: None,
                        activation: ClaudeActivation::CatalogOnly,
                    },
                    TOKEN,
                )
                .await
                .unwrap();
            assert_eq!(result.live, ClaudeLiveResult::NotAttempted);
        }
        let after: Value = serde_json::from_str(&f.row(app)).unwrap();
        assert_eq!(after["requestOptions"], before["requestOptions"]);
        assert_eq!(after["name"], "再次原生保存");
        assert_eq!(
            after["requestOptions"]["requestHeaders"]["authorization"],
            "fake-sensitive-header"
        );
        assert_eq!(
            f.client
                .app_provider_request_options_read(app, "fixture", &target(), TOKEN)
                .await
                .unwrap(),
            result
        );
    }
    assert!(!f.root.path().join(".claude/settings.json").exists());
    assert!(!f.root.path().join(".codex/config.toml").exists());
    let reopened = GlobalDatabase::for_home(f.root.path()).unwrap();
    assert!(
        AppProviderStore::new(reopened)
            .unwrap()
            .request_options_view(&tenant(), AppId::Claude, "fixture")
            .unwrap()
            .headers_configured
    );
}
#[tokio::test]
async fn cc04_ui_endpoints_options_permissions_validation_and_explicit_clear() {
    let f = Fixture::new().await;
    let before = f.row(AppId::Claude);
    assert!(
        f.client
            .app_provider_request_options_save(AppId::Claude, "fixture", &request(), "invalid")
            .await
            .is_err()
    );
    assert!(
        f.client
            .app_provider_request_options_read(AppId::Claude, "fixture", &target(), "invalid")
            .await
            .is_err()
    );
    let mut invalid = request();
    invalid.target_host_id = "another-host".into();
    assert!(
        f.client
            .app_provider_request_options_save(AppId::Claude, "fixture", &invalid, TOKEN)
            .await
            .is_err()
    );
    invalid = request();
    invalid
        .options
        .local_proxy_request_overrides
        .as_mut()
        .unwrap()
        .headers
        .as_mut()
        .unwrap()
        .insert("Authorization".into(), "secret".into());
    assert!(
        f.client
            .app_provider_request_options_save(AppId::Claude, "fixture", &invalid, TOKEN)
            .await
            .is_err()
    );
    assert_eq!(before, f.row(AppId::Claude));
    f.client
        .app_provider_request_options_save(AppId::Claude, "fixture", &request(), TOKEN)
        .await
        .unwrap();
    let clear: RequestOptionsWrite =
        serde_json::from_value(json!({"targetHostId":"host","options":{"requestHeaders":{}}}))
            .unwrap();
    let result = f
        .client
        .app_provider_request_options_save(AppId::Claude, "fixture", &clear, TOKEN)
        .await
        .unwrap();
    assert!(!result.headers_configured && result.overrides_configured);
    let clear: RequestOptionsWrite = serde_json::from_value(
        json!({"targetHostId":"host","options":{"localProxyRequestOverrides":{}}}),
    )
    .unwrap();
    let result = f
        .client
        .app_provider_request_options_save(AppId::Claude, "fixture", &clear, TOKEN)
        .await
        .unwrap();
    assert!(!result.headers_configured && !result.overrides_configured);
}

#[tokio::test]
async fn cc04_ui_endpoints_native_address_book_persists_full_url_and_server_time() {
    use vw_api_types::app_providers::endpoints::{
        AppProviderEndpointChange as Change, AppProviderEndpointKind as Kind,
    };
    let f = Fixture::new().await;
    let url = "https://fixture.test/v1/messages/".to_string();
    let rows = f
        .client
        .app_provider_endpoint_change(
            AppId::Claude,
            "fixture",
            &Change::Add { url: url.clone(), kind: Kind::FullUrl },
        )
        .await
        .unwrap();
    assert_eq!(rows[0].url, url);
    assert_eq!(rows[0].kind, Kind::FullUrl);
    assert!(rows[0].added_at > 0);
    assert_eq!(rows[0].last_used, None);
    let rows = f
        .client
        .app_provider_endpoint_change(
            AppId::Claude,
            "fixture",
            &Change::MarkUsed { url: url.clone(), kind: Kind::FullUrl },
        )
        .await
        .unwrap();
    assert!(rows[0].last_used.unwrap() >= rows[0].added_at);
    assert_eq!(f.client.app_provider_endpoints(AppId::Claude, "fixture").await.unwrap(), rows);
    let stored: Value = serde_json::from_str(&f.row(AppId::Claude)).unwrap();
    assert_eq!(stored["customEndpoints"], serde_json::to_value(&rows).unwrap());
    assert_eq!(stored["claude"]["config"]["api_key"], "fake-secret");
    let before = f.row(AppId::Claude);
    assert!(
        f.client
            .app_provider_endpoint_change(
                AppId::Claude,
                "fixture",
                &Change::Add {
                    url: "https://user:fake-secret@fixture.test".into(),
                    kind: Kind::BaseUrl
                }
            )
            .await
            .is_err()
    );
    assert_eq!(f.row(AppId::Claude), before);
    assert!(
        f.client
            .app_provider_endpoint_change(
                AppId::Claude,
                "fixture",
                &Change::Remove { url, kind: Kind::FullUrl }
            )
            .await
            .unwrap()
            .is_empty()
    );
}

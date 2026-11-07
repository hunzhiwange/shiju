use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{app_providers::opencode::*, id::TenantScope};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-opencode-token-with-at-least-32-chars";
struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    port: u16,
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
        std::fs::create_dir_all(root.path().join(".config/opencode")).unwrap();
        std::fs::write(
            root.path().join(".config/opencode/opencode.jsonc"),
            r#"{/*keep*/"provider":{"other":{"models":{"small":{}}}},"small_model":"other/small","mcp":{"fixture":{}}}"#,
        )
        .unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"opencode","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let router = super::opencode::router_with_policy(Some(policy.clone()))
            .layer(Extension(db.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            axum::serve(listener, Router::new().nest("/v1", router)).await.unwrap()
        });
        let client = GatewayClient::new(
            GatewayEndpoint::new("127.0.0.1", port)
                .with_tenant_scope(TenantScope::new("p", "a").unwrap()),
        )
        .unwrap();
        Self { root, db, port, client, server }
    }
    fn file(&self) -> PathBuf {
        self.root.path().join(".config/opencode/opencode.jsonc")
    }
    fn request(&self, activation: OpenCodeActivation) -> OpenCodeWrite {
        OpenCodeWrite {
            target_host_id: "host".into(),
            profile: OpenCodeProfile {
                name: "Fixture".into(),
                format: OpenCodeFormat::V1,
                declaration: json!({"npm":"@ai-sdk/openai-compatible","options":{"apiKey":"fake-secret"},"models":{"one":{}}}),
            },
            activation,
        }
    }
    fn target(&self) -> OpenCodeTarget {
        OpenCodeTarget { target_host_id: "host".into() }
    }
    fn record(&self, id: &str) -> Option<Value> {
        crate::app_providers::AppProviderStore::new(self.db.clone())
            .unwrap()
            .opencode_record(&TenantScope::new("p", "a").unwrap(), id)
            .unwrap()
    }
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.file()).unwrap()
    }
}
#[tokio::test]
async fn cc04_opencode_sdk_api_member_default_remove_and_catalog_delete() {
    let f = Fixture::new().await;
    assert_eq!(
        f.client
            .opencode_provider_save("one", &f.request(OpenCodeActivation::Add), TOKEN)
            .await
            .unwrap()
            .live,
        OpenCodeLiveResult::Applied
    );
    let mut request = f.request(OpenCodeActivation::IfMember);
    request.profile.declaration["options"]["apiKey"] = json!("fake-edited");
    f.client.opencode_provider_save("one", &request, TOKEN).await.unwrap();
    assert!(String::from_utf8_lossy(&f.bytes()).contains("fake-edited"));
    let model = OpenCodeDefaultModel { target_host_id: "host".into(), model: "one/one".into() };
    assert_eq!(
        f.client.opencode_default_model(&model, TOKEN).await.unwrap().live,
        OpenCodeLiveResult::Applied
    );
    let state = f.client.opencode_state(&f.target(), TOKEN).await.unwrap();
    assert_eq!(state.members, vec!["one", "other"]);
    assert_eq!(state.model.as_deref(), Some("one/one"));
    let bytes = f.bytes();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("/*keep*/"));
    assert!(text.contains("\"small_model\":\"other/small\""));
    let bad = OpenCodeDefaultModel { model: "one/missing".into(), ..model };
    assert!(f.client.opencode_default_model(&bad, TOKEN).await.is_err());
    assert_eq!(f.bytes(), bytes);
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    assert!(
        store
            .delete(&tenant, vw_api_types::agent_apps::AppId::OpenCode, "one", f.root.path())
            .is_err()
    );
    assert_eq!(
        f.client.opencode_provider_remove_live("one", &f.target(), TOKEN).await.unwrap().live,
        OpenCodeLiveResult::Applied
    );
    assert!(f.record("one").is_some());
    assert_eq!(f.client.opencode_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    store.delete(&tenant, vw_api_types::agent_apps::AppId::OpenCode, "one", f.root.path()).unwrap();
    assert!(f.record("one").is_none());
    let state: Value = serde_json::from_slice(
        &std::fs::read(f.root.path().join("private/provider-live/aG9zdA/opencode.state.json"))
            .unwrap(),
    )
    .unwrap();
    assert!(state["pointer"].is_null());
}
#[tokio::test]
async fn cc04_opencode_import_is_read_only_and_does_not_overwrite_catalog() {
    let f = Fixture::new().await;
    let before = f.bytes();
    let mut permissions = std::fs::metadata(f.file()).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(f.file(), permissions).unwrap();
    assert_eq!(f.client.opencode_import(&f.target(), TOKEN).await.unwrap().imported, 1);
    assert_eq!(f.client.opencode_import(&f.target(), TOKEN).await.unwrap().imported, 0);
    assert_eq!(f.bytes(), before);
    assert!(f.record("other").is_some());
    assert!(!f.root.path().join("private/provider-live").exists());
    f.client
        .opencode_provider_save("draft", &f.request(OpenCodeActivation::CatalogOnly), TOKEN)
        .await
        .unwrap();
    assert_eq!(f.bytes(), before);
}
#[tokio::test]
async fn cc04_opencode_permission_tenant_and_host_denials_leave_no_changes() {
    let f = Fixture::new().await;
    let before = f.bytes();
    assert!(
        f.client
            .opencode_provider_save("one", &f.request(OpenCodeActivation::Add), "bad-token")
            .await
            .is_err()
    );
    let mut bad = f.request(OpenCodeActivation::Add);
    bad.target_host_id = "foreign".into();
    assert!(f.client.opencode_provider_save("one", &bad, TOKEN).await.is_err());
    let foreign = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", f.port)
            .with_tenant_scope(TenantScope::new("p", "b").unwrap()),
    )
    .unwrap();
    assert!(
        foreign
            .opencode_provider_save("one", &f.request(OpenCodeActivation::Add), TOKEN)
            .await
            .is_err()
    );
    let path = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(path, policy.to_string()).unwrap();
    assert!(
        f.client
            .opencode_provider_save("one", &f.request(OpenCodeActivation::Add), TOKEN)
            .await
            .is_err()
    );
    assert_eq!(f.client.opencode_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    assert_eq!(f.bytes(), before);
    assert!(f.record("one").is_none());
    assert!(!f.root.path().join("private/provider-live").exists());
}
#[tokio::test]
async fn cc04_opencode_partial_builtin_cannot_reactivate_after_removal() {
    let f = Fixture::new().await;
    f.client.opencode_import(&f.target(), TOKEN).await.unwrap();
    let before = f.bytes();
    f.client.opencode_provider_add("other", &f.target(), TOKEN).await.unwrap();
    f.client.opencode_provider_remove_live("other", &f.target(), TOKEN).await.unwrap();
    let removed = f.bytes();
    assert!(f.client.opencode_provider_add("other", &f.target(), TOKEN).await.is_err());
    assert_eq!(f.bytes(), removed);
    assert!(f.record("other").is_some());
    assert_ne!(before, removed);
}

fn form(declaration: &str, activation: OpenCodeActivation) -> OpenCodeFormWrite {
    OpenCodeFormWrite {
        target_host_id: "host".into(),
        fields: vw_api_types::app_providers::basic::ProviderBasic {
            name: "中文节点".into(),
            category: Some("隔离测试".into()),
            notes: Some("中文备注".into()),
            website_url: None,
        },
        format: OpenCodeFormat::V1,
        declaration: declaration.into(),
        api_key: vw_api_types::app_providers::create::ProviderSecretWrite::Keep,
        activation,
        revision: None,
    }
}

#[tokio::test]
async fn cc04_ui_opencode_form_updates_native_node_and_preserves_protected_values() {
    use vw_api_types::app_providers::create::ProviderSecretWrite;
    let f = Fixture::new().await;
    let mut body = form(
        r#"{"npm":"@ai-sdk/openai-compatible","options":{"baseURL":"https://example.test/v1","headers":{"Authorization":"fake-hidden-header"}},"models":{"one":{"name":"旧模型"}}}"#,
        OpenCodeActivation::Add,
    );
    body.api_key = ProviderSecretWrite::Replace { value: "fake-hidden-key".into() };
    assert_eq!(
        f.client.opencode_form_create("one", &body, TOKEN).await.unwrap().live,
        OpenCodeLiveResult::Applied
    );
    let initial = f.bytes();
    assert!(f.client.opencode_form_create("one", &body, TOKEN).await.is_err());
    assert_eq!(f.bytes(), initial);
    assert!(f.client.opencode_form_read("one", &f.target(), "bad-token").await.is_err());
    let view = f.client.opencode_form_read("one", &f.target(), TOKEN).await.unwrap();
    let config = view.config.unwrap();
    assert!(config.has_saved_key);
    assert!(config.declaration.contains("保留执行端值"));
    assert!(!config.declaration.contains("fake-hidden"));
    let mut redirected = form(
        &config.declaration.replace("https://example.test/v1", "https://other.test/v1"),
        OpenCodeActivation::IfMember,
    );
    redirected.revision = Some(config.revision.clone());
    assert!(matches!(
        f.client.opencode_form_update("one", &redirected, TOKEN).await.unwrap_err(),
        vw_gateway_client::AgentAppsReadError::Http { status: 422, .. }
    ));
    assert_eq!(f.bytes(), initial);
    let mut edit =
        form(&config.declaration.replace("旧模型", "新模型"), OpenCodeActivation::IfMember);
    edit.revision = Some(config.revision);
    edit.fields = view.fields;
    edit.fields.notes = Some("新备注".into());
    assert_eq!(
        f.client.opencode_form_update("one", &edit, TOKEN).await.unwrap().live,
        OpenCodeLiveResult::Applied
    );
    let bytes = f.bytes();
    let native = std::str::from_utf8(&bytes).unwrap();
    for expected in ["新模型", "/*keep*/", "fake-hidden-key", "fake-hidden-header", "other/small"]
    {
        assert!(native.contains(expected));
    }
    let row = f.record("one").unwrap();
    assert_eq!(
        vw_agent_apps::providers::basic::read(&row).unwrap().notes.as_deref(),
        Some("新备注")
    );
    assert!(matches!(
        f.client.opencode_form_update("one", &edit, TOKEN).await.unwrap_err(),
        vw_gateway_client::AgentAppsReadError::Http { status: 409, .. }
    ));
    assert_eq!(f.bytes(), bytes);
    assert_eq!(f.record("one"), Some(row));
    assert_eq!(
        f.client.opencode_provider_remove_live("one", &f.target(), TOKEN).await.unwrap().live,
        OpenCodeLiveResult::Applied
    );
    assert!(f.record("one").is_some());
    assert_eq!(
        f.client.opencode_provider_add("one", &f.target(), TOKEN).await.unwrap().live,
        OpenCodeLiveResult::Applied
    );
    assert_eq!(
        f.client.opencode_state(&f.target(), TOKEN).await.unwrap().members,
        vec!["one", "other"]
    );
}

#[tokio::test]
async fn cc04_ui_opencode_form_rejects_invalid_node_and_write_permission_without_effects() {
    let f = Fixture::new().await;
    let before = f.bytes();
    let bad = form(r#"{"models":{"one":false}}"#, OpenCodeActivation::Add);
    assert!(matches!(
        f.client.opencode_form_create("one", &bad, TOKEN).await.unwrap_err(),
        vw_gateway_client::AgentAppsReadError::Http { status: 422, .. }
    ));
    let path = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(path, policy.to_string()).unwrap();
    let valid =
        form(r#"{"npm":"@ai-sdk/openai-compatible","models":{"one":{}}}"#, OpenCodeActivation::Add);
    assert!(matches!(
        f.client.opencode_form_create("one", &valid, TOKEN).await.unwrap_err(),
        vw_gateway_client::AgentAppsReadError::Http { status: 403, .. }
    ));
    assert_eq!(f.bytes(), before);
    assert!(f.record("one").is_none());
    assert!(!f.root.path().join("private/provider-live").exists());
}

#[tokio::test]
async fn cc04_ui_opencode_v2_edit_stays_in_catalog_until_explicit_add() {
    let f = Fixture::new().await;
    let initial = f.bytes();
    let mut body = form(
        r#"{"package":"@opencode/example","models":{"model/id":{"name":"中文模型","limit":{"context":4096}}}}"#,
        OpenCodeActivation::CatalogOnly,
    );
    body.format = OpenCodeFormat::V2;
    assert_eq!(
        f.client.opencode_form_create("native", &body, TOKEN).await.unwrap().live,
        OpenCodeLiveResult::NotAttempted
    );
    let config =
        f.client.opencode_form_read("native", &f.target(), TOKEN).await.unwrap().config.unwrap();
    assert_eq!(config.format, OpenCodeFormat::V2);
    body.declaration = config.declaration.replace("4096", "8192");
    body.revision = Some(config.revision);
    body.activation = OpenCodeActivation::IfMember;
    assert_eq!(
        f.client.opencode_form_update("native", &body, TOKEN).await.unwrap().live,
        OpenCodeLiveResult::NotAttempted
    );
    assert_eq!(f.bytes(), initial);
    assert_eq!(
        f.client.opencode_provider_add("native", &f.target(), TOKEN).await.unwrap().live,
        OpenCodeLiveResult::Applied
    );
    let bytes = f.bytes();
    let native = std::str::from_utf8(&bytes).unwrap();
    for expected in ["\"providers\"", "\"native\"", "8192", "中文模型", "/*keep*/", "other/small"]
    {
        assert!(native.contains(expected));
    }
}

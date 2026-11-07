use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{app_providers::hermes::*, id::TenantScope};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-hermes-token-with-at-least-32-chars";
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
        std::fs::create_dir_all(root.path().join(".hermes")).unwrap();
        std::fs::write(
            root.path().join(".hermes/config.yaml"),
            "# keep\nagent: {max_turns: 50}\ncustom_providers:\n  - name: other\n    models: {small: {}}\n",
        )
        .unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"hermes","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let router = super::hermes::router_with_policy(Some(policy.clone()))
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
        self.root.path().join(".hermes/config.yaml")
    }
    fn request(&self, activation: HermesActivation) -> HermesWrite {
        HermesWrite {
            target_host_id: "host".into(),
            profile: HermesProfile {
                name: "Fixture".into(),
                declaration: json!({"api_mode":"chat_completions","api_key":"fake-secret","models":[{"id":"one"}]}),
            },
            activation,
        }
    }
    fn target(&self) -> HermesTarget {
        HermesTarget { target_host_id: "host".into() }
    }
    fn record(&self, id: &str) -> Option<Value> {
        crate::app_providers::AppProviderStore::new(self.db.clone())
            .unwrap()
            .hermes_record(&TenantScope::new("p", "a").unwrap(), id)
            .unwrap()
    }
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.file()).unwrap()
    }
}
#[tokio::test]
async fn cc04_hermes_sdk_api_member_edit_remove_and_catalog_delete() {
    let f = Fixture::new().await;
    assert_eq!(
        f.client
            .hermes_provider_save("one", &f.request(HermesActivation::Add), TOKEN)
            .await
            .unwrap()
            .live,
        HermesLiveResult::Applied
    );
    assert_eq!(
        f.client.hermes_provider_remove_live("one", &f.target(), TOKEN).await.unwrap().live,
        HermesLiveResult::Applied
    );
    assert_eq!(f.client.hermes_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    assert_eq!(
        f.client.hermes_provider_add("one", &f.target(), TOKEN).await.unwrap().live,
        HermesLiveResult::Applied
    );
    let mut request = f.request(HermesActivation::IfMember);
    request.profile.declaration["api_key"] = json!("fake-edited");
    f.client.hermes_provider_save("one", &request, TOKEN).await.unwrap();
    assert!(String::from_utf8_lossy(&f.bytes()).contains("fake-edited"));
    let state = f.client.hermes_state(&f.target(), TOKEN).await.unwrap();
    assert_eq!(state.members, vec!["one", "other"]);
    assert!(String::from_utf8_lossy(&f.bytes()).contains("# keep\nagent: {max_turns: 50}\n"));
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    assert!(
        store
            .delete(&tenant, vw_api_types::agent_apps::AppId::Hermes, "one", f.root.path())
            .is_err()
    );
    assert_eq!(
        f.client.hermes_provider_remove_live("one", &f.target(), TOKEN).await.unwrap().live,
        HermesLiveResult::Applied
    );
    assert!(f.record("one").is_some());
    assert_eq!(f.client.hermes_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    store.delete(&tenant, vw_api_types::agent_apps::AppId::Hermes, "one", f.root.path()).unwrap();
    assert!(f.record("one").is_none());
    let state: Value = serde_json::from_slice(
        &std::fs::read(f.root.path().join("private/provider-live/aG9zdA/hermes.state.json"))
            .unwrap(),
    )
    .unwrap();
    assert!(state["pointer"].is_null());
}
#[tokio::test]
async fn cc04_hermes_import_is_read_only_and_does_not_overwrite_catalog() {
    let f = Fixture::new().await;
    let before = f.bytes();
    let mut permissions = std::fs::metadata(f.file()).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(f.file(), permissions).unwrap();
    assert_eq!(f.client.hermes_import(&f.target(), TOKEN).await.unwrap().imported, 1);
    assert_eq!(f.client.hermes_import(&f.target(), TOKEN).await.unwrap().imported, 0);
    assert_eq!(f.bytes(), before);
    assert!(f.record("other").is_some());
    assert!(!f.root.path().join("private/provider-live").exists());
    f.client
        .hermes_provider_save("draft", &f.request(HermesActivation::CatalogOnly), TOKEN)
        .await
        .unwrap();
    assert_eq!(f.bytes(), before);
}
#[tokio::test]
async fn cc04_hermes_permission_tenant_and_host_denials_leave_no_changes() {
    let f = Fixture::new().await;
    let before = f.bytes();
    assert!(
        f.client
            .hermes_provider_save("one", &f.request(HermesActivation::Add), "bad-token")
            .await
            .is_err()
    );
    let mut bad = f.request(HermesActivation::Add);
    bad.target_host_id = "foreign".into();
    assert!(f.client.hermes_provider_save("one", &bad, TOKEN).await.is_err());
    let foreign = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", f.port)
            .with_tenant_scope(TenantScope::new("p", "b").unwrap()),
    )
    .unwrap();
    assert!(
        foreign
            .hermes_provider_save("one", &f.request(HermesActivation::Add), TOKEN)
            .await
            .is_err()
    );
    let path = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(path, policy.to_string()).unwrap();
    assert!(
        f.client
            .hermes_provider_save("one", &f.request(HermesActivation::Add), TOKEN)
            .await
            .is_err()
    );
    assert_eq!(f.client.hermes_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    assert_eq!(f.bytes(), before);
    assert!(f.record("one").is_none());
    assert!(!f.root.path().join("private/provider-live").exists());
}

#[tokio::test]
async fn cc04_hermes_imported_overlay_cannot_be_edited_or_removed() {
    let f = Fixture::new().await;
    std::fs::write(f.file(), "# overlay\nproviders:\n  native: {name: overlay, api_key: fake-overlay, key_env: FIXTURE_KEY, models: {m: {}}}\n").unwrap();
    let bytes = f.bytes();
    assert_eq!(f.client.hermes_import(&f.target(), TOKEN).await.unwrap().imported, 1);
    assert_eq!(
        f.client.hermes_state(&f.target(), TOKEN).await.unwrap().read_only_members,
        vec!["overlay"]
    );
    let record = f.record("overlay").unwrap();
    assert_eq!(record["hermes"]["declaration"]["key_env"], "FIXTURE_KEY");
    assert!(
        f.client
            .hermes_provider_save("overlay", &f.request(HermesActivation::IfMember), TOKEN)
            .await
            .is_err()
    );
    assert!(f.client.hermes_provider_remove_live("overlay", &f.target(), TOKEN).await.is_err());
    assert!(f.client.hermes_provider_add("native", &f.target(), TOKEN).await.is_err());
    assert_eq!(f.record("overlay").unwrap(), record);
    assert_eq!(f.bytes(), bytes);
}
#[tokio::test]
async fn cc04_hermes_partial_catalog_edit_preserves_credentials_and_future_fields() {
    let f = Fixture::new().await;
    let mut request = f.request(HermesActivation::CatalogOnly);
    request.profile.declaration["key_env"] = json!("FIXTURE_KEY");
    request.profile.declaration["future"] = json!({"keep":true});
    f.client.hermes_provider_save("one", &request, TOKEN).await.unwrap();
    request.profile.declaration = json!({"apiKey":"fake-new"});
    f.client.hermes_provider_save("one", &request, TOKEN).await.unwrap();
    let record = f.record("one").unwrap();
    assert_eq!(record["hermes"]["declaration"]["api_key"], "fake-new");
    assert_eq!(record["hermes"]["declaration"]["key_env"], "FIXTURE_KEY");
    assert_eq!(record["hermes"]["declaration"]["future"], json!({"keep":true}));
    f.client.hermes_provider_add("one", &f.target(), TOKEN).await.unwrap();
    let doc: Value = serde_yaml::from_slice(&f.bytes()).unwrap();
    let row =
        doc["custom_providers"].as_array().unwrap().iter().find(|v| v["name"] == "one").unwrap();
    assert_eq!(row["api_key"], "fake-new");
    assert_eq!(row["key_env"], "FIXTURE_KEY");
    assert_eq!(row["future"], json!({"keep":true}));
}

fn editor_request(activation: HermesActivation) -> HermesFormWrite {
    use vw_api_types::app_providers::{basic::ProviderBasic, create::ProviderSecretWrite};
    HermesFormWrite {
        target_host_id: "host".into(),
        fields: ProviderBasic {
            name: "中文供应商".into(),
            category: Some("工作".into()),
            notes: Some("中文动态备注".into()),
            website_url: None,
        },
        declaration: json!({
            "base_url":"https://fixture.example.test/v1",
            "api_mode":"chat_completions",
            "models":[{"id":"first", "name":"中文模型","context_length":8192,"supports_vision":true}],
            "rate_limit_delay":0.5
        }).to_string(),
        api_key: ProviderSecretWrite::Replace { value: "fake-form-key".into() },
        activation,
        revision: None,
    }
}

#[tokio::test]
async fn cc04_ui_hermes_sdk_form_edits_one_native_member_and_keeps_secrets() {
    use vw_api_types::app_providers::create::ProviderSecretWrite;
    let f = Fixture::new().await;
    let mut req = editor_request(HermesActivation::Add);
    let result = f.client.hermes_form_create("native", &req, TOKEN).await.unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, HermesLiveResult::Applied);
    let mut native = f.request(HermesActivation::IfMember);
    native.profile.declaration = json!({
        "base_url":"https://fixture.example.test/v1", "api_mode":"chat_completions",
        "api_key":"fake-form-key","key_env":"FIXTURE_KEY", "future":"opaque",
        "models":{"first":{"name":"中文模型","extra":"fake-private-model-field"}}
    });
    f.client.hermes_provider_save("native", &native, TOKEN).await.unwrap();
    let view = f.client.hermes_form_read("native", &f.target(), TOKEN).await.unwrap();
    assert_eq!(view.fields.notes.as_deref(), Some("中文动态备注"));
    let config = view.config.unwrap();
    assert!(config.has_saved_key);
    for concealed in ["fake-form-key", "FIXTURE_KEY", "fake-private-model-field", "opaque"] {
        assert!(!config.declaration.contains(concealed));
    }
    req.activation = HermesActivation::IfMember;
    req.api_key = ProviderSecretWrite::Keep;
    req.revision = Some(config.revision);
    req.fields.notes = Some("已编辑".into());
    let result = f.client.hermes_form_update("native", &req, TOKEN).await.unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, HermesLiveResult::Applied);
    let file = String::from_utf8(f.bytes()).unwrap();
    assert!(file.starts_with("# keep\nagent: {max_turns: 50}\ncustom_providers:\n  - name: other\n    models: {small: {}}\n"));
    for expected in [
        "fake-form-key",
        "FIXTURE_KEY",
        "fake-private-model-field",
        "opaque",
        "context_length: 8192",
        "rate_limit_delay: 0.5",
        "supports_vision: true",
    ] {
        assert!(file.contains(expected), "{expected}");
    }
    assert_eq!(f.record("native").unwrap()["notes"], "已编辑");
    let before = f.bytes();
    assert!(matches!(
        f.client.hermes_form_update("native", &req, TOKEN).await,
        Err(vw_gateway_client::AgentAppsReadError::Http { status: 409, .. })
    ));
    assert_eq!(f.bytes(), before);

    assert_eq!(
        f.client.hermes_provider_remove_live("native", &f.target(), TOKEN).await.unwrap().live,
        HermesLiveResult::Applied
    );
    assert!(f.record("native").is_some());
    assert_eq!(
        f.client.hermes_provider_add("native", &f.target(), TOKEN).await.unwrap().live,
        HermesLiveResult::Applied
    );
    assert_eq!(
        f.client.hermes_state(&f.target(), TOKEN).await.unwrap().members,
        vec!["native", "other"]
    );
}

#[tokio::test]
async fn cc04_ui_hermes_sdk_form_rejects_invalid_and_unauthorized_writes() {
    let f = Fixture::new().await;
    let mut req = editor_request(HermesActivation::Add);
    let before = f.bytes();
    assert!(f.client.hermes_form_create("native", &req, "denied").await.is_err());
    req.declaration = r#"{"models":[{"id":"","name":"中文模型"}]}"#.into();
    assert!(matches!(
        f.client.hermes_form_create("native", &req, TOKEN).await,
        Err(vw_gateway_client::AgentAppsReadError::Http { status: 422, .. })
    ));
    assert!(f.record("native").is_none());
    assert_eq!(f.bytes(), before);
}

#[tokio::test]
async fn cc04_ui_hermes_sdk_can_edit_an_unimported_native_member() {
    use vw_api_types::app_providers::create::ProviderSecretWrite;
    let f = Fixture::new().await;
    assert!(f.record("other").is_none());
    let view = f.client.hermes_form_read("other", &f.target(), TOKEN).await.unwrap();
    assert_eq!(view.fields.name, "other");
    let mut req = editor_request(HermesActivation::IfMember);
    req.api_key = ProviderSecretWrite::Keep;
    req.revision = Some(view.config.unwrap().revision);
    let changed = String::from_utf8(f.bytes()).unwrap().replace("small", "external");
    std::fs::write(f.file(), &changed).unwrap();
    assert!(matches!(
        f.client.hermes_form_update("other", &req, TOKEN).await,
        Err(vw_gateway_client::AgentAppsReadError::Http { status: 409, .. })
    ));
    assert_eq!(f.bytes(), changed.as_bytes());
    assert!(f.record("other").is_none());

    let view = f.client.hermes_form_read("other", &f.target(), TOKEN).await.unwrap();
    req.revision = Some(view.config.unwrap().revision);
    let result = f.client.hermes_form_update("other", &req, TOKEN).await.unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, HermesLiveResult::Applied);
    assert_eq!(f.client.hermes_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    assert!(String::from_utf8(f.bytes()).unwrap().contains("中文模型"));
}

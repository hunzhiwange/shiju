use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{app_providers::openclaw::*, id::TenantScope};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-openclaw-token-with-at-least-32-chars";
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
        std::fs::create_dir_all(root.path().join(".openclaw")).unwrap();
        std::fs::write(
            root.path().join(".openclaw/openclaw.json"),
            r#"{/*keep*/"models":{"providers":{"other":{"models":[{"id":"small"}]}}},"agents":{"defaults":{"model":{"primary":"other/small","fallbacks":["one/one","other/small"],"future":true},"workspace":"keep"}},"memory":{"fixture":{}}}"#,
        )
        .unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"openclaw","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let router = super::openclaw::router_with_policy(Some(policy.clone()))
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
        self.root.path().join(".openclaw/openclaw.json")
    }
    fn request(&self, activation: OpenClawActivation) -> OpenClawWrite {
        OpenClawWrite {
            target_host_id: "host".into(),
            profile: OpenClawProfile {
                name: "Fixture".into(),
                declaration: json!({"api":"openai-completions","apiKey":"fake-secret","models":[{"id":"one"}]}),
            },
            activation,
        }
    }
    fn target(&self) -> OpenClawTarget {
        OpenClawTarget { target_host_id: "host".into() }
    }
    fn record(&self, id: &str) -> Option<Value> {
        crate::app_providers::AppProviderStore::new(self.db.clone())
            .unwrap()
            .openclaw_record(&TenantScope::new("p", "a").unwrap(), id)
            .unwrap()
    }
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.file()).unwrap()
    }
}
#[tokio::test]
async fn cc04_openclaw_sdk_api_member_default_remove_and_catalog_delete() {
    let f = Fixture::new().await;
    assert_eq!(
        f.client
            .openclaw_provider_save("one", &f.request(OpenClawActivation::Add), TOKEN)
            .await
            .unwrap()
            .live,
        OpenClawLiveResult::Applied
    );
    assert_eq!(
        f.client.openclaw_provider_remove_live("one", &f.target(), TOKEN).await.unwrap().live,
        OpenClawLiveResult::Applied
    );
    assert_eq!(f.client.openclaw_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    assert_eq!(
        f.client.openclaw_provider_add("one", &f.target(), TOKEN).await.unwrap().live,
        OpenClawLiveResult::Applied
    );
    let mut request = f.request(OpenClawActivation::IfMember);
    request.profile.declaration["apiKey"] = json!("fake-edited");
    f.client.openclaw_provider_save("one", &request, TOKEN).await.unwrap();
    assert!(String::from_utf8_lossy(&f.bytes()).contains("fake-edited"));
    let model = OpenClawDefaultModel { target_host_id: "host".into(), model: "one/one".into() };
    assert_eq!(
        f.client.openclaw_default_model(&model, TOKEN).await.unwrap().live,
        OpenClawLiveResult::Applied
    );
    let state = f.client.openclaw_state(&f.target(), TOKEN).await.unwrap();
    assert_eq!(state.members, vec!["one", "other"]);
    assert_eq!(state.model.as_deref(), Some("one/one"));
    let bytes = f.bytes();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("/*keep*/"));
    assert!(text.contains("\"workspace\":\"keep\""));
    assert!(text.contains("\"fallbacks\":[\"other/small\"]"));
    let bad = OpenClawDefaultModel { model: "one/missing".into(), ..model };
    assert!(f.client.openclaw_default_model(&bad, TOKEN).await.is_err());
    assert_eq!(f.bytes(), bytes);
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    assert!(
        store
            .delete(&tenant, vw_api_types::agent_apps::AppId::OpenClaw, "one", f.root.path())
            .is_err()
    );
    assert_eq!(
        f.client.openclaw_provider_remove_live("one", &f.target(), TOKEN).await.unwrap().live,
        OpenClawLiveResult::Applied
    );
    assert!(f.record("one").is_some());
    assert_eq!(f.client.openclaw_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    store.delete(&tenant, vw_api_types::agent_apps::AppId::OpenClaw, "one", f.root.path()).unwrap();
    assert!(f.record("one").is_none());
    let state: Value = serde_json::from_slice(
        &std::fs::read(f.root.path().join("private/provider-live/aG9zdA/openclaw.state.json"))
            .unwrap(),
    )
    .unwrap();
    assert!(state["pointer"].is_null());
}
#[tokio::test]
async fn cc04_openclaw_import_is_read_only_and_does_not_overwrite_catalog() {
    let f = Fixture::new().await;
    let before = f.bytes();
    let mut permissions = std::fs::metadata(f.file()).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(f.file(), permissions).unwrap();
    assert_eq!(f.client.openclaw_import(&f.target(), TOKEN).await.unwrap().imported, 1);
    assert_eq!(f.client.openclaw_import(&f.target(), TOKEN).await.unwrap().imported, 0);
    assert_eq!(f.bytes(), before);
    assert!(f.record("other").is_some());
    assert!(!f.root.path().join("private/provider-live").exists());
    f.client
        .openclaw_provider_save("draft", &f.request(OpenClawActivation::CatalogOnly), TOKEN)
        .await
        .unwrap();
    assert_eq!(f.bytes(), before);
}
#[tokio::test]
async fn cc04_openclaw_permission_tenant_and_host_denials_leave_no_changes() {
    let f = Fixture::new().await;
    let before = f.bytes();
    assert!(
        f.client
            .openclaw_provider_save("one", &f.request(OpenClawActivation::Add), "bad-token")
            .await
            .is_err()
    );
    let mut bad = f.request(OpenClawActivation::Add);
    bad.target_host_id = "foreign".into();
    assert!(f.client.openclaw_provider_save("one", &bad, TOKEN).await.is_err());
    let foreign = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", f.port)
            .with_tenant_scope(TenantScope::new("p", "b").unwrap()),
    )
    .unwrap();
    assert!(
        foreign
            .openclaw_provider_save("one", &f.request(OpenClawActivation::Add), TOKEN)
            .await
            .is_err()
    );
    let path = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(path, policy.to_string()).unwrap();
    assert!(
        f.client
            .openclaw_provider_save("one", &f.request(OpenClawActivation::Add), TOKEN)
            .await
            .is_err()
    );
    assert_eq!(f.client.openclaw_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    assert_eq!(f.bytes(), before);
    assert!(f.record("one").is_none());
    assert!(!f.root.path().join("private/provider-live").exists());
}

#[tokio::test]
async fn cc04_ui_openclaw_sdk_edits_only_the_named_member_and_preserves_secrets() {
    use vw_api_types::app_providers::{basic::ProviderBasic, create::ProviderSecretWrite};
    let f = Fixture::new().await;
    let mut create = OpenClawFormWrite {
        target_host_id: "host".into(),
        fields: ProviderBasic {
            name: "中文档案".into(),
            category: Some("工作".into()),
            notes: Some("中文备注".into()),
            website_url: Some("https://website.example.test".into()),
        },
        declaration: json!({
            "baseUrl":"https://api.example.test",
            "api":"openai-completions",
            "models":[{"id":"one","name":"中文模型","input":["text"],"contextWindow":8192}]
        })
        .to_string(),
        api_key: ProviderSecretWrite::Replace { value: "fake-new-key".into() },
        user_agent: true,
        activation: OpenClawActivation::Add,
        revision: None,
    };
    let first = f.client.openclaw_form_create("one", &create, TOKEN).await.unwrap();
    assert!(first.catalog_saved);
    assert_eq!(first.live, OpenClawLiveResult::Applied);
    let before = f.bytes();
    let mut native = f.request(OpenClawActivation::IfMember);
    native.profile.name.clone_from(&create.fields.name);
    native.profile.declaration = serde_json::from_str(&create.declaration).unwrap();
    native.profile.declaration["apiKey"] = json!("fake-new-key");
    native.profile.declaration["headers"] =
        json!({"uSeR-aGeNt":"fixture-custom-agent","Authorization":"fake-private-header"});
    native.profile.declaration["models"][0]["private"] = json!("fake-private-model-value");
    native.profile.declaration["models"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"two","name":"另一个模型"}));
    f.client.openclaw_provider_save("one", &native, TOKEN).await.unwrap();
    let view = f.client.openclaw_form_read("one", &f.target(), TOKEN).await.unwrap();
    assert_eq!(view.fields, create.fields);
    let config = view.config.unwrap();
    assert!(config.has_saved_key && config.user_agent);
    assert!(!config.declaration.contains("fake-"));
    assert!(!config.declaration.contains("headers"));
    assert!(!config.declaration.contains("apiKey"));
    let mut node: Value = serde_json::from_str(&config.declaration).unwrap();
    node["models"][0]["name"] = json!("更新的中文模型");
    node["models"].as_array_mut().unwrap().swap(0, 1);
    create.fields.name = "更新后的档案".into();
    create.declaration = node.to_string();
    create.api_key = ProviderSecretWrite::Keep;
    create.user_agent = false;
    create.activation = OpenClawActivation::IfMember;
    create.revision = Some(config.revision);
    let saved = f.client.openclaw_form_update("one", &create, TOKEN).await.unwrap();
    assert!(saved.catalog_saved);
    assert_eq!(saved.live, OpenClawLiveResult::Applied);
    let stored = f.record("one").unwrap();
    let node = &stored["openclaw"]["declaration"];
    assert_eq!(node["models"][1]["name"], "更新的中文模型");
    assert_eq!(node["models"][1]["private"], "fake-private-model-value");
    assert_eq!(node["apiKey"], "fake-new-key");
    assert_eq!(node["headers"], json!({"Authorization":"fake-private-header"}));
    let after = f.bytes();
    assert_ne!(before, after);
    let live = vw_agent_apps::providers::openclaw::declarations(Some(&after)).unwrap();
    assert_eq!(&live["one"].declaration, node);
    assert_eq!(live["other"].declaration, json!({"models":[{"id":"small"}]}));
    let rendered = String::from_utf8(after).unwrap();
    assert!(rendered.contains("/*keep*/"));
    assert!(rendered.contains("\"workspace\":\"keep\""));

    assert_eq!(
        f.client.openclaw_provider_remove_live("one", &f.target(), TOKEN).await.unwrap().live,
        OpenClawLiveResult::Applied
    );
    assert!(f.record("one").is_some());
    assert_eq!(f.client.openclaw_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    assert_eq!(
        f.client.openclaw_provider_add("one", &f.target(), TOKEN).await.unwrap().live,
        OpenClawLiveResult::Applied
    );
    assert_eq!(
        vw_agent_apps::providers::openclaw::declarations(Some(&f.bytes())).unwrap()["one"]
            .declaration,
        *node
    );
}

#[tokio::test]
async fn cc04_ui_openclaw_sdk_rejects_invalid_stale_and_unauthorized_writes() {
    use vw_api_types::app_providers::create::ProviderSecretWrite;
    use vw_gateway_client::AgentAppsReadError;
    let f = Fixture::new().await;
    f.client
        .openclaw_provider_save("one", &f.request(OpenClawActivation::Add), TOKEN)
        .await
        .unwrap();
    let original = f.record("one").unwrap();
    let before = f.bytes();
    let view = f.client.openclaw_form_read("one", &f.target(), TOKEN).await.unwrap();
    let config = view.config.unwrap();
    let mut form = OpenClawFormWrite {
        target_host_id: "host".into(),
        fields: view.fields,
        declaration: r#"{"models":[{"id":""}]}"#.into(),
        api_key: ProviderSecretWrite::Keep,
        user_agent: false,
        activation: OpenClawActivation::IfMember,
        revision: Some(config.revision),
    };
    assert!(matches!(
        f.client.openclaw_form_update("one", &form, TOKEN).await,
        Err(AgentAppsReadError::Http { status: 422, .. })
    ));
    form.declaration =
        r#"{"api":"openai-completions","baseUrl":"https://foreign.example.test","models":[]}"#
            .into();
    assert!(matches!(
        f.client.openclaw_form_update("one", &form, TOKEN).await,
        Err(AgentAppsReadError::Http { status: 422, .. })
    ));
    form.declaration = config.declaration;
    assert!(matches!(
        f.client.openclaw_form_update("one", &form, "denied").await,
        Err(AgentAppsReadError::Http { status: 401 | 403, .. })
    ));
    assert!(f.client.openclaw_form_read("one", &f.target(), "denied").await.is_err());
    let stale = OpenClawFormWrite { revision: Some("stale".into()), ..form.clone() };
    assert!(matches!(
        f.client.openclaw_form_update("one", &stale, TOKEN).await,
        Err(AgentAppsReadError::Http { status: 409, .. })
    ));
    assert_eq!(f.record("one").unwrap(), original);
    assert_eq!(f.bytes(), before);

    form.activation = OpenClawActivation::CatalogOnly;
    form.api_key = ProviderSecretWrite::Clear;
    let saved = f.client.openclaw_form_update("one", &form, TOKEN).await.unwrap();
    assert!(saved.catalog_saved);
    assert_eq!(saved.live, OpenClawLiveResult::NotAttempted);
    assert!(f.record("one").unwrap().pointer("/openclaw/declaration/apiKey").is_none());
    assert_eq!(f.bytes(), before);
}

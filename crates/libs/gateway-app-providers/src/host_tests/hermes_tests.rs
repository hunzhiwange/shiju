use crate::{global_database::GlobalDatabase, security::pairing::PairingGuard};
use axum::{Extension, Router, extract::Request, middleware, response::IntoResponse};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use gateway_app_provider::{AgentAppsReadError, GatewayAppProviderClient};
use gateway_endpoint::{GatewayAuth, GatewayEndpoint};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, time::Duration};
use vw_api_app_provider_extended::hermes::*;
use vw_api_transport::id::TenantScope;

const SKEY: &str = "fixture-hermes-gateway-skey";
const CHILD: &str = "SHIJU_HERMES_HOST_TEST_ROOT";

// Gateway execution paths are initialized once per process.
async fn isolated(test: &str) -> bool {
    if std::env::var_os(CHILD).is_some() {
        return false;
    }
    let root = tempfile::tempdir().unwrap();
    let home = root.path().canonicalize().unwrap();
    let (_, module) = module_path!().split_once("::").unwrap();
    let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", &format!("{module}::{test}"), "--nocapture"])
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .env("HOME", &home)
        .env("CFFIXED_USER_HOME", &home)
        .env("VIBEWINDOW_TEST_ROOT", &home)
        .env(CHILD, &home)
        .current_dir(&home)
        .kill_on_drop(true);
    for (key, name) in [
        ("CODEX_HOME", "codex"),
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_CACHE_HOME", "cache"),
        ("XDG_STATE_HOME", "state"),
        ("XDG_RUNTIME_DIR", "runtime"),
        ("TMPDIR", "tmp"),
    ] {
        let path = home.join(name);
        std::fs::create_dir(&path).unwrap();
        command.env(key, path);
    }
    let output = tokio::time::timeout(Duration::from_secs(60), command.output())
        .await
        .expect("isolated Hermes test timed out")
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed; 0 failed"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
    true
}

struct Fixture {
    root: PathBuf,
    db: GlobalDatabase,
    port: u16,
    client: GatewayAppProviderClient,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Fixture {
    async fn new() -> Self {
        let root = PathBuf::from(std::env::var_os(CHILD).expect("isolated Hermes test root"));
        assert_eq!(agent_paths::paths().home, root);
        std::fs::create_dir(root.join(".hermes")).unwrap();
        std::fs::write(
            root.join(".hermes/config.yaml"),
            "# keep\nagent: {max_turns: 50}\ncustom_providers:\n  - name: other\n    models: {small: {}}\n",
        )
        .unwrap();
        let db = GlobalDatabase::for_home(&root).unwrap();
        let pairing = PairingGuard::new(true, &[SKEY.to_owned()]);
        let router = super::hermes::router_with_policy(None)
            .layer(Extension(db.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
            .layer(middleware::from_fn(move |req: Request, next: middleware::Next| {
                let auth = crate::gateway::api::auth::require_pairing_auth(&pairing, req.headers());
                async move {
                    match auth {
                        Ok(()) => next.run(req).await,
                        Err(error) => error.into_response(),
                    }
                }
            }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            axum::serve(listener, Router::new().nest("/v1", router)).await.unwrap()
        });
        let client = Self::client_for(port, "a", Some(SKEY));
        Self { root, db, port, client, server }
    }
    fn client_for(port: u16, company: &str, skey: Option<&str>) -> GatewayAppProviderClient {
        // Requests authenticate through the endpoint skey. The SDK's legacy token argument is unused.
        let endpoint = GatewayEndpoint::new("127.0.0.1", port)
            .with_tenant_scope(TenantScope::new("p", company).unwrap())
            .with_auth(GatewayAuth { skey: skey.map(str::to_owned) });
        GatewayAppProviderClient::new(gateway_http::GatewayTransport::new(endpoint).unwrap())
    }
    fn file(&self) -> PathBuf {
        self.root.join(".hermes/config.yaml")
    }
    fn provider_live(&self, company: &str) -> PathBuf {
        let scope = serde_json::to_vec(&("p", company)).unwrap();
        agent_paths::paths()
            .data
            .join("apps")
            .join(hex::encode(Sha256::digest(scope)))
            .join("provider-live")
    }
    fn request(&self, activation: HermesActivation) -> HermesWrite {
        HermesWrite {
            target_host_id: local_app_auth::HOST_ID.into(),
            profile: HermesProfile {
                name: "Fixture".into(),
                declaration: json!({"api_mode":"chat_completions","api_key":"fake-secret","models":[{"id":"one"}]}),
            },
            activation,
        }
    }
    fn target(&self) -> HermesTarget {
        HermesTarget { target_host_id: local_app_auth::HOST_ID.into() }
    }
    fn record(&self, id: &str) -> Option<Value> {
        self.tenant_record("a", id)
    }
    fn tenant_record(&self, company: &str, id: &str) -> Option<Value> {
        crate::app_providers::AppProviderStore::new(self.db.clone())
            .unwrap()
            .hermes_record(&TenantScope::new("p", company).unwrap(), id)
            .unwrap()
    }
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.file()).unwrap()
    }
}
#[tokio::test]
async fn cc04_hermes_sdk_api_member_edit_remove_and_catalog_delete() {
    if isolated("cc04_hermes_sdk_api_member_edit_remove_and_catalog_delete").await {
        return;
    }
    let f = Fixture::new().await;
    assert_eq!(
        f.client
            .hermes_provider_save("one", &f.request(HermesActivation::Add), "")
            .await
            .unwrap()
            .live,
        HermesLiveResult::Applied
    );
    assert_eq!(
        f.client.hermes_provider_remove_live("one", &f.target(), "").await.unwrap().live,
        HermesLiveResult::Applied
    );
    assert_eq!(f.client.hermes_state(&f.target(), "").await.unwrap().members, vec!["other"]);
    assert_eq!(
        f.client.hermes_provider_add("one", &f.target(), "").await.unwrap().live,
        HermesLiveResult::Applied
    );
    let mut request = f.request(HermesActivation::IfMember);
    request.profile.declaration["api_key"] = json!("fake-edited");
    f.client.hermes_provider_save("one", &request, "").await.unwrap();
    assert!(String::from_utf8_lossy(&f.bytes()).contains("fake-edited"));
    let state = f.client.hermes_state(&f.target(), "").await.unwrap();
    assert_eq!(state.members, vec!["one", "other"]);
    assert!(String::from_utf8_lossy(&f.bytes()).contains("# keep\nagent: {max_turns: 50}\n"));
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    assert!(
        store
            .delete(&tenant, vw_api_app_identity::AppId::Hermes, "one", &f.root)
            .is_err()
    );
    assert_eq!(
        f.client.hermes_provider_remove_live("one", &f.target(), "").await.unwrap().live,
        HermesLiveResult::Applied
    );
    assert!(f.record("one").is_some());
    assert_eq!(f.client.hermes_state(&f.target(), "").await.unwrap().members, vec!["other"]);
    store.delete(&tenant, vw_api_app_identity::AppId::Hermes, "one", &f.root).unwrap();
    assert!(f.record("one").is_none());
    let state_path = f
        .provider_live("a")
        .join(URL_SAFE_NO_PAD.encode(local_app_auth::HOST_ID))
        .join("hermes.state.json");
    let state: Value = serde_json::from_slice(&std::fs::read(state_path).unwrap()).unwrap();
    assert!(state["pointer"].is_null());
}
#[tokio::test]
async fn cc04_hermes_import_is_read_only_and_does_not_overwrite_catalog() {
    if isolated("cc04_hermes_import_is_read_only_and_does_not_overwrite_catalog").await {
        return;
    }
    let f = Fixture::new().await;
    let before = f.bytes();
    let mut permissions = std::fs::metadata(f.file()).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(f.file(), permissions).unwrap();
    assert_eq!(f.client.hermes_import(&f.target(), "").await.unwrap().imported, 1);
    assert_eq!(f.client.hermes_import(&f.target(), "").await.unwrap().imported, 0);
    assert_eq!(f.bytes(), before);
    assert!(f.record("other").is_some());
    assert!(!f.provider_live("a").exists());
    f.client
        .hermes_provider_save("draft", &f.request(HermesActivation::CatalogOnly), "")
        .await
        .unwrap();
    assert_eq!(f.bytes(), before);
}
#[tokio::test]
async fn cc04_hermes_pairing_denials_leave_no_changes_and_catalog_is_tenant_scoped() {
    if isolated("cc04_hermes_pairing_denials_leave_no_changes_and_catalog_is_tenant_scoped").await {
        return;
    }
    let f = Fixture::new().await;
    let before = f.bytes();
    let request = f.request(HermesActivation::Add);
    for skey in [None, Some("wrong-gateway-skey")] {
        let unpaired = Fixture::client_for(f.port, "a", skey);
        assert!(matches!(
            unpaired.hermes_provider_save("one", &request, "").await,
            Err(AgentAppsReadError::Http { status: 401, .. })
        ));
    }
    assert_eq!(f.client.hermes_state(&f.target(), "").await.unwrap().members, vec!["other"]);
    assert_eq!(f.bytes(), before);
    assert!(f.record("one").is_none());
    assert!(f.tenant_record("b", "one").is_none());
    assert!(!f.provider_live("a").exists());
    assert!(!f.provider_live("b").exists());

    let other = Fixture::client_for(f.port, "b", Some(SKEY));
    let result = other
        .hermes_provider_save("one", &f.request(HermesActivation::CatalogOnly), "")
        .await
        .unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, HermesLiveResult::NotAttempted);
    assert!(f.tenant_record("b", "one").is_some());
    assert!(f.record("one").is_none());
    assert_eq!(f.bytes(), before);
    assert!(!f.provider_live("a").exists());
    assert!(!f.provider_live("b").exists());
}

#[tokio::test]
async fn cc04_hermes_imported_overlay_cannot_be_edited_or_removed() {
    if isolated("cc04_hermes_imported_overlay_cannot_be_edited_or_removed").await {
        return;
    }
    let f = Fixture::new().await;
    std::fs::write(f.file(), "# overlay\nproviders:\n  native: {name: overlay, api_key: fake-overlay, key_env: FIXTURE_KEY, models: {m: {}}}\n").unwrap();
    let bytes = f.bytes();
    assert_eq!(f.client.hermes_import(&f.target(), "").await.unwrap().imported, 1);
    assert_eq!(
        f.client.hermes_state(&f.target(), "").await.unwrap().read_only_members,
        vec!["overlay"]
    );
    let record = f.record("overlay").unwrap();
    assert_eq!(record["hermes"]["declaration"]["key_env"], "FIXTURE_KEY");
    assert!(
        f.client
            .hermes_provider_save("overlay", &f.request(HermesActivation::IfMember), "")
            .await
            .is_err()
    );
    assert!(f.client.hermes_provider_remove_live("overlay", &f.target(), "").await.is_err());
    assert!(f.client.hermes_provider_add("native", &f.target(), "").await.is_err());
    assert_eq!(f.record("overlay").unwrap(), record);
    assert_eq!(f.bytes(), bytes);
}
#[tokio::test]
async fn cc04_hermes_partial_catalog_edit_preserves_credentials_and_future_fields() {
    if isolated("cc04_hermes_partial_catalog_edit_preserves_credentials_and_future_fields").await {
        return;
    }
    let f = Fixture::new().await;
    let mut request = f.request(HermesActivation::CatalogOnly);
    request.profile.declaration["key_env"] = json!("FIXTURE_KEY");
    request.profile.declaration["future"] = json!({"keep":true});
    f.client.hermes_provider_save("one", &request, "").await.unwrap();
    request.profile.declaration = json!({"apiKey":"fake-new"});
    f.client.hermes_provider_save("one", &request, "").await.unwrap();
    let record = f.record("one").unwrap();
    assert_eq!(record["hermes"]["declaration"]["api_key"], "fake-new");
    assert_eq!(record["hermes"]["declaration"]["key_env"], "FIXTURE_KEY");
    assert_eq!(record["hermes"]["declaration"]["future"], json!({"keep":true}));
    f.client.hermes_provider_add("one", &f.target(), "").await.unwrap();
    let doc: Value = serde_yaml::from_slice(&f.bytes()).unwrap();
    let row =
        doc["custom_providers"].as_array().unwrap().iter().find(|v| v["name"] == "one").unwrap();
    assert_eq!(row["api_key"], "fake-new");
    assert_eq!(row["key_env"], "FIXTURE_KEY");
    assert_eq!(row["future"], json!({"keep":true}));
}

fn editor_request(activation: HermesActivation) -> HermesFormWrite {
use vw_api_app_provider_base::basic::ProviderBasic;
    use vw_api_app_provider_base::create::ProviderSecretWrite;
    HermesFormWrite {
        target_host_id: local_app_auth::HOST_ID.into(),
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
    if isolated("cc04_ui_hermes_sdk_form_edits_one_native_member_and_keeps_secrets").await {
        return;
    }
use vw_api_app_provider_base::create::ProviderSecretWrite;
    let f = Fixture::new().await;
    let mut req = editor_request(HermesActivation::Add);
    let result = f.client.hermes_form_create("native", &req, "").await.unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, HermesLiveResult::Applied);
    let mut native = f.request(HermesActivation::IfMember);
    native.profile.declaration = json!({
        "base_url":"https://fixture.example.test/v1", "api_mode":"chat_completions",
        "api_key":"fake-form-key","key_env":"FIXTURE_KEY", "future":"opaque",
        "models":{"first":{"name":"中文模型","extra":"fake-private-model-field"}}
    });
    f.client.hermes_provider_save("native", &native, "").await.unwrap();
    let view = f.client.hermes_form_read("native", &f.target(), "").await.unwrap();
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
    let result = f.client.hermes_form_update("native", &req, "").await.unwrap();
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
        f.client.hermes_form_update("native", &req, "").await,
        Err(gateway_app_provider::AgentAppsReadError::Http { status: 409, .. })
    ));
    assert_eq!(f.bytes(), before);

    assert_eq!(
        f.client.hermes_provider_remove_live("native", &f.target(), "").await.unwrap().live,
        HermesLiveResult::Applied
    );
    assert!(f.record("native").is_some());
    assert_eq!(
        f.client.hermes_provider_add("native", &f.target(), "").await.unwrap().live,
        HermesLiveResult::Applied
    );
    assert_eq!(
        f.client.hermes_state(&f.target(), "").await.unwrap().members,
        vec!["native", "other"]
    );
}

#[tokio::test]
async fn cc04_ui_hermes_sdk_form_rejects_invalid_and_unauthorized_writes() {
    if isolated("cc04_ui_hermes_sdk_form_rejects_invalid_and_unauthorized_writes").await {
        return;
    }
    let f = Fixture::new().await;
    let mut req = editor_request(HermesActivation::Add);
    let before = f.bytes();
    let unpaired = Fixture::client_for(f.port, "a", Some("wrong-gateway-skey"));
    assert!(matches!(
        unpaired.hermes_form_create("native", &req, "").await,
        Err(AgentAppsReadError::Http { status: 401, .. })
    ));
    req.declaration = r#"{"models":[{"id":"","name":"中文模型"}]}"#.into();
    assert!(matches!(
        f.client.hermes_form_create("native", &req, "").await,
        Err(gateway_app_provider::AgentAppsReadError::Http { status: 422, .. })
    ));
    assert!(f.record("native").is_none());
    assert_eq!(f.bytes(), before);
}

#[tokio::test]
async fn cc04_ui_hermes_sdk_can_edit_an_unimported_native_member() {
    if isolated("cc04_ui_hermes_sdk_can_edit_an_unimported_native_member").await {
        return;
    }
use vw_api_app_provider_base::create::ProviderSecretWrite;
    let f = Fixture::new().await;
    assert!(f.record("other").is_none());
    let view = f.client.hermes_form_read("other", &f.target(), "").await.unwrap();
    assert_eq!(view.fields.name, "other");
    let mut req = editor_request(HermesActivation::IfMember);
    req.api_key = ProviderSecretWrite::Keep;
    req.revision = Some(view.config.unwrap().revision);
    let changed = String::from_utf8(f.bytes()).unwrap().replace("small", "external");
    std::fs::write(f.file(), &changed).unwrap();
    assert!(matches!(
        f.client.hermes_form_update("other", &req, "").await,
        Err(gateway_app_provider::AgentAppsReadError::Http { status: 409, .. })
    ));
    assert_eq!(f.bytes(), changed.as_bytes());
    assert!(f.record("other").is_none());

    let view = f.client.hermes_form_read("other", &f.target(), "").await.unwrap();
    req.revision = Some(view.config.unwrap().revision);
    let result = f.client.hermes_form_update("other", &req, "").await.unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, HermesLiveResult::Applied);
    assert_eq!(f.client.hermes_state(&f.target(), "").await.unwrap().members, vec!["other"]);
    assert!(String::from_utf8(f.bytes()).unwrap().contains("中文模型"));
}

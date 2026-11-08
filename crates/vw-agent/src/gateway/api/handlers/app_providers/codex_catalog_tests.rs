use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{
    app_providers::{claude::ClaudeActivation, codex_catalog::*},
    id::TenantScope,
};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-codex-token-with-at-least-32-chars";
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
        std::fs::create_dir(root.path().join(".codex")).unwrap();
        std::fs::write(root.path().join(".codex/config.toml"), "# fixture comment\n").unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"codex","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let router = super::codex_catalog::router_with_policy(Some(policy.clone()))
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
        Self { root, db, client, server }
    }
    fn file(&self) -> PathBuf {
        self.root.path().join(".codex/config.toml")
    }
    fn request(&self, catalog: bool, activation: ClaudeActivation) -> CodexWrite {
        CodexWrite {target_host_id:"host".into(),profile:CodexProfile{name:"Fixture".into(),config:"model_provider='vendor'\nmodel='saved'\n[model_providers.vendor]\nbase_url='https://fixture.test/v1'\nwire_api='responses'\n".into(),api_key:Some("fake-route-secret".into()),preserve_login:false},catalog:if catalog {Some(vw_api_types::model_catalog::SavedCodexCatalog {models:vec![vw_api_types::model_catalog::CodexNativeModel{slug:"saved".into(),display_name:"Saved".into(),description:"fixture".into(),base_instructions:"fixture instruction".into(),supports_reasoning_summaries:false,supports_parallel_tool_calls:false,context_window:None}]})}else{None},activation}
    }
    fn target(&self) -> CodexTarget {
        CodexTarget { target_host_id: "host".into() }
    }
    fn state(&self) -> Value {
        serde_json::from_slice(
            &std::fs::read(self.root.path().join("private/provider-live/aG9zdA/codex.state.json"))
                .unwrap(),
        )
        .unwrap()
    }
    fn record(&self, id: &str) -> Option<String> {
        GatewaySettingsStore::new(self.db.clone())
            .unwrap()
            .get(
                &TenantScope::new("p", "a").unwrap(),
                "app_providers.records",
                &format!("\"codex\":{id}"),
            )
            .unwrap()
    }
}

#[tokio::test]
async fn cc04_codex_catalog_sdk_api_commits_saved_directory_and_scoped_pointer() {
    let f = Fixture::new().await;
    let r = f
        .client
        .codex_provider_save("one", &f.request(true, ClaudeActivation::First), TOKEN)
        .await
        .unwrap();
    assert!(r.catalog_saved);
    assert_eq!(r.live, CodexLiveResult::Applied);
    assert_eq!(f.state()["pointer"]["id"], "one");
    let config = std::fs::read_to_string(f.file()).unwrap();
    assert!(config.contains("shiju-model-catalog.json"));
    assert!(!config.contains("modelCatalog"));
    let catalog = f.root.path().join(".codex/shiju-model-catalog.json");
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(&catalog).unwrap()).unwrap()["models"][0]["slug"],
        "saved"
    );
    assert!(!serde_json::to_string(&r).unwrap().contains("fake-route-secret"));
    let r = f
        .client
        .codex_provider_save("two", &f.request(false, ClaudeActivation::CatalogOnly), TOKEN)
        .await
        .unwrap();
    assert_eq!(r.live, CodexLiveResult::NotAttempted);
    assert_eq!(
        f.client.codex_provider_switch("two", &f.target(), TOKEN).await.unwrap().live,
        CodexLiveResult::Applied
    );
    assert_eq!(f.state()["pointer"]["id"], "two");
    assert!(!catalog.exists());
    assert!(!std::fs::read_to_string(f.file()).unwrap().contains("model_catalog_json"));
    assert_eq!(
        f.client.codex_provider_switch("one", &f.target(), TOKEN).await.unwrap().live,
        CodexLiveResult::Applied
    );
    assert!(catalog.exists());
}
#[tokio::test]
async fn cc04_codex_catalog_sdk_api_denial_and_profile_failure_keep_native_files() {
    let f = Fixture::new().await;
    let before = std::fs::read(f.file()).unwrap();
    assert!(
        f.client
            .codex_provider_save(
                "denied",
                &f.request(true, ClaudeActivation::Switch),
                "invalid-token"
            )
            .await
            .is_err()
    );
    assert!(f.record("denied").is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    f.client
        .codex_provider_save("one", &f.request(true, ClaudeActivation::CatalogOnly), TOKEN)
        .await
        .unwrap();
    std::fs::write(f.file(), "profile='active'\n[profiles.active]\nmodel_provider='foreign'\n")
        .unwrap();
    let before = std::fs::read(f.file()).unwrap();
    assert!(f.client.codex_provider_switch("one", &f.target(), TOKEN).await.is_err());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert!(!f.root.path().join(".codex/shiju-model-catalog.json").exists());
}

#[test]
fn cc04_codex_catalog_saved_revisions_are_deployment_and_tenant_scoped() {
    use crate::model_catalog::ModelCatalogStore;
    use vw_api_types::model_catalog::*;
    let root = tempfile::tempdir().unwrap();
    let store = ModelCatalogStore::new(GlobalDatabase::for_home(root.path()).unwrap()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let identity = ModelCatalogIdentity {
        target_host_id: "host".into(),
        app_id: vw_api_types::agent_apps::AppId::Codex,
        provider_id: "one".into(),
        endpoint: "https://fixture.test/v1".into(),
    };
    let mut catalog = SavedCodexCatalog {
        models: vec![CodexNativeModel {
            slug: "first".into(),
            display_name: "First".into(),
            description: "fixture".into(),
            base_instructions: "fixture".into(),
            supports_reasoning_summaries: false,
            supports_parallel_tool_calls: false,
            context_window: None,
        }],
    };
    assert!(store.read(&tenant, &identity, None).unwrap().is_none());
    let first = store.save(&tenant, &identity, Some(&catalog)).unwrap().unwrap();
    catalog.models[0].slug = "second".into();
    let second = store.save(&tenant, &identity, Some(&catalog)).unwrap().unwrap();
    assert_ne!(first, second);
    assert_eq!(
        store.read(&tenant, &identity, Some(&first)).unwrap().unwrap().models[0].slug,
        "first"
    );
    assert!(store.read(&TenantScope::new("p", "b").unwrap(), &identity, Some(&first)).is_err());
    for dimension in 0..3 {
        let mut other = identity.clone();
        match dimension {
            0 => other.target_host_id = "other".into(),
            1 => other.provider_id = "other".into(),
            _ => other.endpoint = "https://other.test/v1".into(),
        };
        assert!(store.read(&tenant, &other, Some(&first)).is_err());
    }
    assert!(store.save(&tenant, &identity, None).unwrap().is_none());
    assert_eq!(store.read(&tenant, &identity, Some(&second)).unwrap().unwrap(), catalog);
    catalog.models[0].context_window = Some(0);
    assert!(store.save(&tenant, &identity, Some(&catalog)).is_err());
}
#[tokio::test]
async fn cc04_codex_catalog_sdk_api_auth_stash_preservation_and_failure() {
    let f = Fixture::new().await;
    let auth = json!({"auth_mode":"chatgpt","tokens":{"id_token":"eyJhbGciOiJub25lIn0.eyJzdWIiOiJhbGljZSJ9.","access_token":"fake-access","refresh_token":"fake-refresh","account_id":"team"}});
    let auth_path = f.root.path().join(".codex/auth.json");
    std::fs::write(&auth_path, auth.to_string()).unwrap();
    let mut request = f.request(true, ClaudeActivation::Switch);
    request.profile.preserve_login = true;
    assert_eq!(
        f.client.codex_provider_save("one", &request, TOKEN).await.unwrap().live,
        CodexLiveResult::Applied
    );
    assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&auth_path).unwrap()).unwrap(), auth);
    request.profile.preserve_login = false;
    assert_eq!(
        f.client.codex_provider_save("one", &request, TOKEN).await.unwrap().live,
        CodexLiveResult::Applied
    );
    assert!(!auth_path.exists());
    let stash: Value = serde_json::from_slice(
        &std::fs::read(f.root.path().join(".codex/shiju-login-stash.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(stash["logins"]["sub:alice|account:team"], auth);
    let before = std::fs::read(f.file()).unwrap();
    std::fs::write(&auth_path, b"invalid-json").unwrap();
    let result = f
        .client
        .codex_provider_save("two", &f.request(false, ClaudeActivation::Switch), TOKEN)
        .await
        .unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, CodexLiveResult::Failed);
    assert_eq!(f.state()["pointer"]["id"], "one");
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert_eq!(std::fs::read(&auth_path).unwrap(), b"invalid-json");
}
#[tokio::test]
async fn cc04_codex_catalog_sdk_api_recovers_marked_catalog_and_pointer_before_replanning() {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use std::os::unix::fs::MetadataExt;
    use vw_agent_apps::providers::live::commit::*;
    let f = Fixture::new().await;
    f.client
        .codex_provider_save("one", &f.request(true, ClaudeActivation::Switch), TOKEN)
        .await
        .unwrap();
    let directory = f.root.path().join(".codex");
    let meta = std::fs::metadata(&directory).unwrap();
    let mut changes = Vec::new();
    for (index, name) in ["config.toml", "shiju-model-catalog.json"].into_iter().enumerate() {
        let before = std::fs::read(directory.join(name)).unwrap();
        let staged = format!(".shiju-live-recovery-{index}");
        std::fs::write(directory.join(&staged), &before).unwrap();
        std::fs::set_permissions(directory.join(&staged), std::fs::Permissions::from_mode(0o600))
            .unwrap();
        let digest = URL_SAFE_NO_PAD.encode(Sha256::digest(&before));
        std::fs::remove_file(directory.join(name)).unwrap();
        changes.push(PendingFile {
            directory: 0,
            name: name.as_bytes().to_vec(),
            staged: staged.into_bytes(),
            pre: None,
            planned: Some(digest),
        });
    }
    let pending = Pending {
        version: 1,
        host: ExecutionHostId::new("host").unwrap(),
        app: vw_api_types::agent_apps::AppId::Codex,
        tenant: TenantScope::new("p", "a").unwrap(),
        directory_device: meta.dev(),
        directory_inode: meta.ino(),
        target: PendingTarget { pointer: Some("one".into()), mode: None, ..Default::default() },
        files: changes,
        published: true,
    };
    let path = f.root.path().join("private/provider-live/aG9zdA/codex.pending.json");
    std::fs::write(&path, serde_json::to_vec(&pending).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let result = f.client.codex_provider_switch("one", &f.target(), TOKEN).await.unwrap();
    assert_eq!(result.live, CodexLiveResult::RecoveryRequired);
    assert!(f.file().exists());
    assert!(directory.join("shiju-model-catalog.json").exists());
    assert!(!path.exists());
    assert_eq!(f.state()["pointer"]["id"], "one");
    assert_eq!(
        f.client.codex_provider_switch("one", &f.target(), TOKEN).await.unwrap().live,
        CodexLiveResult::Applied
    );
}

#[tokio::test]
async fn cc04_codex_catalog_missing_write_grant_and_managed_rows_are_rejected() {
    let f = Fixture::new().await;
    let path = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(&path, policy.to_string()).unwrap();
    let before = std::fs::read(f.file()).unwrap();
    assert!(
        f.client
            .codex_provider_save("denied", &f.request(true, ClaudeActivation::Switch), TOKEN)
            .await
            .is_err()
    );
    assert!(f.record("denied").is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert_eq!(std::fs::read_dir(f.root.path().join(".codex")).unwrap().count(), 1);
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(true);
    std::fs::write(&path, policy.to_string()).unwrap();
    f.client
        .codex_provider_save("one", &f.request(true, ClaudeActivation::CatalogOnly), TOKEN)
        .await
        .unwrap();
    let settings = GatewaySettingsStore::new(f.db.clone()).unwrap();
    let mut row: Value = serde_json::from_str(&f.record("one").unwrap()).unwrap();
    row["meta"] = json!({"authBinding":{"accountId":"fake-unresolved-account"}});
    settings
        .upsert(
            &TenantScope::new("p", "a").unwrap(),
            "app_providers.records",
            "\"codex\":one",
            &row.to_string(),
        )
        .unwrap();
    assert!(f.client.codex_provider_switch("one", &f.target(), TOKEN).await.is_err());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert_eq!(std::fs::read_dir(f.root.path().join(".codex")).unwrap().count(), 1);
}

#[tokio::test]
async fn cc04_ui_codex_sdk_edits_mapping_and_preserves_inactive_fields_through_native_transaction()
{
    use vw_api_types::app_providers::{basic::ProviderBasic, create::ProviderSecretWrite};
    let f = Fixture::new().await;
    let mut seed = f.request(true, ClaudeActivation::First);
    seed.profile.config.push_str(
        "\n[mcp_servers.\"中文夹具\"]\ncommand = 'fixture-private-command' # fixture-secret-comment\n",
    );
    f.client.codex_provider_save("one", &seed, TOKEN).await.unwrap();

    let target = f.target();
    let initial = f.client.codex_form_read("one", &target, TOKEN).await.unwrap();
    let mut config = initial.config.unwrap();
    assert!(config.has_saved_key);
    assert_eq!(config.inactive, vec![vec!["config", "mcp_servers", "中文夹具"]]);
    let serialized = serde_json::to_string(&config).unwrap();
    for secret in ["fake-route-secret", "fixture-private-command", "fixture-secret-comment"] {
        assert!(!serialized.contains(secret));
    }
    assert_eq!(config.models.as_ref().unwrap()[0].slug, "saved");
    config.models.as_mut().unwrap()[0].display_name = "中文显示名".into();
    let request = CodexFormWrite {
        target_host_id: target.target_host_id,
        fields: ProviderBasic { name: "中文供应商".into(), ..initial.fields },
        toml: config.toml.replace("saved", "new-default"),
        api_key: ProviderSecretWrite::Keep,
        models: config.models,
        apply: true,
    };
    let before_native = std::fs::read(f.file()).unwrap();
    let before_record = f.record("one").unwrap();
    for (id, payload, token, create, status) in [
        ("one", request.clone(), "wrong-token", false, 401),
        ("one", request.clone(), TOKEN, true, 409),
        ("missing", request.clone(), TOKEN, false, 404),
        ("one", CodexFormWrite { toml: "model = [".into(), ..request.clone() }, TOKEN, false, 422),
        (
            "one",
            CodexFormWrite {
                toml: request.toml.replace("fixture.test", "different.test"),
                ..request.clone()
            },
            TOKEN,
            false,
            422,
        ),
    ] {
        let result = if create {
            f.client.codex_form_create(id, &payload, token).await
        } else {
            f.client.codex_form_update(id, &payload, token).await
        };
        assert!(
            matches!(result, Err(vw_gateway_client::AgentAppsReadError::Http {
            status: actual, body: Value::Null
        }) if actual == status),
            "{result:?}"
        );
    }
    assert_eq!(std::fs::read(f.file()).unwrap(), before_native);
    assert_eq!(f.record("one").unwrap(), before_record);
    assert_eq!(f.state()["pointer"]["id"], "one");

    let result = f.client.codex_form_update("one", &request, TOKEN).await.unwrap();
    assert_eq!(result.live, CodexLiveResult::Applied);
    assert!(result.catalog_saved);
    let record = f.record("one").unwrap();
    assert!(record.contains("fixture-private-command"));
    assert!(record.contains("fixture-secret-comment"));
    let native = std::fs::read_to_string(f.file()).unwrap();
    assert!(native.contains("new-default"));
    assert!(native.contains("fake-route-secret"));
    assert!(native.contains("# fixture comment"));
    assert!(!native.contains("fixture-private-command"));
    let catalog: Value = serde_json::from_slice(
        &std::fs::read(f.root.path().join(".codex/shiju-model-catalog.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(catalog["models"][0]["display_name"], "中文显示名");
    assert_eq!(catalog["models"][0]["base_instructions"], "fixture instruction");

    let result = f.client.codex_form_create("two", &request, TOKEN).await.unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, CodexLiveResult::NotAttempted);
    assert_eq!(std::fs::read_to_string(f.file()).unwrap(), native);
    assert_eq!(f.state()["pointer"]["id"], "one");

    let g = Fixture::new().await;
    let result = g.client.codex_form_create("first", &request, TOKEN).await.unwrap();
    assert_eq!(result.live, CodexLiveResult::Applied);
    assert_eq!(g.state()["pointer"]["id"], "first");
}

#[tokio::test]
async fn auth6_remove_codex_reference_is_private_and_global_switch_is_rejected() {
    use crate::{
        agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
        app_providers::AppProviderStore,
    };
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use vw_agent_apps::auth::credential_file::{AccountCredential, CredentialFileStore};
    use vw_api_types::{
        TimestampMs,
        agent_apps::AppId,
        agent_apps_auth::{AuthErrorCode, AuthService},
    };

    let f = Fixture::new().await;
    let root = f.root.path().join("private");
    let tenant = TenantScope::new("p", "a").unwrap();
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "codex",
        host: "host",
        service: AuthService::CodexOauth,
    };
    let store = AppProviderStore::new(f.db.clone()).unwrap();
    let accounts = AuthAccountStore::new(f.db.clone(), &root).unwrap();
    let id = "fixture-account";
    let owner = CredentialFileStore::new(&root).unwrap().for_owner(scope.principal).unwrap();
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
    let claims = URL_SAFE_NO_PAD.encode(br#"{"sub":"subject","chatgpt_account_id":"workspace"}"#);
    owner
        .lease(&tenant, scope.host, AppId::Codex, id, true)
        .unwrap()
        .save(&AccountCredential::CodexAuth(json!({
            "auth_mode": "chatgpt",
            "tokens": {
                "access_token": "fixture-access",
                "refresh_token": "fixture-refresh",
                "account_id": "workspace",
                    "id_token": format!("{header}.{claims}.signature")
            }
        })))
        .unwrap();
    accounts
        .insert_metadata(
            &scope,
            id,
            &AuthAccountMetadata::new(r#"["workspace","subject"]"#, TimestampMs(1)).unwrap(),
        )
        .unwrap();
    let profile = vw_agent_apps::providers::codex_catalog::import(
        Some(b"model_provider='openai'\nmodel='fixture-model'\n"),
        Some(b"{}"),
    )
    .unwrap();
    store.codex_save(&tenant, "bound", &profile, None, None).unwrap();
    store.bind_codex_account(scope, &root, "bound", None, Some(id)).unwrap();

    let config_before = std::fs::read(f.file()).unwrap();
    let global_auth = f.file().with_file_name("auth.json");
    std::fs::write(&global_auth, b"{}").unwrap();
    assert!(matches!(
        f.client.codex_provider_switch("bound", &f.target(), TOKEN).await,
        Err(vw_gateway_client::AgentAppsReadError::Http { status: 422, body: Value::Null })
    ));
    assert_eq!(store.remove_auth_account(scope, &root, id, None), Err(AuthErrorCode::AccountInUse));
    assert_eq!(std::fs::read(f.file()).unwrap(), config_before);
    assert_eq!(std::fs::read(global_auth).unwrap(), b"{}");
}

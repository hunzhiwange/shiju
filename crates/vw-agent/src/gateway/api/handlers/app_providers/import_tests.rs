use crate::{app_providers::AppProviderStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::Path};
use vw_api_types::{agent_apps::AppId, app_providers::import::*, id::TenantScope};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-import-authorized-token-32-characters";
fn write(home: &Path, name: &str, bytes: &[u8]) {
    let file = home.join(name);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, bytes).unwrap();
}
fn tenant() -> TenantScope {
    TenantScope::new("p", "a").unwrap()
}
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
        std::fs::create_dir(root.path().join("private")).unwrap();
        std::fs::set_permissions(
            root.path().join("private"),
            std::fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy, json!({"principals":[{"id":"import","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":root.path().join("private"),"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let router = super::import::router_with_policy(Some(policy))
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
    fn request(&self, trigger: ImportTrigger) -> ProviderImport {
        ProviderImport { target_host_id: "host".into(), trigger }
    }
    fn record(&self, app: AppId, id: &str) -> Value {
        AppProviderStore::preview_record(&self.db, &tenant(), app, id).unwrap()
    }
}
#[tokio::test]
async fn cc04_import_sdk_api_keeps_existing_catalog_and_native_files() {
    let f = Fixture::new().await;
    let files: &[(AppId,&str,&[u8])] = &[
        (AppId::Claude,".claude/settings.json",br#"{"env":{"ANTHROPIC_BASE_URL":"https://relay.example","ANTHROPIC_AUTH_TOKEN":"fake-key","ANTHROPIC_MODEL":"claude-test"},"unknown":true}"#),
        (AppId::Codex,".codex/config.toml",b"model_provider='relay'\nmodel='fixture'\n[model_providers.relay]\nbase_url='https://relay.example/v1'\nwire_api='responses'\n"),
        (AppId::Gemini,".gemini/.env",b"GEMINI_API_KEY='fake-key'\nGOOGLE_GEMINI_BASE_URL=https://relay.example\nGEMINI_MODEL=fixture\n"),
        (AppId::GrokBuild,".grok/config.toml",b"[models]\ndefault='fixture'\n[model.fixture]\nmodel='fixture'\nname='fixture'\nbase_url='https://relay.example/v1'\napi_key='fake-key'\napi_backend='chat_completions'\ncontext_window=32000\n"),
        (AppId::Agy,".gemini/antigravity-cli/settings.json",br#"{"model":"Display Name","unknown":true}"#),
    ];
    write(
        f.root.path(),
        ".codex/auth.json",
        br#"{"OPENAI_API_KEY":"fake-key","tokens":{"access_token":"fake-login-never-copy"}}"#,
    );
    for &(app, path, bytes) in files {
        write(f.root.path(), path, bytes);
        let result =
            f.client.provider_import(app, &f.request(ImportTrigger::Startup), TOKEN).await.unwrap();
        assert_eq!(result.imported, 1, "{app:?}");
        assert_eq!(std::fs::read(f.root.path().join(path)).unwrap(), bytes);
        assert_eq!(serde_json::to_value(&result).unwrap(), json!({"imported":1}));
        let before = f.record(app, "default");
        assert!(!before.to_string().contains("fake-login-never-copy"));
        // Invalid native files prove the startup skip occurs before reading them.
        write(f.root.path(), path, b"invalid-native");
        assert_eq!(
            f.client
                .provider_import(app, &f.request(ImportTrigger::Startup), TOKEN)
                .await
                .unwrap()
                .imported,
            0
        );
        assert_eq!(
            f.client
                .provider_import(app, &f.request(ImportTrigger::Manual), TOKEN)
                .await
                .unwrap()
                .imported,
            0
        );
        assert_eq!(f.record(app, "default"), before);
        write(f.root.path(), path, bytes);
        assert_eq!(std::fs::read(f.root.path().join(path)).unwrap(), bytes);
    }
    assert_eq!(
        std::fs::read(f.root.path().join(".codex/auth.json")).unwrap(),
        br#"{"OPENAI_API_KEY":"fake-key","tokens":{"access_token":"fake-login-never-copy"}}"#
    );
    assert!(!f.root.path().join("private/provider-live").exists());
    assert!(!f.root.path().join("private/agy-profiles").exists());
    let other = TenantScope::new("p", "other").unwrap();
    assert!(
        AppProviderStore::new(f.db.clone())
            .unwrap()
            .list(&other, AppId::Claude)
            .unwrap()
            .is_empty()
    );
}
#[tokio::test]
async fn cc04_import_incremental_pi_preserves_saved_row_and_rejects_generic_desktop_dsh() {
    let f = Fixture::new().await;
    let path = ".pi/agent/models.json";
    write(
        f.root.path(),
        path,
        br#"{"providers":{"one":{"apiKey":"fake-old","models":[]}},"defaults":{"keep":true}}"#,
    );
    assert_eq!(
        f.client
            .provider_import(AppId::Pi, &f.request(ImportTrigger::Startup), TOKEN)
            .await
            .unwrap()
            .imported,
        0
    );
    assert_eq!(
        f.client
            .provider_import(AppId::Pi, &f.request(ImportTrigger::Manual), TOKEN)
            .await
            .unwrap()
            .imported,
        1
    );
    let before = f.record(AppId::Pi, "one");
    let native = br#"{"providers":{"one":{"apiKey":"fake-new","models":[]},"two":{"apiKey":"fake-two","models":[]}},"defaults":{"keep":true}}"#;
    write(f.root.path(), path, native);
    assert_eq!(
        f.client
            .provider_import(AppId::Pi, &f.request(ImportTrigger::Manual), TOKEN)
            .await
            .unwrap()
            .imported,
        1
    );
    assert_eq!(f.record(AppId::Pi, "one"), before);
    assert!(f.record(AppId::Pi, "two").to_string().contains("fake-two"));
    assert_eq!(std::fs::read(f.root.path().join(path)).unwrap(), native);
    for app in [AppId::ClaudeDesktop, AppId::Dsh] {
        let e = f
            .client
            .provider_import(app, &f.request(ImportTrigger::Manual), TOKEN)
            .await
            .unwrap_err();
        assert!(!format!("{e:?}").contains("fake-"));
        assert!(
            AppProviderStore::new(f.db.clone()).unwrap().list(&tenant(), app).unwrap().is_empty()
        );
    }
}
#[tokio::test]
async fn cc04_import_denies_before_catalog_or_native_side_effects() {
    let f = Fixture::new().await;
    write(f.root.path(), ".gemini/.env", b"GEMINI_API_KEY=fake-key\n");
    assert!(
        f.client
            .provider_import(AppId::Gemini, &f.request(ImportTrigger::Manual), "wrong-token")
            .await
            .is_err()
    );
    let policy_path = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&policy_path).unwrap()).unwrap();
    for missing in ["provider_config_read", "provider_config_write"] {
        policy["principals"][0]["grants"][0][missing] = json!(false);
        std::fs::write(&policy_path, policy.to_string()).unwrap();
        assert!(
            f.client
                .provider_import(AppId::Gemini, &f.request(ImportTrigger::Manual), TOKEN)
                .await
                .is_err()
        );
        policy["principals"][0]["grants"][0][missing] = json!(true);
    }
    assert!(
        AppProviderStore::new(f.db.clone())
            .unwrap()
            .list(&tenant(), AppId::Gemini)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        std::fs::read(f.root.path().join(".gemini/.env")).unwrap(),
        b"GEMINI_API_KEY=fake-key\n"
    );
    assert!(!f.root.path().join("private/provider-live").exists());
}

#[tokio::test]
async fn cc04_import_existing_nondefault_official_card_blocks_startup() {
    let f = Fixture::new().await;
    let store = AppProviderStore::new(f.db.clone()).unwrap();
    store
        .agy_save(
            &tenant(),
            "saved",
            &vw_api_types::app_providers::agy::AgyProfile {
                name: "Existing native login".into(),
                model: None,
                api_key: None,
                base_url: None,
            },
        )
        .unwrap();
    write(f.root.path(), ".gemini/antigravity-cli/settings.json", br#"{"model":"Display Name"}"#);
    let before = f.record(AppId::Agy, "saved");
    assert_eq!(
        f.client
            .provider_import(AppId::Agy, &f.request(ImportTrigger::Startup), TOKEN)
            .await
            .unwrap()
            .imported,
        0
    );
    assert_eq!(store.list(&tenant(), AppId::Agy).unwrap().len(), 1);
    assert_eq!(f.record(AppId::Agy, "saved"), before);
    assert!(AppProviderStore::preview_record(&f.db, &tenant(), AppId::Agy, "default").is_err());
}
#[tokio::test]
async fn cc04_import_omp_uses_native_yaml_without_changing_defaults() {
    let f = Fixture::new().await;
    let bytes = b"providers:\n  relay:\n    apiKey: fake-key\n    models: []\ndefaults:\n  model: fixture\n";
    // OMP's read owner resolves its own default directory, rather than a generic live snapshot.
    write(f.root.path(), ".omp/agent/models.yml", bytes);
    let result = f
        .client
        .provider_import(AppId::Omp, &f.request(ImportTrigger::Manual), TOKEN)
        .await
        .unwrap();
    assert_eq!(result.imported, 1);
    assert!(f.record(AppId::Omp, "relay").to_string().contains("fake-key"));
    assert_eq!(std::fs::read(f.root.path().join(".omp/agent/models.yml")).unwrap(), bytes);
}

#[tokio::test]
async fn cc04_import_grok_manual_official_is_separate_from_startup() {
    let f = Fixture::new().await;
    write(f.root.path(), ".grok/config.toml", b"# native official\n");
    assert_eq!(
        f.client
            .provider_import(AppId::GrokBuild, &f.request(ImportTrigger::Startup), TOKEN)
            .await
            .unwrap()
            .imported,
        0
    );
    assert_eq!(
        f.client
            .provider_import(AppId::GrokBuild, &f.request(ImportTrigger::Manual), TOKEN)
            .await
            .unwrap()
            .imported,
        1
    );
    let before = f.record(AppId::GrokBuild, "grokbuild-official");
    assert_eq!(before["grok"]["config"]["kind"], "official");
    assert_eq!(
        f.client
            .provider_import(AppId::GrokBuild, &f.request(ImportTrigger::Manual), TOKEN)
            .await
            .unwrap()
            .imported,
        0
    );
    assert_eq!(f.record(AppId::GrokBuild, "grokbuild-official"), before);
    assert!(
        AppProviderStore::preview_record(&f.db, &tenant(), AppId::GrokBuild, "default").is_err()
    );
    let native = b"[models]\ndefault='fixture'\n[model.fixture]\nmodel='fixture'\nname='fixture'\nbase_url='https://relay.example/v1'\napi_key='fake-key'\napi_backend='chat_completions'\ncontext_window=32000\n";
    write(f.root.path(), ".grok/config.toml", native);
    assert_eq!(
        f.client
            .provider_import(AppId::GrokBuild, &f.request(ImportTrigger::Startup), TOKEN)
            .await
            .unwrap()
            .imported,
        0
    );
    assert_eq!(
        f.client
            .provider_import(AppId::GrokBuild, &f.request(ImportTrigger::Manual), TOKEN)
            .await
            .unwrap()
            .imported,
        1
    );
    assert_eq!(f.record(AppId::GrokBuild, "grokbuild-official"), before);
    assert!(f.record(AppId::GrokBuild, "default").to_string().contains("fake-key"));
    assert_eq!(std::fs::read(f.root.path().join(".grok/config.toml")).unwrap(), native);
    assert!(!f.root.path().join("private/provider-live").exists());
}

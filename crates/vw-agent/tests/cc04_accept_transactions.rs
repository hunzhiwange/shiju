#![cfg(target_os = "macos")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Duration,
};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{claude::*, codex_catalog::*, editor_merge::*, mode_state::*},
    id::TenantScope,
};
use vw_gateway_client::{AgentAppsReadError, GatewayClient, GatewayEndpoint};

const HOST: &str = "host";
const TOKEN: &str = "fixture-transaction-writer-token-at-least-32";
const READER: &str = "fixture-transaction-reader-token-at-least-32";
const LOGIN: &str = r#"{"auth_mode":"chatgpt","tokens":{"id_token":"eyJhbGciOiJub25lIn0.eyJzdWIiOiJhbGljZSJ9.","access_token":"fixture-access","refresh_token":"fixture-refresh","account_id":"team"}}"#;

#[tokio::test]
#[ignore = "run by the integration tests in a subprocess with an isolated HOME"]
async fn gateway_child() {
    let home = PathBuf::from(std::env::var_os("CC04_ACCEPTANCE_HOME").expect("fixture HOME"));
    assert_eq!(std::env::var_os("HOME").as_deref(), Some(home.as_os_str()));
    assert!(home.join("access.json").is_file());
    let config = vw_agent::Config {
        workspace_dir: home.join("workspace"),
        config_path: home.join("config.json"),
        ..Default::default()
    };
    fs::create_dir_all(&config.workspace_dir).unwrap();
    vw_agent::gateway::run_gateway("127.0.0.1", 0, config).await.unwrap();
}

struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Fixture(TempDir);

impl Fixture {
    fn new() -> Self {
        let fixture = Self(tempfile::tempdir().unwrap());
        fs::set_permissions(fixture.0.path(), fs::Permissions::from_mode(0o700)).unwrap();
        for dir in ["private", ".claude", ".codex"] {
            fs::create_dir(fixture.path(dir)).unwrap();
            fs::set_permissions(fixture.path(dir), fs::Permissions::from_mode(0o700)).unwrap();
        }
        let principals =
            [(TOKEN, "writer", true), (READER, "reader", false)].map(|(token, id, write)| {
                json!({
                    "id": id,
                    "token_sha256": hex::encode(Sha256::digest(token)),
                    "grants": [{
                        "platform_id": "p", "company_id": "a", "host_id": HOST,
                        "provider_config_read": true, "provider_config_write": write,
                    }],
                })
            });
        fs::write(
            fixture.path("access.json"),
            json!({
                "principals": principals,
                "hosts": [{
                    "id": HOST, "online": true, "binding": "local", "home": fixture.0.path(),
                    "credential_root": fixture.path("private"), "search_paths": [],
                    "applications": fixture.path("Applications"),
                }],
            })
            .to_string(),
        )
        .unwrap();
        fs::set_permissions(fixture.path("access.json"), fs::Permissions::from_mode(0o600))
            .unwrap();
        fixture
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.0.path().join(relative)
    }

    fn read_json(&self, path: &str) -> Value {
        serde_json::from_slice(&fs::read(self.path(path)).unwrap()).unwrap()
    }

    async fn start(&self) -> (GatewayProcess, GatewayClient) {
        let output = tempfile::NamedTempFile::new_in(self.0.path()).unwrap();
        let log = output.reopen().unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "gateway_child", "--nocapture"])
            .current_dir(self.0.path())
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", self.0.path())
            .env("TMPDIR", self.0.path())
            .env("CC04_ACCEPTANCE_HOME", self.0.path())
            .env("SHIJU_AGENT_APPS_ACCESS_FILE", self.path("access.json"))
            .stdin(Stdio::null())
            .stdout(log.try_clone().unwrap())
            .stderr(log)
            .spawn()
            .unwrap();
        let mut gateway = GatewayProcess(child);
        let client = tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                let log = fs::read_to_string(output.path()).unwrap();
                assert!(gateway.0.try_wait().unwrap().is_none(), "Gateway exited: {log}");
                if let Some(port) = log.lines().find_map(|line| {
                    line.strip_prefix("VibeWindow Gateway listening on http://127.0.0.1:")
                        .and_then(|port| port.parse().ok())
                }) {
                    let endpoint = GatewayEndpoint::new("127.0.0.1", port)
                        .with_tenant_scope(TenantScope::new("p", "a").unwrap());
                    let client = GatewayClient::new(endpoint).unwrap();
                    if client.system_health().await.is_ok() {
                        return client;
                    }
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .unwrap_or_else(|error| {
            panic!(
                "Gateway did not become ready: {error}\n{}",
                fs::read_to_string(output.path()).unwrap()
            )
        });
        (gateway, client)
    }
}

struct ImmutableFile(PathBuf);

impl ImmutableFile {
    fn new(path: PathBuf) -> Self {
        Self::set_flag(&path, "uchg");
        Self(path)
    }

    fn set_flag(path: &Path, flag: &str) {
        let status = Command::new("/usr/bin/chflags").arg(flag).arg(path).status().unwrap();
        assert!(status.success(), "could not set {flag} on transaction fixture");
    }
}

impl Drop for ImmutableFile {
    fn drop(&mut self) {
        Self::set_flag(&self.0, "nouchg");
    }
}

async fn direct_provider(client: &GatewayClient, app: AppId) -> String {
    let state = client
        .provider_mode_state(
            app,
            &ModeStateQuery {
                target_host_id: HOST.into(),
                provider_id: None,
                mode: ProviderMode::Direct,
            },
            TOKEN,
        )
        .await
        .unwrap();
    assert_eq!(state.mode, ProviderMode::Direct);
    assert_eq!(state.direct_provider, state.in_use_provider);
    state.direct_provider.unwrap()
}

fn codex_profile(id: &str, preserve_login: bool, activation: ClaudeActivation) -> CodexWrite {
    CodexWrite {
        target_host_id: HOST.into(),
        profile: CodexProfile {
            name: id.into(),
            config: format!(
                "model_provider='vendor'\nmodel='{id}-model'\n[model_providers.vendor]\nbase_url='https://{id}.example/v1'\nwire_api='responses'\n"
            ),
            api_key: Some(format!("fixture-{id}-key")),
            preserve_login,
        },
        catalog: None,
        activation,
    }
}

fn assert_codex_config(fixture: &Fixture, id: &str) {
    let path = fixture.path(".codex/config.toml");
    let config = fs::read_to_string(&path).unwrap();
    assert!(config.contains("# keep this comment\n"));
    let parsed: toml::Table = toml::from_str(&config).unwrap();
    assert_eq!(parsed["model"].as_str().unwrap(), format!("{id}-model"));
    assert_eq!(parsed["user_preference"].as_str(), Some("unchanged"));
    let route = parsed["model_provider"].as_str().unwrap();
    assert_ne!(route, "openai");
    assert_eq!(
        parsed["model_providers"][route]["base_url"].as_str().unwrap(),
        format!("https://{id}.example/v1")
    );
    assert_eq!(
        parsed["model_providers"][route]["experimental_bearer_token"].as_str().unwrap(),
        format!("fixture-{id}-key")
    );
    assert!(!parsed.contains_key("experimental_bearer_token"));
    assert!(!parsed.contains_key("openai_base_url"));
    assert_private(&path);
}

fn assert_private(path: &Path) {
    assert_eq!(fs::metadata(path).unwrap().permissions().mode() & 0o777, 0o600);
}

#[tokio::test]
async fn cc04_accept_transactions_switch_and_restart_recovery() {
    let fixture = Fixture::new();
    let auth = fixture.path(".codex/auth.json");
    fs::write(&auth, LOGIN).unwrap();
    fs::set_permissions(&auth, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(
        fixture.path(".codex/config.toml"),
        "# keep this comment\nuser_preference = 'unchanged'\n",
    )
    .unwrap();
    let (gateway, client) = fixture.start().await;
    let target = CodexTarget { target_host_id: HOST.into() };
    let state = "private/provider-live/aG9zdA/codex.state.json";
    let pending = "private/provider-live/aG9zdA/codex.pending.json";

    let first = codex_profile("one", true, ClaudeActivation::First);
    let result = client.codex_provider_save("one", &first, TOKEN).await.unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, CodexLiveResult::Applied);
    assert_eq!(direct_provider(&client, AppId::Codex).await, "one");
    for (id, preserve_login) in [("two", true), ("three", false)] {
        let profile = codex_profile(id, preserve_login, ClaudeActivation::CatalogOnly);
        let result = client.codex_provider_save(id, &profile, TOKEN).await.unwrap();
        assert!(result.catalog_saved);
        assert_eq!(result.live, CodexLiveResult::NotAttempted);
    }

    for id in ["two", "one", "two", "one"] {
        assert_eq!(
            client.codex_provider_switch(id, &target, TOKEN).await.unwrap().live,
            CodexLiveResult::Applied,
        );
        assert_eq!(direct_provider(&client, AppId::Codex).await, id);
        assert_codex_config(&fixture, id);
        assert_eq!(fs::read(&auth).unwrap(), LOGIN.as_bytes());
        assert_private(&auth);
        assert!(!fixture.path(pending).exists());
    }

    // Force a real post-publication rename failure, leaving the previous state readable.
    let immutable = ImmutableFile::new(fixture.path(state));
    let result = client.codex_provider_switch("three", &target, TOKEN).await.unwrap();
    assert_eq!(result.live, CodexLiveResult::RecoveryRequired);
    assert_eq!(fixture.read_json(pending)["published"], true);
    assert_eq!(fixture.read_json(state)["pointer"]["id"], "one");
    assert_codex_config(&fixture, "three");
    assert!(!auth.exists());
    drop(gateway);
    drop(immutable);

    let (_gateway, client) = fixture.start().await;
    assert!(
        !fixture.path(pending).exists(),
        "startup must settle the published transaction before serving"
    );
    assert_eq!(direct_provider(&client, AppId::Codex).await, "three");
    assert_codex_config(&fixture, "three");
    assert!(!auth.exists());
    let stash = ".codex/shiju-login-stash.json";
    assert_eq!(
        fixture.read_json(stash)["logins"]["sub:alice|account:team"],
        serde_json::from_str::<Value>(LOGIN).unwrap(),
    );
    assert_private(&fixture.path(stash));
    assert_private(&fixture.path(state));
}

#[tokio::test]
async fn cc04_accept_transactions_concurrent_edit_preserves_external_data() {
    let fixture = Fixture::new();
    let settings = fixture.path(".claude/settings.json");
    fs::write(&settings, br#"{"theme":"auto","hooks":{"old":true}}"#).unwrap();
    let (_gateway, client) = fixture.start().await;
    let config = ClaudeConfig::ThirdParty {
        base_url: "https://fixture.example/v1".into(),
        api_key: "fixture-claude-key".into(),
        api_key_header: false,
    };
    let request = ClaudeWrite {
        target_host_id: HOST.into(),
        claude: ClaudeProfile { name: "Fixture".into(), config, model: None },
        activation: ClaudeActivation::First,
    };
    let result = client.claude_provider_create("active", &request, TOKEN).await.unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, ClaudeLiveResult::Applied);
    let open = EditorOpen { target_host_id: HOST.into() };
    let editor =
        client.app_provider_editor_open(AppId::Claude, "active", &open, TOKEN).await.unwrap();
    assert!(editor.current);
    assert!(!serde_json::to_string(&editor).unwrap().contains("fixture-claude-key"));

    let mut external = fixture.read_json(".claude/settings.json");
    external["theme"] = json!("light");
    external["hooks"] = json!({"external":true});
    external["env"]["USER_FLAG"] = json!("keep");
    let external_bytes = serde_json::to_vec(&external).unwrap();
    fs::write(&settings, &external_bytes).unwrap();

    let mut edit = EditorSave {
        target_host_id: HOST.into(),
        context: editor.context,
        edits: vec![
            EditorEdit {
                path: vec!["theme".into()],
                value: EditorValue::Set { value: json!("dark") },
            },
            EditorEdit {
                path: vec!["includeCoAuthoredBy".into()],
                value: EditorValue::Set { value: json!(false) },
            },
        ],
        choice: EditorChoice::Refuse,
        resolution: None,
    };
    let state = fixture.path("private/provider-live/aG9zdA/claude.state.json");
    let before = fs::read(&state).unwrap();
    let record = client.app_provider_get(AppId::Claude, "active").await.unwrap();
    let denied = client.app_provider_editor_save(AppId::Claude, "active", &edit, READER).await;
    assert!(matches!(denied, Err(AgentAppsReadError::Http { status: 403, .. })));
    assert_eq!(fs::read(&settings).unwrap(), external_bytes);
    assert_eq!(fs::read(&state).unwrap(), before);
    assert_eq!(client.app_provider_get(AppId::Claude, "active").await.unwrap(), record);

    let conflict =
        client.app_provider_editor_save(AppId::Claude, "active", &edit, TOKEN).await.unwrap();
    assert!(!conflict.saved);
    assert_eq!(conflict.conflicts, vec![vec!["theme".to_string()]]);
    assert_eq!(fs::read(&settings).unwrap(), external_bytes);
    edit.choice = EditorChoice::KeepTheirs;
    edit.resolution = Some(conflict.resolution.expect("conflict resolution token"));
    assert!(
        client.app_provider_editor_save(AppId::Claude, "active", &edit, TOKEN).await.unwrap().saved
    );
    external["includeCoAuthoredBy"] = json!(false);
    assert_eq!(fixture.read_json(".claude/settings.json"), external);
    assert_eq!(external["env"]["ANTHROPIC_AUTH_TOKEN"], "fixture-claude-key");
    assert_eq!(client.app_provider_get(AppId::Claude, "active").await.unwrap(), record);
    assert_eq!(direct_provider(&client, AppId::Claude).await, "active");
    assert!(!fixture.path("private/provider-live/aG9zdA/claude.pending.json").exists());
    assert_private(&settings);
}

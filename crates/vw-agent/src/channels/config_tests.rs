use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::{AutonomyConfig, Config, NonCliNaturalLanguageApprovalMode};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::{ChatMessage, Provider, ProviderRuntimeOptions};
use crate::app::agent::security::SecretStore;
use crate::app::agent::tools::Tool;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::SystemTime;

use super::super::{
    ChannelRuntimeContext, ChannelRuntimeDefaults, ConfigFileStamp, RuntimeConfigState,
};
use super::{
    channel_message_timeout_budget_secs, config_file_stamp,
    decrypt_optional_secret_for_runtime_reload, effective_channel_message_timeout_secs,
    load_runtime_defaults_from_config_file, maybe_apply_runtime_config_update,
    resolved_default_model, resolved_default_provider, runtime_autonomy_policy_from_config,
    runtime_config_path, runtime_config_store, runtime_defaults_from_config,
    runtime_defaults_snapshot, snapshot_non_cli_excluded_tools,
};

struct StaticProvider;

const CONFIG_ENV_KEYS: &[&str] = &[
    "VIBEWINDOW_API_KEY",
    "API_KEY",
    "VIBEWINDOW_PROVIDER",
    "VIBEWINDOW_MODEL_PROVIDER",
    "MODEL_PROVIDER",
    "PROVIDER",
    "VIBEWINDOW_MODEL",
    "MODEL",
];

fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|error| error.into_inner())
}

struct ConfigEnvGuard {
    _lock: MutexGuard<'static, ()>,
    previous_env: Vec<(&'static str, Option<OsString>)>,
}

impl ConfigEnvGuard {
    fn new() -> Self {
        let lock = env_lock();
        let previous_env =
            CONFIG_ENV_KEYS.iter().map(|key| (*key, std::env::var_os(*key))).collect::<Vec<_>>();

        for &key in CONFIG_ENV_KEYS {
            unsafe {
                std::env::remove_var(key);
            }
        }

        Self { _lock: lock, previous_env }
    }
}

impl Drop for ConfigEnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.previous_env {
            unsafe {
                if let Some(value) = value {
                    std::env::set_var(key, value);
                } else {
                    std::env::remove_var(key);
                }
            }
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(message.to_string())
    }
}

fn context(vibewindow_dir: Option<PathBuf>, excluded_tools: Vec<String>) -> ChannelRuntimeContext {
    let provider: Arc<dyn Provider> = Arc::new(StaticProvider);
    ChannelRuntimeContext {
        channels_by_name: Arc::new(HashMap::new()),
        provider: Arc::clone(&provider),
        default_provider: Arc::new("ctx-provider".to_string()),
        memory: Arc::new(NoneMemory::new()),
        tools_registry: Arc::new(Vec::<Box<dyn Tool>>::new()),
        observer: Arc::new(NoopObserver),
        system_prompt: Arc::new("system".to_string()),
        model: Arc::new("ctx-model".to_string()),
        temperature: 0.8,
        auto_save_memory: false,
        max_iterations: 2,
        min_relevance_score: 0.0,
        conversation_histories: Arc::new(Mutex::new(HashMap::<String, Vec<ChatMessage>>::new())),
        provider_cache: Arc::new(Mutex::new(HashMap::new())),
        route_overrides: Arc::new(Mutex::new(HashMap::new())),
        api_key: Some("ctx-key".to_string()),
        api_url: Some("https://ctx.example.test".to_string()),
        reliability: Arc::new(crate::app::agent::config::ReliabilityConfig::default()),
        provider_runtime_options: ProviderRuntimeOptions {
            vibewindow_dir,
            ..ProviderRuntimeOptions::default()
        },
        workspace_dir: Arc::new(PathBuf::from(".")),
        message_timeout_secs: 30,
        interrupt_on_new_message: false,
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        hooks: None,
        non_cli_excluded_tools: Arc::new(Mutex::new(excluded_tools)),
        query_classification: crate::app::agent::config::QueryClassificationConfig::default(),
        model_routes: Vec::new(),
        approval_manager: Arc::new(ApprovalManager::from_config(&AutonomyConfig::default())),
    }
}

fn defaults(provider: &str, model: &str) -> ChannelRuntimeDefaults {
    ChannelRuntimeDefaults {
        default_provider: provider.to_string(),
        model: model.to_string(),
        temperature: 0.4,
        api_key: Some("cached-key".to_string()),
        api_url: Some("https://cached.example.test".to_string()),
        reliability: crate::app::agent::config::ReliabilityConfig::default(),
    }
}

#[test]
fn timeout_helpers_apply_floor_scale_and_saturating_math() {
    assert_eq!(effective_channel_message_timeout_secs(1), 30);
    assert_eq!(effective_channel_message_timeout_secs(45), 45);
    assert_eq!(channel_message_timeout_budget_secs(10, 0), 10);
    assert_eq!(channel_message_timeout_budget_secs(10, 99), 40);
}

#[test]
fn runtime_defaults_preserve_configured_values() {
    let config = Config {
        default_provider: Some("provider-a".to_string()),
        default_model: Some("model-a".to_string()),
        default_temperature: 0.25,
        api_key: Some("key".to_string()),
        api_url: Some("https://api.example.test".to_string()),
        ..Config::default()
    };

    assert_eq!(resolved_default_provider(&config), "provider-a");
    assert_eq!(resolved_default_model(&config), "model-a");

    let defaults = runtime_defaults_from_config(&config);
    assert_eq!(defaults.default_provider, "provider-a");
    assert_eq!(defaults.model, "model-a");
    assert_eq!(defaults.temperature, 0.25);
    assert_eq!(defaults.api_key.as_deref(), Some("key"));

    let policy = runtime_autonomy_policy_from_config(&config);
    assert_eq!(policy.auto_approve, config.autonomy.auto_approve);
    let _guard = runtime_config_store().lock().unwrap_or_else(|e| e.into_inner());
}

#[test]
fn runtime_defaults_use_builtin_provider_and_model_when_unset() {
    let config = Config::default();

    assert_eq!(resolved_default_provider(&config), "openrouter");
    assert_eq!(resolved_default_model(&config), "zhipuai-coding-plan/glm-5");

    let defaults = runtime_defaults_from_config(&config);
    assert_eq!(defaults.default_provider, "openrouter");
    assert_eq!(defaults.model, "zhipuai-coding-plan/glm-5");
    assert_eq!(defaults.temperature, config.default_temperature);
}

#[test]
fn resolved_defaults_use_hardcoded_fallbacks_when_fields_are_none() {
    let config = Config { default_provider: None, default_model: None, ..Config::default() };

    assert_eq!(resolved_default_provider(&config), "zhipuai-coding-plan");
    assert_eq!(resolved_default_model(&config), "zhipuai-coding-plan/glm-5");
}

#[test]
fn runtime_autonomy_policy_preserves_all_runtime_fields() {
    let mut mode_by_channel = HashMap::new();
    mode_by_channel.insert("telegram".to_string(), NonCliNaturalLanguageApprovalMode::Direct);

    let mut config = Config::default();
    config.autonomy.auto_approve = vec!["shell".to_string()];
    config.autonomy.always_ask = vec!["file_write".to_string()];
    config.autonomy.non_cli_excluded_tools = vec!["browser".to_string()];
    config.autonomy.non_cli_approval_approvers = vec!["telegram:admin".to_string()];
    config.autonomy.non_cli_natural_language_approval_mode =
        NonCliNaturalLanguageApprovalMode::RequestConfirm;
    config.autonomy.non_cli_natural_language_approval_mode_by_channel = mode_by_channel.clone();

    let policy = runtime_autonomy_policy_from_config(&config);

    assert_eq!(policy.auto_approve, vec!["shell"]);
    assert_eq!(policy.always_ask, vec!["file_write"]);
    assert_eq!(policy.non_cli_excluded_tools, vec!["browser"]);
    assert_eq!(policy.non_cli_approval_approvers, vec!["telegram:admin"]);
    assert_eq!(
        policy.non_cli_natural_language_approval_mode,
        NonCliNaturalLanguageApprovalMode::RequestConfirm
    );
    assert_eq!(policy.non_cli_natural_language_approval_mode_by_channel, mode_by_channel);
}

#[test]
fn timeout_budget_saturates_on_large_values() {
    assert_eq!(channel_message_timeout_budget_secs(u64::MAX, usize::MAX), u64::MAX);
}

#[test]
fn runtime_config_path_uses_vibewindow_dir_when_present() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(Some(dir.path().to_path_buf()), Vec::new());
    assert_eq!(runtime_config_path(&ctx), Some(dir.path().join(".vibewindow.json")));

    let ctx = context(None, Vec::new());
    assert_eq!(runtime_config_path(&ctx), None);
}

#[test]
fn runtime_defaults_snapshot_uses_context_without_cached_config() {
    let ctx = context(None, Vec::new());

    let snapshot = runtime_defaults_snapshot(&ctx);

    assert_eq!(snapshot.default_provider, "ctx-provider");
    assert_eq!(snapshot.model, "ctx-model");
    assert_eq!(snapshot.temperature, 0.8);
    assert_eq!(snapshot.api_key.as_deref(), Some("ctx-key"));
    assert_eq!(snapshot.api_url.as_deref(), Some("https://ctx.example.test"));
}

#[test]
fn runtime_defaults_snapshot_uses_context_when_config_path_is_not_cached() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(Some(dir.path().to_path_buf()), Vec::new());
    let config_path = runtime_config_path(&ctx).unwrap();
    runtime_config_store().lock().unwrap_or_else(|e| e.into_inner()).remove(&config_path);

    let snapshot = runtime_defaults_snapshot(&ctx);

    assert_eq!(snapshot.default_provider, "ctx-provider");
    assert_eq!(snapshot.model, "ctx-model");
    assert_eq!(snapshot.temperature, 0.8);
    assert_eq!(snapshot.api_key.as_deref(), Some("ctx-key"));
    assert_eq!(snapshot.api_url.as_deref(), Some("https://ctx.example.test"));
}

#[test]
fn runtime_defaults_snapshot_prefers_cached_config_state() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(Some(dir.path().to_path_buf()), Vec::new());
    let config_path = runtime_config_path(&ctx).unwrap();
    let cached = defaults("cached-provider", "cached-model");

    {
        let mut store = runtime_config_store().lock().unwrap_or_else(|e| e.into_inner());
        store.insert(
            config_path.clone(),
            RuntimeConfigState {
                defaults: cached.clone(),
                last_applied_stamp: Some(ConfigFileStamp {
                    modified: SystemTime::UNIX_EPOCH,
                    len: 1,
                }),
            },
        );
    }

    let snapshot = runtime_defaults_snapshot(&ctx);

    assert_eq!(snapshot.default_provider, "cached-provider");
    assert_eq!(snapshot.model, "cached-model");
    assert_eq!(snapshot.api_key, cached.api_key);

    runtime_config_store().lock().unwrap_or_else(|e| e.into_inner()).remove(&config_path);
}

#[test]
fn snapshot_non_cli_excluded_tools_clones_current_list() {
    let ctx = context(None, vec!["shell".to_string(), "browser".to_string()]);

    let snapshot = snapshot_non_cli_excluded_tools(&ctx);

    assert_eq!(snapshot, vec!["shell", "browser"]);
}

#[test]
fn decrypt_optional_secret_for_runtime_reload_handles_all_value_shapes() {
    let dir = tempfile::tempdir().unwrap();
    let store = SecretStore::new(dir.path(), true);

    let mut missing = None;
    decrypt_optional_secret_for_runtime_reload(&store, &mut missing, "config.api_key").unwrap();
    assert_eq!(missing, None);

    let mut plain = Some("plain".to_string());
    decrypt_optional_secret_for_runtime_reload(&store, &mut plain, "config.api_key").unwrap();
    assert_eq!(plain.as_deref(), Some("plain"));

    let encrypted = store.encrypt("secret").unwrap();
    let mut encrypted_value = Some(encrypted);
    decrypt_optional_secret_for_runtime_reload(&store, &mut encrypted_value, "config.api_key")
        .unwrap();
    assert_eq!(encrypted_value.as_deref(), Some("secret"));

    let mut invalid = Some("enc2:not-hex".to_string());
    let err = decrypt_optional_secret_for_runtime_reload(&store, &mut invalid, "config.api_key")
        .unwrap_err();
    assert!(err.to_string().contains("Failed to decrypt config.api_key"));
}

#[tokio::test]
async fn config_file_stamp_reports_existing_files_only() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.toml");
    assert!(config_file_stamp(&missing).await.is_none());

    let path = dir.path().join("vibewindow.toml");
    tokio::fs::write(&path, "default_provider = \"provider-a\"\n").await.unwrap();
    let stamp = config_file_stamp(&path).await.unwrap();
    assert!(stamp.len > 0);
}

#[tokio::test]
async fn load_runtime_defaults_reports_read_and_parse_errors() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.toml");
    assert!(
        load_runtime_defaults_from_config_file(&missing)
            .await
            .unwrap_err()
            .to_string()
            .contains("Failed to read")
    );

    let invalid = dir.path().join("invalid.toml");
    tokio::fs::write(&invalid, "default_provider = [").await.unwrap();
    assert!(
        load_runtime_defaults_from_config_file(&invalid)
            .await
            .unwrap_err()
            .to_string()
            .contains("Failed to parse")
    );
}

#[tokio::test]
async fn load_runtime_defaults_reads_toml_config_and_policy() {
    let dir = tempfile::tempdir().unwrap();
    let store = SecretStore::new(dir.path(), true);
    let encrypted = store.encrypt("secret").unwrap();
    let mut mode_by_channel = HashMap::new();
    mode_by_channel.insert("telegram".to_string(), NonCliNaturalLanguageApprovalMode::Direct);

    let mut config = Config::default();
    config.default_provider = Some("openrouter".to_string());
    config.default_model = Some("runtime-model".to_string());
    config.default_temperature = 0.35;
    config.api_key = Some(encrypted);
    config.api_url = Some("https://runtime.example.test/v1".to_string());
    config.reliability.provider_retries = 7;
    config.autonomy.auto_approve = vec!["shell".to_string()];
    config.autonomy.always_ask = vec!["file_write".to_string()];
    config.autonomy.non_cli_excluded_tools = vec!["browser".to_string()];
    config.autonomy.non_cli_approval_approvers = vec!["telegram:admin".to_string()];
    config.autonomy.non_cli_natural_language_approval_mode =
        NonCliNaturalLanguageApprovalMode::RequestConfirm;
    config.autonomy.non_cli_natural_language_approval_mode_by_channel = mode_by_channel.clone();

    let path = dir.path().join(".vibewindow.json");
    tokio::fs::write(&path, toml::to_string(&config).unwrap()).await.unwrap();

    let (defaults, policy) = load_runtime_defaults_from_config_file(&path).await.unwrap();

    assert_eq!(defaults.default_provider, "openrouter");
    assert_eq!(defaults.model, "runtime-model");
    assert_eq!(defaults.temperature, 0.35);
    assert_eq!(defaults.api_key.as_deref(), Some("secret"));
    assert_eq!(defaults.api_url.as_deref(), Some("https://runtime.example.test/v1"));
    assert_eq!(defaults.reliability.provider_retries, 7);
    assert_eq!(policy.auto_approve, vec!["shell"]);
    assert_eq!(policy.always_ask, vec!["file_write"]);
    assert_eq!(policy.non_cli_excluded_tools, vec!["browser"]);
    assert_eq!(policy.non_cli_approval_approvers, vec!["telegram:admin"]);
    assert_eq!(
        policy.non_cli_natural_language_approval_mode,
        NonCliNaturalLanguageApprovalMode::RequestConfirm
    );
    assert_eq!(policy.non_cli_natural_language_approval_mode_by_channel, mode_by_channel);
}

#[tokio::test]
async fn maybe_apply_runtime_config_update_skips_missing_paths_and_unchanged_stamp() {
    let ctx = context(None, Vec::new());
    maybe_apply_runtime_config_update(&ctx).await.unwrap();

    let dir = tempfile::tempdir().unwrap();
    let ctx = context(Some(dir.path().to_path_buf()), Vec::new());
    maybe_apply_runtime_config_update(&ctx).await.unwrap();

    let config_path = runtime_config_path(&ctx).unwrap();
    tokio::fs::write(&config_path, "not valid toml [").await.unwrap();
    let stamp = config_file_stamp(&config_path).await.unwrap();

    {
        let mut store = runtime_config_store().lock().unwrap_or_else(|e| e.into_inner());
        store.insert(
            config_path.clone(),
            RuntimeConfigState {
                defaults: defaults("same-provider", "same-model"),
                last_applied_stamp: Some(stamp),
            },
        );
    }

    maybe_apply_runtime_config_update(&ctx).await.unwrap();

    runtime_config_store().lock().unwrap_or_else(|e| e.into_inner()).remove(&config_path);
}

#[tokio::test]
async fn maybe_apply_runtime_config_update_returns_load_errors_for_changed_files() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(Some(dir.path().to_path_buf()), Vec::new());
    let config_path = runtime_config_path(&ctx).unwrap();

    tokio::fs::write(&config_path, "default_provider = [").await.unwrap();

    let err = maybe_apply_runtime_config_update(&ctx).await.unwrap_err();

    assert!(err.to_string().contains("Failed to parse"));
    assert!(
        !runtime_config_store()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(&config_path)
    );
}

#[tokio::test]
async fn maybe_apply_runtime_config_update_applies_changed_config_to_runtime_state() {
    let _env = ConfigEnvGuard::new();
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(Some(dir.path().to_path_buf()), vec!["old_tool".to_string()]);
    let config_path = runtime_config_path(&ctx).unwrap();
    runtime_config_store().lock().unwrap_or_else(|e| e.into_inner()).remove(&config_path);
    ctx.provider_cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert("old-provider".to_string(), Arc::clone(&ctx.provider));

    let mut mode_by_channel = HashMap::new();
    mode_by_channel.insert("telegram".to_string(), NonCliNaturalLanguageApprovalMode::Disabled);

    let mut config = Config::default();
    config.default_provider = Some("custom:https://llm.example.test/v1".to_string());
    config.default_model = Some("runtime-reload-model".to_string());
    config.default_temperature = 0.12;
    config.api_key = Some("runtime-key".to_string());
    config.api_url = Some("https://runtime-reload.example.test/v1".to_string());
    config.autonomy.auto_approve = vec!["mock_price".to_string()];
    config.autonomy.always_ask = vec!["shell".to_string()];
    config.autonomy.non_cli_excluded_tools = vec!["browser".to_string(), "fs".to_string()];
    config.autonomy.non_cli_approval_approvers = vec!["telegram:alice".to_string()];
    config.autonomy.non_cli_natural_language_approval_mode =
        NonCliNaturalLanguageApprovalMode::Direct;
    config.autonomy.non_cli_natural_language_approval_mode_by_channel = mode_by_channel;

    tokio::fs::write(&config_path, toml::to_string(&config).unwrap()).await.unwrap();

    maybe_apply_runtime_config_update(&ctx).await.unwrap();

    {
        let cache = ctx.provider_cache.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(cache.len(), 1);
        assert!(cache.contains_key("custom:https://llm.example.test/v1"));
    }

    let state = {
        runtime_config_store()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&config_path)
            .cloned()
            .unwrap()
    };
    assert_eq!(state.defaults.default_provider, "custom:https://llm.example.test/v1");
    assert_eq!(state.defaults.model, "runtime-reload-model");
    assert_eq!(state.defaults.temperature, 0.12);
    assert_eq!(state.defaults.api_key.as_deref(), Some("runtime-key"));
    assert_eq!(
        snapshot_non_cli_excluded_tools(&ctx),
        vec!["browser".to_string(), "fs".to_string()]
    );
    assert!(!ctx.approval_manager.needs_approval("mock_price"));
    assert!(ctx.approval_manager.needs_approval("shell"));
    assert!(ctx.approval_manager.is_non_cli_approval_actor_allowed("telegram", "alice"));
    assert_eq!(
        ctx.approval_manager.non_cli_natural_language_approval_mode_for_channel("telegram"),
        NonCliNaturalLanguageApprovalMode::Disabled
    );
    assert_eq!(
        ctx.approval_manager.non_cli_natural_language_approval_mode_for_channel("discord"),
        NonCliNaturalLanguageApprovalMode::Direct
    );

    runtime_config_store().lock().unwrap_or_else(|e| e.into_inner()).remove(&config_path);
}

#[tokio::test]
async fn maybe_apply_runtime_config_update_does_not_mutate_state_when_provider_creation_fails() {
    let _env = ConfigEnvGuard::new();
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(Some(dir.path().to_path_buf()), vec!["old_tool".to_string()]);
    let config_path = runtime_config_path(&ctx).unwrap();
    runtime_config_store().lock().unwrap_or_else(|e| e.into_inner()).remove(&config_path);
    ctx.provider_cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert("old-provider".to_string(), Arc::clone(&ctx.provider));

    let config = Config {
        default_provider: Some("custom:ftp://llm.example.test/v1".to_string()),
        default_model: Some("bad-provider-model".to_string()),
        autonomy: AutonomyConfig {
            non_cli_excluded_tools: vec!["browser".to_string()],
            ..AutonomyConfig::default()
        },
        ..Config::default()
    };
    tokio::fs::write(&config_path, toml::to_string(&config).unwrap()).await.unwrap();

    let err = maybe_apply_runtime_config_update(&ctx).await.unwrap_err();

    assert!(err.to_string().contains("custom provider URL must use http/https"));
    assert_eq!(snapshot_non_cli_excluded_tools(&ctx), vec!["old_tool"]);
    assert!(
        !runtime_config_store()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(&config_path)
    );
    let cache = ctx.provider_cache.lock().unwrap_or_else(|e| e.into_inner());
    assert_eq!(cache.len(), 1);
    assert!(cache.contains_key("old-provider"));
}

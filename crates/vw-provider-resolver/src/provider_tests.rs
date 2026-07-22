use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use tokio::sync::{Mutex, MutexGuard};

use super::*;

static TEST_LOCK: Mutex<()> = Mutex::const_new(());
static TEST_ROOT: OnceLock<PathBuf> = OnceLock::new();

struct ProviderTestEnv {
    _lock: MutexGuard<'static, ()>,
    config_dir: PathBuf,
    saved_env: Vec<(&'static str, Option<OsString>)>,
}

impl ProviderTestEnv {
    async fn new() -> Self {
        let lock = TEST_LOCK.lock().await;
        let root = test_root();
        let config_dir = fresh_dir(&root, "config");

        std::fs::create_dir_all(&config_dir).expect("config dir should be created");

        let saved_env = save_env(&[
            "VIBEWINDOW_WORKSPACE",
            "TEST_PROVIDER_KEY",
            "SECOND_PROVIDER_KEY",
            "VIBEWINDOW_PROVIDER_KEY",
        ]);

        unsafe {
            std::env::set_var("VIBEWINDOW_WORKSPACE", &config_dir);
            std::env::remove_var("TEST_PROVIDER_KEY");
            std::env::remove_var("SECOND_PROVIDER_KEY");
            std::env::remove_var("VIBEWINDOW_PROVIDER_KEY");
        }

        crate::models::invalidate_cache();
        invalidate_cache().await;

        Self { _lock: lock, config_dir, saved_env }
    }

    fn config_path(&self) -> PathBuf {
        self.config_dir.join(".vibewindow.json")
    }

    fn set_env(&self, key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }

    async fn write_config(&self, value: Value) {
        write_json(&self.config_path(), &value);
        invalidate_cache().await;
    }
}

impl Drop for ProviderTestEnv {
    fn drop(&mut self) {
        for (key, value) in &self.saved_env {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

fn save_env(keys: &[&'static str]) -> Vec<(&'static str, Option<OsString>)> {
    keys.iter().map(|key| (*key, std::env::var_os(key))).collect()
}

fn test_root() -> PathBuf {
    TEST_ROOT
        .get_or_init(|| {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos();
            std::env::temp_dir()
                .join(format!("vw-provider-resolver-tests-{}-{nanos}", std::process::id()))
        })
        .clone()
}

fn fresh_dir(root: &Path, prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    root.join(format!("{prefix}-{nanos}"))
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("json parent dir should be created");
    }
    std::fs::write(path, serde_json::to_string_pretty(value).expect("json value should serialize"))
        .expect("json file should be written");
}

fn config_with_default(default_model: Value, providers: Value) -> Value {
    json!({
        "default_model": default_model,
        "default_temperature": 0.7,
        "providers": providers
    })
}

fn active_test_provider_config() -> Value {
    json!({
        "zz-test-provider": {
            "name": "Configured Provider",
            "api": "https://config.example/test",
            "env": ["TEST_PROVIDER_KEY"],
            "adapter": "acp",
            "options": {"timeout_seconds": 30},
            "models": {
                "alpha": {
                    "id": "alpha-api",
                    "name": "Alpha Config",
                    "family": "config-family",
                    "release_date": "2026-01-01",
                    "status": "active",
                    "headers": {"x-config": "config"},
                    "options": {"configured": true},
                    "limit": {"context": 32768, "input": 16384, "output": 8192},
                    "provider": {"adapter": "custom-adapter"}
                },
                "alias-a": {
                    "id": "shared-api",
                    "name": "Alias A",
                    "status": "active",
                    "limit": {"context": 1024, "output": 256}
                },
                "alias-b": {
                    "id": "shared-api",
                    "name": "Alias B",
                    "status": "active",
                    "limit": {"context": 1024, "output": 256}
                }
            }
        }
    })
}

#[test]
fn suggest_returns_best_three_matches_above_threshold() {
    let suggestions = suggest(
        "test-providr",
        vec!["other", "test-provider", "test-provider-alt", "test-proxy", "unrelated"],
    );

    assert_eq!(suggestions.len(), 3);
    assert_eq!(suggestions[0], "test-provider");
    assert!(suggestions.contains(&"test-provider-alt".to_string()));
}

#[test]
fn read_config_fingerprint_reports_missing_and_existing_file() {
    let runtime = tokio::runtime::Runtime::new().expect("runtime should build");
    runtime.block_on(async {
        let env = ProviderTestEnv::new().await;
        let missing = read_config_fingerprint();
        assert_eq!(missing.path, env.config_path().to_string_lossy());
        assert!(!missing.exists);

        env.write_config(config_with_default(Value::Null, json!({}))).await;
        let existing = read_config_fingerprint();
        assert!(existing.exists);
        assert!(existing.len > 0);
        assert!(existing.modified_nanos > 0);
    });
}

#[tokio::test]
async fn settings_state_keeps_disabled_config_models() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "zz-test-provider": {
                "models": {
                    "alpha": {"status": "active"},
                    "beta": {"status": "disabled"}
                }
            }
        }),
    ))
    .await;

    let providers = list_for_settings().await;
    let provider = providers.get("zz-test-provider").expect("provider should exist");

    assert!(provider.models.contains_key("alpha"));
    assert!(provider.models.contains_key("beta"));
    assert_eq!(provider.models["beta"].status, "disabled");
}

#[tokio::test]
async fn provider_without_models_is_dropped() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "empty-provider": {
                "name": "Empty Provider"
            }
        }),
    ))
    .await;

    assert!(!list_for_settings().await.contains_key("empty-provider"));
}

#[tokio::test]
async fn filtered_state_keeps_only_active_models_and_merges_config_fields() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    let providers = list().await;
    let provider = providers.get("zz-test-provider").expect("configured provider should exist");
    let model = provider.models.get("alpha").expect("active model should exist");

    assert_eq!(provider.name, "Configured Provider");
    assert!(matches!(provider.source, ProviderSource::Config));
    assert_eq!(provider.options["timeout_seconds"], json!(30));
    assert!(!provider.models.contains_key("beta"));
    assert_eq!(model.api.id, "alpha-api");
    assert_eq!(model.api.url, "https://config.example/test");
    assert_eq!(model.api.adapter, "custom-adapter");
    assert_eq!(model.name, "Alpha Config");
    assert_eq!(model.family.as_deref(), Some("config-family"));
    assert_eq!(model.release_date, "2026-01-01");
    assert_eq!(model.headers["x-config"], "config");
    assert_eq!(model.options["configured"], json!(true));
    assert_eq!(model.limit.context, 32768);
    assert_eq!(model.limit.input, Some(16384));
    assert_eq!(model.limit.output, 8192);
    assert!(model.capabilities.toolcall);
    assert!(model.capabilities.input.text);
    assert_eq!(model.cost.input, 0.0);
}

#[tokio::test]
async fn env_source_does_not_expose_key_when_provider_declares_multiple_env_vars() {
    let env = ProviderTestEnv::new().await;
    env.set_env("SECOND_PROVIDER_KEY", "second-secret");
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "multi-env-provider": {
                "env": ["MISSING_PROVIDER_KEY", "SECOND_PROVIDER_KEY"],
                "models": {
                    "alpha": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    let provider = get_provider("multi-env-provider").await.expect("provider should resolve");

    assert!(matches!(provider.source, ProviderSource::Env));
    assert_eq!(provider.key, None);
}

#[tokio::test]
async fn custom_config_provider_uses_safe_defaults() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "custom-only": {
                "api": "https://config.example/custom",
                "models": {
                    "custom-model": {
                        "status": "active",
                        "limit": {"context": 7000, "output": 700}
                    }
                }
            }
        }),
    ))
    .await;

    let model =
        get_model("custom-only", "custom-model").await.expect("custom model should resolve");

    assert_eq!(model.api.id, "custom-model");
    assert_eq!(model.api.adapter, default_adapter());
    assert_eq!(model.name, "custom-model");
    assert_eq!(model.provider_id, "custom-only");
    assert!(model.capabilities.toolcall);
    assert!(model.capabilities.input.text);
    assert_eq!(model.cost.output, 0.0);
}

#[tokio::test]
async fn provider_id_and_model_id_defaults_are_used_when_config_names_are_absent() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "unnamed-provider": {
                "models": {
                    "unnamed-model": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    let provider = get_provider("unnamed-provider").await.expect("provider should resolve");
    let model = get_model("unnamed-provider", "unnamed-model").await.expect("model should resolve");

    assert_eq!(provider.name, "unnamed-provider");
    assert_eq!(model.name, "unnamed-model");
    assert_eq!(model.api.id, "unnamed-model");
    assert_eq!(model.api.url, "");
}

#[tokio::test]
async fn env_source_sets_key_only_for_single_env_provider() {
    let env = ProviderTestEnv::new().await;
    env.set_env("TEST_PROVIDER_KEY", "env-secret");
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    let provider = get_provider("zz-test-provider").await.expect("provider should resolve");

    assert!(matches!(provider.source, ProviderSource::Env));
    assert_eq!(provider.key.as_deref(), Some("env-secret"));
}

#[tokio::test]
async fn get_small_model_prioritizes_github_copilot_models() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "github-copilot-test": {
                "models": {
                    "claude-haiku-4.5": {"status": "active"},
                    "gpt-5-mini": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    let model = get_small_model("github-copilot-test").await.expect("small model should resolve");

    assert_eq!(model.id, "gpt-5-mini");
}

#[tokio::test]
async fn get_model_resolves_unique_api_alias() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    let model =
        get_model("zz-test-provider", "alpha-api").await.expect("unique api alias should resolve");

    assert_eq!(model.id, "alpha");
}

#[tokio::test]
async fn get_model_reports_provider_suggestions() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    let error =
        get_model("zz-test-providr", "alpha").await.expect_err("unknown provider should error");

    assert_eq!(error.provider_id, "zz-test-providr");
    assert_eq!(error.model_id, "alpha");
    assert_eq!(error.suggestions, vec!["zz-test-provider"]);
}

#[tokio::test]
async fn get_model_reports_model_suggestions_for_missing_model() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    let error =
        get_model("zz-test-provider", "alpah").await.expect_err("unknown model should error");

    assert_eq!(error.suggestions.first().map(String::as_str), Some("alpha"));
}

#[tokio::test]
async fn get_model_reports_ambiguous_api_aliases_as_suggestions() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    let error = get_model("zz-test-provider", "shared-api")
        .await
        .expect_err("ambiguous api alias should error");

    assert_eq!(error.suggestions, vec!["alias-a", "alias-b"]);
}

#[tokio::test]
async fn default_model_uses_configured_model_without_loading_state() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(json!("manual-provider/manual-model"), json!({}))).await;

    let parsed = default_model().await.expect("configured default should parse");

    assert_eq!(parsed.provider_id, "manual-provider");
    assert_eq!(parsed.model_id, "manual-model");
}

#[tokio::test]
async fn default_model_falls_back_to_sorted_active_provider_model() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "zz-test-provider": {
                "models": {
                    "alpha": {"status": "active"},
                    "gpt-5-nano": {
                        "status": "active",
                        "limit": {"context": 1024, "output": 256}
                    }
                }
            }
        }),
    ))
    .await;

    let parsed = default_model().await.expect("fallback default should resolve");

    assert_eq!(parsed.provider_id, "zz-test-provider");
    assert_eq!(parsed.model_id, "gpt-5-nano");
}

#[tokio::test]
async fn default_model_errors_when_filter_removes_all_models() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, json!({}))).await;

    let error = default_model().await.expect_err("empty filtered state should error");

    assert_eq!(error, "没有可用的 provider");
}

#[tokio::test]
async fn get_small_model_uses_vibewindow_priority() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "vibewindow-local": {
                "models": {
                    "other-small": {"status": "active"},
                    "gpt-5-nano": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    let model = get_small_model("vibewindow-local").await.expect("small model should resolve");

    assert_eq!(model.id, "gpt-5-nano");
}

#[tokio::test]
async fn get_small_model_returns_none_without_priority_match() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "zz-test-provider": {
                "models": {
                    "alpha": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    assert!(get_small_model("zz-test-provider").await.is_none());
}

#[tokio::test]
async fn get_small_model_returns_none_for_missing_provider() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    assert!(get_small_model("missing-provider").await.is_none());
}

#[tokio::test]
async fn get_provider_returns_none_for_unknown_provider() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    assert!(get_provider("unknown-provider").await.is_none());
}

#[tokio::test]
async fn config_fingerprint_change_invalidates_cached_state() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "zz-test-provider": {
                "models": {
                    "alpha": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    assert!(get_model("zz-test-provider", "alpha").await.is_ok());

    std::thread::sleep(std::time::Duration::from_millis(5));
    write_json(
        &env.config_path(),
        &config_with_default(
            Value::Null,
            json!({
                "zz-test-provider": {
                    "models": {
                        "beta": {"status": "active"}
                    }
                }
            }),
        ),
    );

    assert!(get_model("zz-test-provider", "alpha").await.is_err());
    assert!(get_model("zz-test-provider", "beta").await.is_ok());
}

#[tokio::test]
async fn config_fingerprint_deletion_invalidates_cached_state() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "zz-test-provider": {
                "models": {
                    "alpha": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    assert!(get_model("zz-test-provider", "alpha").await.is_ok());

    std::fs::remove_file(env.config_path()).expect("config file should be removed");

    assert!(get_model("zz-test-provider", "alpha").await.is_err());
    let fingerprint = read_config_fingerprint();
    assert_eq!(fingerprint.path, env.config_path().to_string_lossy());
    assert!(!fingerprint.exists);
    assert_eq!(fingerprint.len, 0);
    assert_eq!(fingerprint.modified_nanos, 0);
}

#[tokio::test]
async fn init_preloads_model_cache_without_changing_query_results() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    init();

    assert!(get_provider("zz-test-provider").await.is_some());
}

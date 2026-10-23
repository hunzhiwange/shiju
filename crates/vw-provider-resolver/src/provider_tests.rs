//! Provider 聚合测试覆盖内置模型、用户配置、环境认证与远端发现合并后的最终查询视图。
//!
//! 用例分别检查设置态与运行态过滤、默认值和能力继承、别名建议、配置指纹失效及小模型选择；
//! 临时配置目录和环境守卫将进程缓存、凭据变量与文件变更隔离在单个串行测试场景中。

use std::ffi::OsString;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use tokio::sync::{Mutex, MutexGuard};

use super::*;

static TEST_LOCK: Mutex<()> = Mutex::const_new(());
static TEST_ROOT: OnceLock<PathBuf> = OnceLock::new();

// 聚合查询依赖进程环境和两层缓存，fixture 持锁期间统一建立并在退出时恢复测试边界。
struct ProviderTestEnv {
    _lock: MutexGuard<'static, ()>,
    config_dir: PathBuf,
    saved_env: Vec<(String, Option<OsString>)>,
}

impl ProviderTestEnv {
    async fn new() -> Self {
        let lock = TEST_LOCK.lock().await;
        let root = test_root();
        let config_dir = fresh_dir(&root, "config");

        std::fs::create_dir_all(&config_dir).expect("config dir should be created");

        let mut saved_env = save_env(&[
            "VIBEWINDOW_TEST_HOME",
            "VIBEWINDOW_WORKSPACE",
            "TEST_PROVIDER_KEY",
            "SECOND_PROVIDER_KEY",
            "VIBEWINDOW_PROVIDER_KEY",
        ]);

        unsafe {
            std::env::set_var("VIBEWINDOW_TEST_HOME", &config_dir);
            std::env::set_var("VIBEWINDOW_WORKSPACE", &config_dir);
            std::env::remove_var("TEST_PROVIDER_KEY");
            std::env::remove_var("SECOND_PROVIDER_KEY");
            std::env::remove_var("VIBEWINDOW_PROVIDER_KEY");
        }

        crate::models::invalidate_cache();
        for name in crate::models::get().await.values().flat_map(|provider| provider.env.iter()) {
            if saved_env.iter().any(|(saved, _)| saved == name) {
                continue;
            }
            saved_env.push((name.clone(), std::env::var_os(name)));
            unsafe { std::env::remove_var(name) };
        }
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

fn save_env(keys: &[&str]) -> Vec<(String, Option<OsString>)> {
    keys.iter().map(|key| ((*key).to_string(), std::env::var_os(key))).collect()
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
async fn provider_without_models_is_kept_for_settings_and_dropped_at_runtime() {
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

    let settings = list_for_settings().await;
    let provider = settings.get("empty-provider").expect("settings should keep editable provider");
    assert!(provider.models.is_empty());
    assert!(!list().await.contains_key("empty-provider"));
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
async fn cross_layer_unknown_model_uses_shared_default_limit() {
    let env = ProviderTestEnv::new().await;
    let fixture: Value = serde_json::from_str(include_str!(
        "../../vw-shared/assets/provider/cross_layer_unknown_model.json"
    ))
    .expect("cross-layer fixture should parse");
    let provider_id = fixture["provider_id"].as_str().unwrap();
    let model_id = fixture["model_id"].as_str().unwrap();
    let providers = json!({
        "models": {model_id: {"name": fixture["model_name"], "status": "active"}}
    });
    let mut provider = providers;
    provider["api"] = fixture["base_url"].clone();
    let mut provider_map = serde_json::Map::new();
    provider_map.insert(provider_id.to_string(), provider);
    env.write_config(config_with_default(Value::Null, Value::Object(provider_map))).await;

    let model = list_for_settings().await[provider_id].models[model_id].clone();
    let expected = vw_shared::provider::state::resolve_model_limit(None, None, None);
    assert_eq!(model.limit.context, expected.context);
    assert_eq!(model.limit.input, expected.input);
    assert_eq!(model.limit.output, expected.output);
}

#[tokio::test]
async fn cross_layer_explicit_model_preserves_limits_and_capabilities_after_reload() {
    let env = ProviderTestEnv::new().await;
    let fixture: Value = serde_json::from_str(include_str!(
        "../../vw-shared/assets/provider/cross_layer_explicit_model.json"
    ))
    .expect("cross-layer fixture should parse");
    let provider_id = fixture["provider_id"].as_str().unwrap();
    let model_id = fixture["model_id"].as_str().unwrap();
    let model = json!({
        "name": fixture["model_name"],
        "family": fixture["family"],
        "reasoning": fixture["reasoning"],
        "tool_call": fixture["tool_call"],
        "temperature": fixture["temperature"],
        "attachment": fixture["attachment"],
        "interleaved": fixture["interleaved"],
        "modalities": {
            "input": fixture["input_modalities"],
            "output": fixture["output_modalities"]
        },
        "limit": {
            "context": fixture["context_limit"],
            "input": fixture["input_limit"],
            "output": fixture["output_limit"]
        },
        "status": "active"
    });
    let mut provider = json!({"api": fixture["base_url"], "models": {}});
    provider["models"][model_id] = model;
    let mut providers = serde_json::Map::new();
    providers.insert(provider_id.to_string(), provider);
    env.write_config(config_with_default(Value::Null, Value::Object(providers))).await;

    let model = list_for_settings().await[provider_id].models[model_id].clone();
    assert_eq!(model.limit.context, fixture["context_limit"].as_u64().unwrap());
    assert_eq!(model.limit.input, Some(fixture["input_limit"].as_u64().unwrap()));
    assert_eq!(model.limit.output, fixture["output_limit"].as_u64().unwrap());
    assert_eq!(model.family.as_deref(), fixture["family"].as_str());
    assert!(model.capabilities.reasoning);
    assert!(!model.capabilities.toolcall);
    assert!(model.capabilities.temperature);
    assert!(model.capabilities.attachment);
    assert!(model.capabilities.input.text && model.capabilities.input.image);
    assert!(model.capabilities.output.text);
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
async fn custom_model_without_limits_uses_shared_defaults_and_explicit_limits_win() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "custom-limit-provider": {
                "models": {
                    "fallback-model": {"status": "active"},
                    "explicit-model": {
                        "status": "active",
                        "limit": {"context": 12_345, "input": 10_000, "output": 2_048}
                    }
                }
            }
        }),
    ))
    .await;

    let fallback = get_model("custom-limit-provider", "fallback-model")
        .await
        .expect("fallback model should resolve");
    assert_eq!(fallback.limit.context, vw_shared::provider::state::DEFAULT_MODEL_CONTEXT_LIMIT);
    assert_eq!(fallback.limit.input, Some(vw_shared::provider::state::DEFAULT_MODEL_INPUT_LIMIT));
    assert_eq!(fallback.limit.output, vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT);

    let explicit = get_model("custom-limit-provider", "explicit-model")
        .await
        .expect("explicit model should resolve");
    assert_eq!(explicit.limit.context, 12_345);
    assert_eq!(explicit.limit.input, Some(10_000));
    assert_eq!(explicit.limit.output, 2_048);
}

#[tokio::test]
async fn custom_model_explicit_capabilities_override_baseline_fields() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "custom-capability-provider": {
                "models": {
                    "custom-model": {
                        "status": "active",
                        "family": "custom-family",
                        "reasoning": true,
                        "tool_call": false,
                        "temperature": true,
                        "attachment": true,
                        "modalities": {
                            "input": ["text", "image"],
                            "output": ["text"]
                        },
                        "limit": {"context": 65536, "input": 60000, "output": 4096}
                    }
                }
            }
        }),
    ))
    .await;

    let model = get_model("custom-capability-provider", "custom-model")
        .await
        .expect("custom model should resolve");

    assert_eq!(model.family.as_deref(), Some("custom-family"));
    assert!(model.capabilities.reasoning);
    assert!(!model.capabilities.toolcall);
    assert!(model.capabilities.temperature);
    assert!(model.capabilities.attachment);
    assert!(model.capabilities.input.text);
    assert!(model.capabilities.input.image);
    assert!(model.capabilities.output.text);
    assert!(!model.capabilities.output.image);
    assert_eq!(model.limit.context, 65536);
    assert_eq!(model.limit.input, Some(60000));
    assert_eq!(model.limit.output, 4096);
}

#[tokio::test]
async fn new_model_inherits_unique_builtin_provider_api() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "deepseek": {
                "models": {
                    "deepseek-future": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    let model =
        get_model("deepseek", "deepseek-future").await.expect("configured model should resolve");

    assert_eq!(model.api.url, "https://api.deepseek.com");
    assert_eq!(model.api.adapter, default_adapter());
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

#[tokio::test]
async fn remote_models_merge_preserves_explicit_status_and_static_metadata() {
    let env = ProviderTestEnv::new().await;
    let listener = TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
    let address = listener.local_addr().expect("test listener should have an address");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("test server should accept a request");
        let mut request = [0_u8; 4096];
        let read = stream.read(&mut request).expect("request should be readable");
        let request = String::from_utf8_lossy(&request[..read]);
        assert!(request.starts_with("GET /v1/models HTTP/1.1\r\n"));
        assert!(
            request
                .lines()
                .any(|line| line.eq_ignore_ascii_case("authorization: Bearer resolver-secret"))
        );
        let body = r#"{"data":[{"id":"gpt-5"},{"id":"remote-resolver-model"}]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).expect("response should be written");
    });
    env.set_env("TEST_PROVIDER_KEY", "resolver-secret");
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "openai": {
                "api": format!("http://{address}/v1/"),
                "env": ["TEST_PROVIDER_KEY"],
                "models": {
                    "gpt-5": {"status": "disabled"}
                }
            }
        }),
    ))
    .await;

    let providers = list_for_settings().await;
    let provider = providers.get("openai").expect("OpenAI provider should resolve");
    let explicit = provider.models.get("gpt-5").expect("explicit model should remain");
    let remote =
        provider.models.get("remote-resolver-model").expect("remote model should be merged");

    assert_eq!(explicit.status, "disabled");
    assert_eq!(explicit.name, "GPT-5");
    assert!(explicit.capabilities.reasoning);
    assert_eq!(remote.status, "active");
    assert_eq!(remote.api.url, format!("http://{address}/v1"));
    server.join().expect("test server should stop");
}

#[tokio::test]
async fn remote_models_http_failure_preserves_existing_models() {
    let env = ProviderTestEnv::new().await;
    let listener = TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
    let address = listener.local_addr().expect("test listener should have an address");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("test server should accept a request");
        let mut request = [0_u8; 4096];
        let read = stream.read(&mut request).expect("request should be readable");
        assert!(String::from_utf8_lossy(&request[..read]).starts_with("GET /models HTTP/1.1\r\n"));
        let response =
            "HTTP/1.1 404 Not Found\r\nContent-Length: 7\r\nConnection: close\r\n\r\nmissing";
        stream.write_all(response.as_bytes()).expect("response should be written");
    });
    env.set_env("TEST_PROVIDER_KEY", "resolver-secret");
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "failure-provider": {
                "api": format!("http://{address}"),
                "env": ["TEST_PROVIDER_KEY"],
                "models": {
                    "configured-model": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    let providers = list().await;
    let provider =
        providers.get("failure-provider").expect("provider loading should degrade gracefully");

    assert!(provider.models.contains_key("configured-model"));
    assert_eq!(provider.models.len(), 1);
    server.join().expect("test server should stop");
}

#[tokio::test]
async fn custom_provider_reasoning_inherits_catalog_for_shiju_and_token_plan() {
    let env = ProviderTestEnv::new().await;
    env.write_config(json!({"providers": {
        "shiju": {"models": {"deepseek-v4.1-flash": {"status": "active"}}},
        "token-plan": {"api": "https://example.test/v1", "models": {
            "deepseek-v4-flash-0731": {"status": "active"},
            "qwen3.7-plus": {"status": "active"},
            "alias": {"id": "glm-5.2", "status": "active"},
            "deepseek-v4-pro": {"status": "active", "reasoning": false},
            "wan2.7-image": {"status": "active"},
            "unknown-model": {"status": "active"}
        }}
    }}))
    .await;
    let state = load_state_impl(false).await;
    assert!(state.providers["shiju"].models["deepseek-v4.1-flash"].capabilities.reasoning);
    for (id, expected) in [
        ("deepseek-v4.1-flash", vec!["low", "high", "max"]),
        ("deepseek-v4-pro-0813", vec!["high", "max"]),
        ("glm-5.3", vec!["low", "high", "max"]),
        ("qwen3.8-max", vec!["low", "medium", "xhigh"]),
        ("qwen3.7-plus", vec![]),
        ("kimi/kimi-k2.8-preview", vec![]),
    ] {
        let model = &state.providers["shiju"].models[id];
        assert_eq!(
            vw_shared::provider::reasoning::reasoning_levels(
                id,
                "openai-compatible",
                "shiju",
                "",
                model.capabilities.reasoning_options.as_deref(),
            ),
            expected,
            "{id}"
        );
    }

    let models = &state.providers["token-plan"].models;
    for id in ["deepseek-v4-flash-0731", "qwen3.7-plus", "alias"] {
        assert!(models[id].capabilities.reasoning, "{id}");
        assert_eq!(models[id].api.url, "https://example.test/v1");
    }
    for id in ["deepseek-v4-pro", "wan2.7-image", "unknown-model"] {
        assert!(!models[id].capabilities.reasoning, "{id}");
    }
}

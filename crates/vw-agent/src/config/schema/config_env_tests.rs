// 覆盖环境变量覆盖配置时的优先级、规范化及无效值处理。

use super::{
    Config,
    config_env::apply_env_overrides,
    proxy::{ProxyConfig, ProxyScope, runtime_proxy_config, set_runtime_proxy_config},
};
use std::ffi::OsString;
use std::path::Path;
use std::sync::MutexGuard;
use vw_config_types::skills::{SkillsDirectoryProvider, SkillsPromptInjectionMode};

const CONFIG_ENV_KEYS: &[&str] = &[
    "VIBEWINDOW_API_KEY",
    "API_KEY",
    "VIBEWINDOW_PROVIDER",
    "VIBEWINDOW_MODEL_PROVIDER",
    "MODEL_PROVIDER",
    "PROVIDER",
    "VIBEWINDOW_MODEL",
    "MODEL",
    "OPENAI_API_KEY",
    "VIBEWINDOW_WORKSPACE",
    "VIBEWINDOW_OPEN_SKILLS_ENABLED",
    "VIBEWINDOW_OPEN_SKILLS_DIR",
    "VIBEWINDOW_SKILLS_PROMPT_MODE",
    "VIBEWINDOW_SKILLS_DIRECTORY_PROVIDER",
    "VIBEWINDOW_GATEWAY_PORT",
    "PORT",
    "VIBEWINDOW_GATEWAY_HOST",
    "HOST",
    "VIBEWINDOW_ALLOW_PUBLIC_BIND",
    "VIBEWINDOW_TEMPERATURE",
    "VIBEWINDOW_REASONING_ENABLED",
    "REASONING_ENABLED",
    "VIBEWINDOW_REASONING_LEVEL",
    "REASONING_LEVEL",
    "VIBEWINDOW_MODEL_SUPPORT_VISION",
    "MODEL_SUPPORT_VISION",
    "VIBEWINDOW_WEB_SEARCH_ENABLED",
    "WEB_SEARCH_ENABLED",
    "VIBEWINDOW_WEB_SEARCH_PROVIDER",
    "WEB_SEARCH_PROVIDER",
    "VIBEWINDOW_BRAVE_API_KEY",
    "BRAVE_API_KEY",
    "VIBEWINDOW_WEB_SEARCH_MAX_RESULTS",
    "WEB_SEARCH_MAX_RESULTS",
    "VIBEWINDOW_WEB_SEARCH_TIMEOUT_SECS",
    "WEB_SEARCH_TIMEOUT_SECS",
    "VIBEWINDOW_STORAGE_PROVIDER",
    "VIBEWINDOW_STORAGE_DB_URL",
    "VIBEWINDOW_STORAGE_CONNECT_TIMEOUT_SECS",
    "VIBEWINDOW_PROXY_ENABLED",
    "VIBEWINDOW_HTTP_PROXY",
    "HTTP_PROXY",
    "VIBEWINDOW_HTTPS_PROXY",
    "HTTPS_PROXY",
    "VIBEWINDOW_ALL_PROXY",
    "ALL_PROXY",
    "VIBEWINDOW_NO_PROXY",
    "NO_PROXY",
    "VIBEWINDOW_PROXY_SCOPE",
    "VIBEWINDOW_PROXY_SERVICES",
    "http_proxy",
    "https_proxy",
    "all_proxy",
    "no_proxy",
];

fn env_lock() -> MutexGuard<'static, ()> {
    crate::test_support::global_env_lock().lock().unwrap_or_else(|error| error.into_inner())
}

struct ConfigEnvGuard {
    _lock: MutexGuard<'static, ()>,
    _runtime_proxy_lock: MutexGuard<'static, ()>,
    previous_env: Vec<(&'static str, Option<OsString>)>,
    previous_proxy: ProxyConfig,
}

impl ConfigEnvGuard {
    fn new() -> Self {
        let lock = env_lock();
        let runtime_proxy_lock = crate::test_support::global_runtime_proxy_lock()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let previous_env =
            CONFIG_ENV_KEYS.iter().map(|key| (*key, std::env::var_os(*key))).collect::<Vec<_>>();
        let previous_proxy = runtime_proxy_config();

        for &key in CONFIG_ENV_KEYS {
            unsafe {
                std::env::remove_var(key);
            }
        }

        Self { _lock: lock, _runtime_proxy_lock: runtime_proxy_lock, previous_env, previous_proxy }
    }

    fn set(&self, key: &'static str, value: &str) {
        assert!(CONFIG_ENV_KEYS.contains(&key), "untracked config environment key: {key}");
        unsafe {
            std::env::set_var(key, value);
        }
    }

    fn remove(&self, key: &'static str) {
        assert!(CONFIG_ENV_KEYS.contains(&key), "untracked config environment key: {key}");
        unsafe {
            std::env::remove_var(key);
        }
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
        set_runtime_proxy_config(self.previous_proxy.clone());
    }
}

#[test]
fn apply_env_overrides_has_stable_in_place_signature() {
    let apply: fn(&mut Config) = apply_env_overrides;

    let _ = apply;
}

#[test]
fn apply_env_overrides_prefers_vibewindow_names_over_generic_fallbacks() {
    let env = ConfigEnvGuard::new();
    for (key, value) in [
        ("VIBEWINDOW_API_KEY", "prefixed-key"),
        ("API_KEY", "generic-key"),
        ("VIBEWINDOW_PROVIDER", "prefixed-provider"),
        ("VIBEWINDOW_MODEL_PROVIDER", "model-provider"),
        ("MODEL_PROVIDER", "generic-model-provider"),
        ("PROVIDER", "legacy-provider"),
        ("VIBEWINDOW_MODEL", "prefixed-model"),
        ("MODEL", "generic-model"),
        ("VIBEWINDOW_GATEWAY_PORT", "19090"),
        ("PORT", "29090"),
        ("VIBEWINDOW_GATEWAY_HOST", "127.0.0.2"),
        ("HOST", "127.0.0.3"),
        ("VIBEWINDOW_REASONING_ENABLED", "yes"),
        ("REASONING_ENABLED", "no"),
        ("VIBEWINDOW_MODEL_SUPPORT_VISION", "off"),
        ("MODEL_SUPPORT_VISION", "on"),
        ("VIBEWINDOW_WEB_SEARCH_ENABLED", "0"),
        ("WEB_SEARCH_ENABLED", "1"),
        ("VIBEWINDOW_WEB_SEARCH_PROVIDER", "brave"),
        ("WEB_SEARCH_PROVIDER", "serper"),
        ("VIBEWINDOW_BRAVE_API_KEY", "prefixed-brave-key"),
        ("BRAVE_API_KEY", "generic-brave-key"),
        ("VIBEWINDOW_WEB_SEARCH_MAX_RESULTS", "7"),
        ("WEB_SEARCH_MAX_RESULTS", "8"),
        ("VIBEWINDOW_WEB_SEARCH_TIMEOUT_SECS", "12"),
        ("WEB_SEARCH_TIMEOUT_SECS", "13"),
    ] {
        env.set(key, value);
    }

    let mut config = Config::default();
    apply_env_overrides(&mut config);

    assert_eq!(config.api_key.as_deref(), Some("prefixed-key"));
    assert_eq!(config.default_provider.as_deref(), Some("prefixed-provider"));
    assert_eq!(config.default_model.as_deref(), Some("prefixed-model"));
    assert_eq!(config.gateway.port, 19090);
    assert_eq!(config.gateway.host, "127.0.0.2");
    assert_eq!(config.runtime.reasoning_enabled, Some(true));
    assert_eq!(config.model_support_vision, Some(false));
    assert!(!config.web_search.enabled);
    assert_eq!(config.web_search.provider, "brave");
    assert_eq!(config.web_search.brave_api_key.as_deref(), Some("prefixed-brave-key"));
    assert_eq!(config.web_search.max_results, 7);
    assert_eq!(config.web_search.timeout_secs, 12);
}

#[test]
fn apply_env_overrides_uses_generic_fallback_values() {
    let env = ConfigEnvGuard::new();
    for (key, value) in [
        ("API_KEY", "generic-key"),
        ("MODEL_PROVIDER", "generic-provider"),
        ("MODEL", "generic-model"),
        ("PORT", "29090"),
        ("HOST", "127.0.0.4"),
        ("REASONING_ENABLED", "false"),
        ("MODEL_SUPPORT_VISION", "true"),
        ("WEB_SEARCH_ENABLED", "true"),
        ("WEB_SEARCH_PROVIDER", "serper"),
        ("BRAVE_API_KEY", "generic-brave-key"),
        ("WEB_SEARCH_MAX_RESULTS", "8"),
        ("WEB_SEARCH_TIMEOUT_SECS", "14"),
    ] {
        env.set(key, value);
    }

    let mut config = Config::default();
    apply_env_overrides(&mut config);

    assert_eq!(config.api_key.as_deref(), Some("generic-key"));
    assert_eq!(config.default_provider.as_deref(), Some("generic-provider"));
    assert_eq!(config.default_model.as_deref(), Some("generic-model"));
    assert_eq!(config.gateway.port, 29090);
    assert_eq!(config.gateway.host, "127.0.0.4");
    assert_eq!(config.runtime.reasoning_enabled, Some(false));
    assert_eq!(config.model_support_vision, Some(true));
    assert!(config.web_search.enabled);
    assert_eq!(config.web_search.provider, "serper");
    assert_eq!(config.web_search.brave_api_key.as_deref(), Some("generic-brave-key"));
    assert_eq!(config.web_search.max_results, 8);
    assert_eq!(config.web_search.timeout_secs, 14);
}

#[test]
fn legacy_provider_replaces_builtin_shiju_default() {
    let env = ConfigEnvGuard::new();
    env.set("PROVIDER", "legacy-provider");

    let mut missing = Config::default();
    missing.default_provider = None;
    apply_env_overrides(&mut missing);
    assert_eq!(missing.default_provider.as_deref(), Some("legacy-provider"));

    let mut default_provider = Config::default();
    apply_env_overrides(&mut default_provider);
    assert_eq!(default_provider.default_provider.as_deref(), Some("legacy-provider"));

    let mut explicit = Config::default();
    explicit.default_provider = Some("openrouter".to_string());
    apply_env_overrides(&mut explicit);
    assert_eq!(explicit.default_provider.as_deref(), Some("openrouter"));
}

#[test]
fn valid_workspace_skills_and_storage_values_are_normalized() {
    let env = ConfigEnvGuard::new();
    for (key, value) in [
        ("VIBEWINDOW_WORKSPACE", "/tmp/vibewindow-config-env"),
        ("VIBEWINDOW_OPEN_SKILLS_ENABLED", "ON"),
        ("VIBEWINDOW_OPEN_SKILLS_DIR", "  /tmp/shared-skills  "),
        ("VIBEWINDOW_SKILLS_PROMPT_MODE", " FULL "),
        ("VIBEWINDOW_SKILLS_DIRECTORY_PROVIDER", "claude-code"),
        ("VIBEWINDOW_STORAGE_PROVIDER", " postgres "),
        ("VIBEWINDOW_STORAGE_DB_URL", " postgres://db.example/vw "),
        ("VIBEWINDOW_STORAGE_CONNECT_TIMEOUT_SECS", "9"),
    ] {
        env.set(key, value);
    }

    let mut config = Config::default();
    apply_env_overrides(&mut config);

    assert_eq!(config.workspace_dir, Path::new("/tmp/vibewindow-config-env").join("workspace"));
    assert!(config.skills.open_skills_enabled);
    assert_eq!(config.skills.open_skills_dir.as_deref(), Some("/tmp/shared-skills"));
    assert_eq!(config.skills.prompt_injection_mode, SkillsPromptInjectionMode::Full);
    assert_eq!(config.skills.directory_provider, SkillsDirectoryProvider::Claude);
    assert_eq!(config.storage.provider.config.provider, "postgres");
    assert_eq!(config.storage.provider.config.db_url.as_deref(), Some("postgres://db.example/vw"));
    assert_eq!(config.storage.provider.config.connect_timeout_secs, Some(9));
}

#[test]
fn boolean_overrides_preserve_multi_alias_flags_but_disable_strict_flags() {
    let env = ConfigEnvGuard::new();
    for (key, value) in [
        ("VIBEWINDOW_OPEN_SKILLS_ENABLED", "off"),
        ("VIBEWINDOW_ALLOW_PUBLIC_BIND", "yes"),
        ("VIBEWINDOW_REASONING_ENABLED", "not-a-bool"),
        ("VIBEWINDOW_MODEL_SUPPORT_VISION", "not-a-bool"),
        ("VIBEWINDOW_WEB_SEARCH_ENABLED", "yes"),
    ] {
        env.set(key, value);
    }

    let mut config = Config::default();
    config.skills.open_skills_enabled = true;
    config.gateway.allow_public_bind = true;
    config.runtime.reasoning_enabled = Some(true);
    config.model_support_vision = Some(true);
    config.web_search.enabled = true;
    apply_env_overrides(&mut config);

    assert!(!config.skills.open_skills_enabled);
    assert!(!config.gateway.allow_public_bind);
    assert_eq!(config.runtime.reasoning_enabled, Some(true));
    assert_eq!(config.model_support_vision, Some(true));
    assert!(!config.web_search.enabled);
}

#[test]
fn blank_and_invalid_values_leave_structured_configuration_unchanged() {
    let env = ConfigEnvGuard::new();
    for (key, value) in [
        ("VIBEWINDOW_API_KEY", ""),
        ("VIBEWINDOW_PROVIDER", ""),
        ("VIBEWINDOW_MODEL", ""),
        ("VIBEWINDOW_WORKSPACE", "  "),
        ("VIBEWINDOW_OPEN_SKILLS_ENABLED", "  "),
        ("VIBEWINDOW_OPEN_SKILLS_DIR", "  "),
        ("VIBEWINDOW_SKILLS_PROMPT_MODE", "verbose"),
        ("VIBEWINDOW_SKILLS_DIRECTORY_PROVIDER", "unknown"),
        ("VIBEWINDOW_GATEWAY_PORT", "70000"),
        ("VIBEWINDOW_GATEWAY_HOST", ""),
        ("VIBEWINDOW_TEMPERATURE", "2.01"),
        ("VIBEWINDOW_REASONING_ENABLED", "unknown"),
        ("VIBEWINDOW_REASONING_LEVEL", "urgent"),
        ("VIBEWINDOW_MODEL_SUPPORT_VISION", "unknown"),
        ("VIBEWINDOW_WEB_SEARCH_PROVIDER", "  "),
        ("VIBEWINDOW_BRAVE_API_KEY", "  "),
        ("VIBEWINDOW_WEB_SEARCH_MAX_RESULTS", "0"),
        ("VIBEWINDOW_WEB_SEARCH_TIMEOUT_SECS", "0"),
        ("VIBEWINDOW_STORAGE_PROVIDER", "  "),
        ("VIBEWINDOW_STORAGE_DB_URL", "  "),
        ("VIBEWINDOW_STORAGE_CONNECT_TIMEOUT_SECS", "0"),
    ] {
        env.set(key, value);
    }

    let mut config = Config::default();
    let original_workspace = config.workspace_dir.clone();
    config.api_key = Some("configured-key".to_string());
    config.default_provider = Some("configured-provider".to_string());
    config.default_model = Some("configured-model".to_string());
    config.skills.open_skills_enabled = true;
    config.skills.open_skills_dir = Some("configured-skills".to_string());
    config.skills.prompt_injection_mode = SkillsPromptInjectionMode::Full;
    config.skills.directory_provider = SkillsDirectoryProvider::Cursor;
    config.gateway.port = 1234;
    config.gateway.host = "configured-host".to_string();
    config.default_temperature = 1.25;
    config.runtime.reasoning_enabled = Some(false);
    config.runtime.reasoning_level = Some("low".to_string());
    config.model_support_vision = Some(false);
    config.web_search.provider = "configured-search".to_string();
    config.web_search.brave_api_key = Some("configured-brave-key".to_string());
    config.web_search.max_results = 6;
    config.web_search.timeout_secs = 8;
    config.storage.provider.config.provider = "sqlite".to_string();
    config.storage.provider.config.db_url = Some("sqlite://configured".to_string());
    config.storage.provider.config.connect_timeout_secs = Some(5);
    apply_env_overrides(&mut config);

    assert_eq!(config.api_key.as_deref(), Some("configured-key"));
    assert_eq!(config.default_provider.as_deref(), Some("configured-provider"));
    assert_eq!(config.default_model.as_deref(), Some("configured-model"));
    assert_eq!(config.workspace_dir, original_workspace);
    assert!(config.skills.open_skills_enabled);
    assert_eq!(config.skills.open_skills_dir.as_deref(), Some("configured-skills"));
    assert_eq!(config.skills.prompt_injection_mode, SkillsPromptInjectionMode::Full);
    assert_eq!(config.skills.directory_provider, SkillsDirectoryProvider::Cursor);
    assert_eq!(config.gateway.port, 1234);
    assert_eq!(config.gateway.host, "configured-host");
    assert_eq!(config.default_temperature, 1.25);
    assert_eq!(config.runtime.reasoning_enabled, Some(false));
    assert_eq!(config.runtime.reasoning_level.as_deref(), Some("low"));
    assert_eq!(config.model_support_vision, Some(false));
    assert_eq!(config.web_search.provider, "configured-search");
    assert_eq!(config.web_search.brave_api_key.as_deref(), Some("configured-brave-key"));
    assert_eq!(config.web_search.max_results, 6);
    assert_eq!(config.web_search.timeout_secs, 8);
    assert_eq!(config.storage.provider.config.provider, "sqlite");
    assert_eq!(config.storage.provider.config.db_url.as_deref(), Some("sqlite://configured"));
    assert_eq!(config.storage.provider.config.connect_timeout_secs, Some(5));
}

#[test]
fn numeric_overrides_accept_inclusive_boundaries() {
    let env = ConfigEnvGuard::new();
    for (key, value) in [
        ("VIBEWINDOW_GATEWAY_PORT", "0"),
        ("VIBEWINDOW_TEMPERATURE", "0"),
        ("VIBEWINDOW_WEB_SEARCH_MAX_RESULTS", "1"),
        ("VIBEWINDOW_WEB_SEARCH_TIMEOUT_SECS", "1"),
        ("VIBEWINDOW_STORAGE_CONNECT_TIMEOUT_SECS", "1"),
    ] {
        env.set(key, value);
    }

    let mut lower = Config::default();
    apply_env_overrides(&mut lower);
    assert_eq!(lower.gateway.port, 0);
    assert_eq!(lower.default_temperature, 0.0);
    assert_eq!(lower.web_search.max_results, 1);
    assert_eq!(lower.web_search.timeout_secs, 1);
    assert_eq!(lower.storage.provider.config.connect_timeout_secs, Some(1));

    for (key, value) in [
        ("VIBEWINDOW_GATEWAY_PORT", "65535"),
        ("VIBEWINDOW_TEMPERATURE", "2"),
        ("VIBEWINDOW_WEB_SEARCH_MAX_RESULTS", "10"),
    ] {
        env.set(key, value);
    }

    let mut upper = Config::default();
    apply_env_overrides(&mut upper);
    assert_eq!(upper.gateway.port, u16::MAX);
    assert_eq!(upper.default_temperature, 2.0);
    assert_eq!(upper.web_search.max_results, 10);
}

#[test]
fn reasoning_level_alias_prefers_prefixed_value_and_normalizes_fallback() {
    let env = ConfigEnvGuard::new();
    env.set("VIBEWINDOW_REASONING_LEVEL", "X_HIGH");
    env.set("REASONING_LEVEL", "low");

    let mut prefixed = Config::default();
    apply_env_overrides(&mut prefixed);
    assert_eq!(prefixed.runtime.reasoning_level.as_deref(), Some("xhigh"));

    env.remove("VIBEWINDOW_REASONING_LEVEL");
    env.set("REASONING_LEVEL", "ME-DIUM");
    let mut fallback = Config::default();
    apply_env_overrides(&mut fallback);
    assert_eq!(fallback.runtime.reasoning_level.as_deref(), Some("medium"));
}

#[test]
fn proxy_overrides_normalize_and_auto_enable_valid_service_scope() {
    let env = ConfigEnvGuard::new();
    for (key, value) in [
        ("VIBEWINDOW_HTTP_PROXY", "  http://127.0.0.1:8080  "),
        ("HTTP_PROXY", "http://127.0.0.1:9090"),
        ("VIBEWINDOW_NO_PROXY", " localhost,127.0.0.1,localhost "),
        ("VIBEWINDOW_PROXY_SCOPE", "services"),
        ("VIBEWINDOW_PROXY_SERVICES", " TOOL.BROWSER,provider.openai,tool.browser "),
    ] {
        env.set(key, value);
    }

    let mut config = Config::default();
    apply_env_overrides(&mut config);

    assert!(config.proxy.enabled);
    assert_eq!(config.proxy.http_proxy.as_deref(), Some("http://127.0.0.1:8080"));
    assert_eq!(config.proxy.no_proxy, vec!["127.0.0.1".to_string(), "localhost".to_string()]);
    assert_eq!(config.proxy.scope, ProxyScope::Services);
    assert_eq!(
        config.proxy.services,
        vec!["provider.openai".to_string(), "tool.browser".to_string()]
    );
    let runtime = runtime_proxy_config();
    assert!(runtime.enabled);
    assert_eq!(runtime.scope, ProxyScope::Services);
}

#[test]
fn explicit_proxy_disable_prevents_url_override_from_auto_enabling() {
    let env = ConfigEnvGuard::new();
    env.set("VIBEWINDOW_PROXY_ENABLED", "off");
    env.set("VIBEWINDOW_HTTPS_PROXY", " https://127.0.0.1:8443 ");

    let mut config = Config::default();
    apply_env_overrides(&mut config);

    assert!(!config.proxy.enabled);
    assert_eq!(config.proxy.https_proxy.as_deref(), Some("https://127.0.0.1:8443"));
}

#[test]
fn all_proxy_override_is_normalized_and_auto_enables_proxy() {
    let env = ConfigEnvGuard::new();
    env.set("VIBEWINDOW_ALL_PROXY", "  http://127.0.0.1:1080  ");

    let mut config = Config::default();
    apply_env_overrides(&mut config);

    assert!(config.proxy.enabled);
    assert_eq!(config.proxy.all_proxy.as_deref(), Some("http://127.0.0.1:1080"));
}

#[test]
fn environment_proxy_scope_applies_normalized_process_variables() {
    let env = ConfigEnvGuard::new();
    env.set("VIBEWINDOW_PROXY_ENABLED", "true");
    env.set("VIBEWINDOW_HTTP_PROXY", " http://127.0.0.1:8080 ");
    env.set("VIBEWINDOW_NO_PROXY", " localhost,127.0.0.1 ");
    env.set("VIBEWINDOW_PROXY_SCOPE", "environment");

    let mut config = Config::default();
    apply_env_overrides(&mut config);

    assert!(config.proxy.enabled);
    assert_eq!(std::env::var("HTTP_PROXY").as_deref(), Ok("http://127.0.0.1:8080"));
    assert_eq!(std::env::var("http_proxy").as_deref(), Ok("http://127.0.0.1:8080"));
    assert_eq!(std::env::var("NO_PROXY").as_deref(), Ok("127.0.0.1,localhost"));
    assert_eq!(std::env::var("no_proxy").as_deref(), Ok("127.0.0.1,localhost"));
}

#[test]
fn invalid_proxy_configurations_are_disabled_before_runtime_use() {
    let env = ConfigEnvGuard::new();
    env.set("VIBEWINDOW_PROXY_ENABLED", "true");

    let mut missing_url = Config::default();
    apply_env_overrides(&mut missing_url);
    assert!(!missing_url.proxy.enabled);

    env.remove("VIBEWINDOW_PROXY_ENABLED");
    env.set("VIBEWINDOW_HTTP_PROXY", "ftp://proxy.example");
    let mut invalid_url = Config::default();
    apply_env_overrides(&mut invalid_url);
    assert!(!invalid_url.proxy.enabled);
    assert_eq!(invalid_url.proxy.http_proxy.as_deref(), Some("ftp://proxy.example"));

    env.set("VIBEWINDOW_HTTP_PROXY", "http://127.0.0.1:8080");
    env.set("VIBEWINDOW_PROXY_SERVICES", "unsupported.service");
    let mut invalid_service = Config::default();
    apply_env_overrides(&mut invalid_service);
    assert!(!invalid_service.proxy.enabled);
    assert_eq!(invalid_service.proxy.services, vec!["unsupported.service".to_string()]);
}

// 覆盖配置扩展接口、模型提供商配置及工作区覆盖规则。

use super::config::{apply_named_model_provider_profile, apply_workspace_override};
use super::{Config, ConfigExt, ModelProviderConfig};
use std::ffi::{OsStr, OsString};
use std::sync::{Mutex, OnceLock};

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

struct EnvVarGuard {
    key: &'static str,
    old_value: Option<OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: impl AsRef<OsStr>) -> Self {
        let old_value = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, value);
        }

        Self { key, old_value }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(old_value) = &self.old_value {
                std::env::set_var(self.key, old_value);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }
}

// 正常值、缺省值与非法输入成组验证，用于固定配置兼容边界和失败语义。
#[test]
fn config_ext_validate_accepts_default_config() {
    Config::default().validate().unwrap();
}

#[test]
fn config_ext_async_methods_return_send_futures() {
    fn assert_send<T: Send>(_: T) {}

    let config = Config::default();

    assert_send(Config::load_or_init());
    assert_send(config.save());
}

#[test]
fn config_ext_apply_env_overrides_is_callable() {
    let mut config = Config::default();

    config.apply_env_overrides();
}

// 错误路径还要确认原配置或存储保持不变，防止校验失败后留下部分更新。
#[test]
fn named_model_provider_profile_returns_when_provider_is_missing_or_unknown() {
    let mut missing = Config::default();
    missing.default_provider = None;

    apply_named_model_provider_profile(&mut missing);

    assert_eq!(missing.default_provider, None);
    assert_eq!(missing.api_url, None);

    let mut unknown = Config::default();
    unknown.default_provider = Some("missing-profile".to_string());

    apply_named_model_provider_profile(&mut unknown);

    assert_eq!(unknown.default_provider.as_deref(), Some("missing-profile"));
    assert_eq!(unknown.api_url, None);
}

#[test]
fn named_model_provider_profile_sets_custom_provider_from_base_url() {
    let mut config = Config::default();
    config.default_provider = Some("local-profile".to_string());
    config.model_providers.insert(
        "local-profile".to_string(),
        ModelProviderConfig {
            base_url: Some("https://models.example/v1".to_string()),
            ..Default::default()
        },
    );

    apply_named_model_provider_profile(&mut config);

    assert_eq!(config.api_url.as_deref(), Some("https://models.example/v1"));
    assert_eq!(config.default_provider.as_deref(), Some("custom:https://models.example/v1"));
}

#[test]
fn named_model_provider_profile_preserves_existing_api_url_and_key() {
    let mut config = Config::default();
    config.default_provider = Some("PROFILE".to_string());
    config.api_url = Some("https://configured.example/v1".to_string());
    config.api_key = Some("configured-key".to_string());
    config.model_providers.insert(
        "profile".to_string(),
        ModelProviderConfig {
            base_url: Some("  https://models.example/v1  ".to_string()),
            requires_openai_auth: true,
            ..Default::default()
        },
    );

    apply_named_model_provider_profile(&mut config);

    assert_eq!(config.api_url.as_deref(), Some("https://configured.example/v1"));
    assert_eq!(config.api_key.as_deref(), Some("configured-key"));
    assert_eq!(config.default_provider.as_deref(), Some("custom:https://models.example/v1"));
}

#[test]
fn named_model_provider_profile_reads_openai_key_for_auth_profiles() {
    let _guard = env_lock();
    let _openai_key = EnvVarGuard::set("OPENAI_API_KEY", "  env-openai-key  ");
    let mut config = Config::default();
    config.default_provider = Some("auth-profile".to_string());
    config.model_providers.insert(
        "auth-profile".to_string(),
        ModelProviderConfig { requires_openai_auth: true, ..Default::default() },
    );

    apply_named_model_provider_profile(&mut config);

    assert_eq!(config.api_key.as_deref(), Some("env-openai-key"));
    assert_eq!(config.default_provider.as_deref(), Some("auth-profile"));
}

#[test]
fn named_model_provider_profile_replaces_blank_top_level_url_and_key() {
    let _guard = env_lock();
    let _openai_key = EnvVarGuard::set("OPENAI_API_KEY", "  env-openai-key  ");
    let mut config = Config::default();
    config.default_provider = Some("auth-profile".to_string());
    config.api_url = Some(" \n ".to_string());
    config.api_key = Some("   ".to_string());
    config.model_providers.insert(
        "auth-profile".to_string(),
        ModelProviderConfig {
            base_url: Some("  https://models.example/v1  ".to_string()),
            requires_openai_auth: true,
            ..Default::default()
        },
    );

    apply_named_model_provider_profile(&mut config);

    assert_eq!(config.api_url.as_deref(), Some("https://models.example/v1"));
    assert_eq!(config.api_key.as_deref(), Some("env-openai-key"));
    assert_eq!(config.default_provider.as_deref(), Some("custom:https://models.example/v1"));
}

#[test]
fn named_model_provider_profile_does_not_copy_key_without_auth_requirement() {
    let _guard = env_lock();
    let _openai_key = EnvVarGuard::set("OPENAI_API_KEY", "env-openai-key");
    let mut config = Config::default();
    config.default_provider = Some("public-profile".to_string());
    config.model_providers.insert(
        "public-profile".to_string(),
        ModelProviderConfig { requires_openai_auth: false, ..Default::default() },
    );

    apply_named_model_provider_profile(&mut config);

    assert_eq!(config.api_key, None);
    assert_eq!(config.default_provider.as_deref(), Some("public-profile"));
}

#[test]
fn named_model_provider_profile_ignores_blank_profile_values() {
    let mut config = Config::default();
    config.default_provider = Some("blank-profile".to_string());
    config.model_providers.insert(
        "blank-profile".to_string(),
        ModelProviderConfig {
            name: Some("   ".to_string()),
            base_url: Some(" \n ".to_string()),
            wire_api: Some("legacy".to_string()),
            ..Default::default()
        },
    );

    apply_named_model_provider_profile(&mut config);

    assert_eq!(config.api_url, None);
    assert_eq!(config.default_provider.as_deref(), Some("blank-profile"));
}

#[test]
fn named_model_provider_profile_keeps_provider_when_profile_name_matches_key() {
    let mut config = Config::default();
    config.default_provider = Some("alias".to_string());
    config.model_providers.insert(
        "alias".to_string(),
        ModelProviderConfig { name: Some(" Alias ".to_string()), ..Default::default() },
    );

    apply_named_model_provider_profile(&mut config);

    assert_eq!(config.default_provider.as_deref(), Some("alias"));
}

#[test]
fn named_model_provider_profile_uses_responses_provider_and_profile_name() {
    let mut responses = Config::default();
    responses.default_provider = Some("codex".to_string());
    responses.model_providers.insert(
        "codex".to_string(),
        ModelProviderConfig {
            name: Some("ignored-name".to_string()),
            wire_api: Some("  ReSpOnSeS  ".to_string()),
            ..Default::default()
        },
    );

    apply_named_model_provider_profile(&mut responses);
    assert_eq!(responses.default_provider.as_deref(), Some("openai-codex"));

    let mut named = Config::default();
    named.default_provider = Some("alias".to_string());
    named.model_providers.insert(
        "alias".to_string(),
        ModelProviderConfig { name: Some("openrouter".to_string()), ..Default::default() },
    );

    apply_named_model_provider_profile(&mut named);
    assert_eq!(named.default_provider.as_deref(), Some("openrouter"));
}

#[test]
fn named_model_provider_profile_prefers_distinct_name_over_custom_url() {
    let mut config = Config::default();
    config.default_provider = Some("alias".to_string());
    config.model_providers.insert(
        "alias".to_string(),
        ModelProviderConfig {
            name: Some("openrouter".to_string()),
            base_url: Some("https://models.example/v1".to_string()),
            ..Default::default()
        },
    );

    apply_named_model_provider_profile(&mut config);

    assert_eq!(config.api_url.as_deref(), Some("https://models.example/v1"));
    assert_eq!(config.default_provider.as_deref(), Some("openrouter"));
}

#[test]
fn named_model_provider_profile_uses_custom_url_when_name_matches_key() {
    let mut config = Config::default();
    config.default_provider = Some("alias".to_string());
    config.model_providers.insert(
        "alias".to_string(),
        ModelProviderConfig {
            name: Some(" Alias ".to_string()),
            base_url: Some("https://models.example/v1".to_string()),
            ..Default::default()
        },
    );

    apply_named_model_provider_profile(&mut config);

    assert_eq!(config.api_url.as_deref(), Some("https://models.example/v1"));
    assert_eq!(config.default_provider.as_deref(), Some("custom:https://models.example/v1"));
}

#[test]
fn workspace_override_ignores_blank_and_resolves_non_blank_paths() {
    let mut config = Config::default();
    let original = config.workspace_dir.clone();

    apply_workspace_override(&mut config, "   ");
    assert_eq!(config.workspace_dir, original);

    apply_workspace_override(&mut config, "/tmp/vw-workspace-override");
    assert_eq!(
        config.workspace_dir,
        std::path::Path::new("/tmp/vw-workspace-override").join("workspace")
    );
}

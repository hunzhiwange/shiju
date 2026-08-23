// 覆盖代理作用域、地址规范化及运行时环境应用规则。

use super::*;
use std::ffi::OsString;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::MutexGuard;

const PROXY_ENV_KEYS: &[&str] = &[
    "HTTP_PROXY",
    "http_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "ALL_PROXY",
    "all_proxy",
    "NO_PROXY",
    "no_proxy",
];

struct ProxyEnvSnapshot(Vec<(&'static str, Option<OsString>)>);

impl ProxyEnvSnapshot {
    fn capture() -> Self {
        Self(PROXY_ENV_KEYS.iter().map(|&key| (key, std::env::var_os(key))).collect())
    }
}

impl Drop for ProxyEnvSnapshot {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            // SAFETY: tests that mutate proxy environment variables hold the proxy-state lock.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(*key, value),
                    None => std::env::remove_var(*key),
                }
            }
        }
    }
}

struct RuntimeProxySnapshot(ProxyConfig);

impl RuntimeProxySnapshot {
    fn capture() -> Self {
        Self(runtime_proxy_config())
    }
}

impl Drop for RuntimeProxySnapshot {
    fn drop(&mut self) {
        set_runtime_proxy_config(self.0.clone());
    }
}

struct RuntimeProxyPoisonReset;

impl Drop for RuntimeProxyPoisonReset {
    fn drop(&mut self) {
        runtime_proxy_client_cache().clear_poison();
        runtime_proxy_state().clear_poison();
    }
}

fn lock_global_proxy_state() -> MutexGuard<'static, ()> {
    crate::test_support::global_runtime_proxy_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn enabled_proxy(url: &str) -> ProxyConfig {
    ProxyConfig { enabled: true, http_proxy: Some(url.to_string()), ..Default::default() }
}

// 正常值、缺省值与非法输入成组验证，用于固定配置兼容边界和失败语义。
#[test]
fn proxy_parsers_accept_all_aliases_case_and_whitespace() {
    for raw in ["1", "true", "yes", "on", " TRUE ", "On"] {
        assert_eq!(parse_proxy_enabled(raw), Some(true), "raw value: {raw}");
    }
    for raw in ["0", "false", "no", "off", " FALSE ", "Off"] {
        assert_eq!(parse_proxy_enabled(raw), Some(false), "raw value: {raw}");
    }
    for raw in ["environment", "env", " ENV "] {
        assert_eq!(parse_proxy_scope(raw), Some(ProxyScope::Environment), "raw value: {raw}");
    }
    for raw in ["vibewindow", "internal", "core", " CORE "] {
        assert_eq!(parse_proxy_scope(raw), Some(ProxyScope::Vibewindow), "raw value: {raw}");
    }
    for raw in ["services", "service", " SERVICE "] {
        assert_eq!(parse_proxy_scope(raw), Some(ProxyScope::Services), "raw value: {raw}");
    }

    assert_eq!(parse_proxy_enabled("sometimes"), None);
    assert_eq!(parse_proxy_enabled(" "), None);
    assert_eq!(parse_proxy_scope("system"), None);
    assert_eq!(parse_proxy_scope(" "), None);
}

#[test]
fn proxy_validation_accepts_every_supported_url_scheme() {
    for url in [
        "http://proxy.example:8080",
        "https://proxy.example:8443",
        "socks5://proxy.example:1080",
        "socks5h://proxy.example:1080",
    ] {
        validate_proxy_config(&enabled_proxy(url))
            .unwrap_or_else(|error| panic!("supported proxy URL {url} should be valid: {error}"));
    }
}

#[test]
fn proxy_validation_accepts_normalized_supported_service_selectors() {
    let config = ProxyConfig {
        enabled: true,
        http_proxy: Some(" http://proxy.example:8080 ".to_string()),
        scope: ProxyScope::Services,
        services: vec![
            " PROVIDER.OPENAI, channel.* ".to_string(),
            " tool.http_request ".to_string(),
        ],
        ..Default::default()
    };

    assert!(validate_proxy_config(&config).is_ok());
}

#[test]
fn proxy_validation_allows_disabled_empty_and_non_service_scopes() {
    assert!(validate_proxy_config(&ProxyConfig::default()).is_ok());

    let disabled_services = ProxyConfig { scope: ProxyScope::Services, ..Default::default() };
    assert!(validate_proxy_config(&disabled_services).is_ok());

    let environment = ProxyConfig {
        scope: ProxyScope::Environment,
        ..enabled_proxy("http://proxy.example:8080")
    };
    assert!(validate_proxy_config(&environment).is_ok());
}

// 错误路径还要确认原配置或存储保持不变，防止校验失败后留下部分更新。
#[test]
fn proxy_validation_rejects_enabled_config_without_nonblank_url() {
    let missing_url = ProxyConfig { enabled: true, ..Default::default() };
    let error = validate_proxy_config(&missing_url).unwrap_err().to_string();
    assert!(error.contains("no proxy URL"));

    let blank_urls = ProxyConfig {
        enabled: true,
        http_proxy: Some("  ".to_string()),
        https_proxy: Some("\t".to_string()),
        all_proxy: Some("\n".to_string()),
        ..Default::default()
    };
    let error = validate_proxy_config(&blank_urls).unwrap_err().to_string();
    assert!(error.contains("no proxy URL"));
}

#[test]
fn proxy_validation_rejects_malformed_scheme_and_hostless_urls() {
    let malformed = ProxyConfig { http_proxy: Some("not a URL".to_string()), ..Default::default() };
    let error = validate_proxy_config(&malformed).unwrap_err().to_string();
    assert!(error.contains("not a valid URL"));

    let bad_scheme =
        ProxyConfig { https_proxy: Some("ftp://proxy.example".to_string()), ..Default::default() };
    let error = validate_proxy_config(&bad_scheme).unwrap_err().to_string();
    assert!(error.contains("https_proxy"));
    assert!(error.contains("Allowed"));

    let hostless =
        ProxyConfig { all_proxy: Some("socks5:/proxy".to_string()), ..Default::default() };
    let error = validate_proxy_config(&hostless).unwrap_err().to_string();
    assert!(error.contains("all_proxy"));
    assert!(error.contains("host is required"));
}

#[test]
fn proxy_validation_rejects_unsupported_service_selector() {
    let config =
        ProxyConfig { services: vec![" provider.unknown ".to_string()], ..Default::default() };

    let error = validate_proxy_config(&config).unwrap_err().to_string();
    assert!(error.contains("provider.unknown"));
    assert!(error.contains("list_services"));
}

#[test]
fn proxy_validation_rejects_enabled_service_scope_without_selectors() {
    let config =
        ProxyConfig { scope: ProxyScope::Services, ..enabled_proxy("http://proxy.example:8080") };

    let error = validate_proxy_config(&config).unwrap_err().to_string();
    assert!(error.contains("non-empty proxy.services"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn no_proxy_value_distinguishes_empty_and_normalized_lists() {
    let empty = ProxyConfig { no_proxy: vec![" ,  ".to_string()], ..Default::default() };
    assert!(no_proxy_value(&empty).is_none());

    let populated = ProxyConfig {
        no_proxy: vec![" localhost, .example.com ".to_string()],
        ..Default::default()
    };
    assert!(no_proxy_value(&populated).is_some());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn proxy_builder_handles_valid_and_invalid_proxy_rules_without_network_io() {
    let valid = ProxyConfig {
        enabled: true,
        http_proxy: Some("http://proxy.example:8080".to_string()),
        https_proxy: Some("https://proxy.example:8443".to_string()),
        all_proxy: Some("socks5://proxy.example:1080".to_string()),
        no_proxy: vec!["localhost,.example.com".to_string()],
        ..Default::default()
    };
    assert!(
        apply_proxy_to_reqwest_builder(&valid, reqwest::Client::builder(), "provider.openai")
            .build()
            .is_ok()
    );

    let invalid = ProxyConfig {
        enabled: true,
        http_proxy: Some("not an http proxy URL".to_string()),
        https_proxy: Some("not an https proxy URL".to_string()),
        all_proxy: Some("not an all proxy URL".to_string()),
        ..Default::default()
    };
    assert!(
        apply_proxy_to_reqwest_builder(&invalid, reqwest::Client::builder(), "provider.openai")
            .build()
            .is_ok()
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn proxy_builder_accepts_each_proxy_kind_without_no_proxy_entries() {
    for config in [
        ProxyConfig {
            enabled: true,
            http_proxy: Some("http://proxy.example:8080".to_string()),
            ..Default::default()
        },
        ProxyConfig {
            enabled: true,
            https_proxy: Some("https://proxy.example:8443".to_string()),
            ..Default::default()
        },
        ProxyConfig {
            enabled: true,
            all_proxy: Some("socks5://proxy.example:1080".to_string()),
            ..Default::default()
        },
    ] {
        assert!(
            apply_proxy_to_reqwest_builder(&config, reqwest::Client::builder(), "provider.openai")
                .build()
                .is_ok()
        );
    }
}

#[test]
fn proxy_builder_skips_service_outside_configured_scope() {
    let config = ProxyConfig {
        enabled: true,
        http_proxy: Some("not a proxy URL".to_string()),
        scope: ProxyScope::Services,
        services: vec!["provider.openai".to_string()],
        ..Default::default()
    };

    assert!(
        apply_proxy_to_reqwest_builder(&config, reqwest::Client::builder(), "channel.slack")
            .build()
            .is_ok()
    );
}

#[test]
fn process_env_application_sets_clears_and_restores_every_proxy_key() {
    let _lock = lock_global_proxy_state();
    let _snapshot = ProxyEnvSnapshot::capture();
    clear_proxy_env();

    let config = ProxyConfig {
        http_proxy: Some(" http://proxy.example:8080 ".to_string()),
        https_proxy: Some("https://proxy.example:8443".to_string()),
        all_proxy: Some("socks5://proxy.example:1080".to_string()),
        no_proxy: vec![" localhost ".to_string(), "127.0.0.1".to_string()],
        ..Default::default()
    };
    apply_proxy_to_process_env(&config);

    for key in ["HTTP_PROXY", "http_proxy"] {
        assert_eq!(std::env::var(key).unwrap(), "http://proxy.example:8080");
    }
    for key in ["HTTPS_PROXY", "https_proxy"] {
        assert_eq!(std::env::var(key).unwrap(), "https://proxy.example:8443");
    }
    for key in ["ALL_PROXY", "all_proxy"] {
        assert_eq!(std::env::var(key).unwrap(), "socks5://proxy.example:1080");
    }
    for key in ["NO_PROXY", "no_proxy"] {
        assert_eq!(std::env::var(key).unwrap(), "127.0.0.1,localhost");
    }

    apply_proxy_to_process_env(&ProxyConfig::default());
    for key in PROXY_ENV_KEYS {
        assert!(std::env::var_os(key).is_none(), "{key} should be removed for empty config");
    }

    apply_proxy_to_process_env(&config);
    clear_proxy_env();
    for key in PROXY_ENV_KEYS {
        assert!(std::env::var_os(key).is_none(), "{key} should be cleared explicitly");
    }
}

#[test]
fn runtime_proxy_cache_key_normalizes_service_and_timeout_values() {
    assert_eq!(
        runtime_proxy_cache_key(" Provider.OpenAI ", None, None),
        "provider.openai|timeout=none|connect_timeout=none"
    );
    assert_eq!(
        runtime_proxy_cache_key("CHANNEL.SLACK", Some(30), Some(7)),
        "channel.slack|timeout=30|connect_timeout=7"
    );
}

#[test]
fn runtime_proxy_cache_helpers_store_read_and_clear_clients() {
    let _lock = lock_global_proxy_state();
    clear_runtime_proxy_client_cache();
    let cache_key = "provider.openai|timeout=none|connect_timeout=none";
    assert!(runtime_proxy_cached_client(cache_key).is_none());

    let client = reqwest::Client::builder().build().expect("client should build without I/O");
    set_runtime_proxy_cached_client(cache_key.to_string(), client);
    assert!(runtime_proxy_cached_client(cache_key).is_some());

    clear_runtime_proxy_client_cache();
    assert!(runtime_proxy_cached_client(cache_key).is_none());
}

#[test]
fn runtime_proxy_config_round_trip_preserves_fields_and_clears_cache() {
    let _lock = lock_global_proxy_state();
    let _snapshot = RuntimeProxySnapshot::capture();
    clear_runtime_proxy_client_cache();
    let client = reqwest::Client::builder().build().expect("client should build without I/O");
    set_runtime_proxy_cached_client("stale".to_string(), client);

    let config = ProxyConfig {
        enabled: true,
        http_proxy: Some("http://proxy.example:8080".to_string()),
        https_proxy: Some("https://proxy.example:8443".to_string()),
        all_proxy: Some("socks5://proxy.example:1080".to_string()),
        no_proxy: vec!["localhost".to_string()],
        scope: ProxyScope::Services,
        services: vec!["provider.openai".to_string()],
    };
    set_runtime_proxy_config(config.clone());

    let actual = runtime_proxy_config();
    assert_eq!(actual.enabled, config.enabled);
    assert_eq!(actual.http_proxy, config.http_proxy);
    assert_eq!(actual.https_proxy, config.https_proxy);
    assert_eq!(actual.all_proxy, config.all_proxy);
    assert_eq!(actual.no_proxy, config.no_proxy);
    assert_eq!(actual.scope, config.scope);
    assert_eq!(actual.services, config.services);
    assert!(runtime_proxy_cached_client("stale").is_none());
}

#[test]
fn runtime_client_builders_populate_plain_and_timeout_cache_entries() {
    let _lock = lock_global_proxy_state();
    let _snapshot = RuntimeProxySnapshot::capture();
    set_runtime_proxy_config(ProxyConfig::default());

    let _plain = build_runtime_proxy_client(" Provider.OpenAI ");
    let plain_key = runtime_proxy_cache_key("provider.openai", None, None);
    assert!(runtime_proxy_cached_client(&plain_key).is_some());
    let _cached_plain = build_runtime_proxy_client("PROVIDER.OPENAI");

    let _timeout = build_runtime_proxy_client_with_timeouts("provider.openai", 5, 2);
    let timeout_key = runtime_proxy_cache_key("provider.openai", Some(5), Some(2));
    assert!(runtime_proxy_cached_client(&timeout_key).is_some());
    let _cached_timeout = build_runtime_proxy_client_with_timeouts("PROVIDER.OPENAI", 5, 2);
}

#[test]
fn runtime_proxy_locks_recover_after_poisoning() {
    let _lock = lock_global_proxy_state();
    let _snapshot = RuntimeProxySnapshot::capture();
    let _poison_reset = RuntimeProxyPoisonReset;

    let cache_poisoned = catch_unwind(AssertUnwindSafe(|| {
        let _guard =
            runtime_proxy_client_cache().write().unwrap_or_else(|poisoned| poisoned.into_inner());
        panic!("poison proxy client cache for recovery coverage");
    }));
    assert!(cache_poisoned.is_err());

    let client = reqwest::Client::builder().build().expect("client should build without I/O");
    set_runtime_proxy_cached_client("recovered".to_string(), client);
    assert!(runtime_proxy_cached_client("recovered").is_some());
    clear_runtime_proxy_client_cache();
    assert!(runtime_proxy_cached_client("recovered").is_none());

    let state_poisoned = catch_unwind(AssertUnwindSafe(|| {
        let _guard = runtime_proxy_state().write().unwrap_or_else(|poisoned| poisoned.into_inner());
        panic!("poison proxy config state for recovery coverage");
    }));
    assert!(state_poisoned.is_err());

    let config = enabled_proxy("http://proxy.example:8080");
    set_runtime_proxy_config(config);
    assert!(runtime_proxy_config().enabled);
}

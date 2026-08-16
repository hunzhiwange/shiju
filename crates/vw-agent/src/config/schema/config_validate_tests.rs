use std::sync::{Mutex, MutexGuard};

use super::config_validate::{normalize_wire_api, validate_config};
use super::{
    Config, EmbeddingRouteConfig, GatewaySkey, ModelProviderConfig, ModelRouteConfig,
    ProviderApiMode, ProxyScope,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());
const CLOUD_ENV_KEYS: [&str; 3] = ["OLLAMA_API_KEY", "VIBEWINDOW_API_KEY", "API_KEY"];

struct EnvRestore {
    saved: Vec<(&'static str, Option<String>)>,
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        unsafe {
            for (key, value) in self.saved.drain(..) {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}

fn clear_cloud_env() -> (MutexGuard<'static, ()>, EnvRestore) {
    let lock = ENV_LOCK.lock().unwrap();
    let saved =
        CLOUD_ENV_KEYS.iter().map(|key| (*key, std::env::var(key).ok())).collect::<Vec<_>>();
    unsafe {
        for key in CLOUD_ENV_KEYS {
            std::env::remove_var(key);
        }
    }
    (lock, EnvRestore { saved })
}

fn assert_invalid(config: Config, expected: &str) {
    assert!(validate_config(&config).unwrap_err().to_string().contains(expected));
}

fn invalid_with(update: impl FnOnce(&mut Config), expected: &str) {
    let mut config = Config::default();
    update(&mut config);
    assert_invalid(config, expected);
}

fn gateway_skey(hash: &str) -> GatewaySkey {
    GatewaySkey {
        enabled: true,
        skey: None,
        skey_hash: hash.to_string(),
        masked_skey: String::new(),
        name: String::new(),
        expires_at: None,
    }
}

fn model_route() -> ModelRouteConfig {
    ModelRouteConfig {
        hint: "reasoning".to_string(),
        provider: "openrouter".to_string(),
        model: "model".to_string(),
        max_tokens: Some(1),
        api_key: None,
    }
}

fn embedding_route() -> EmbeddingRouteConfig {
    EmbeddingRouteConfig {
        hint: "semantic".to_string(),
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        dimensions: Some(1536),
        api_key: None,
    }
}

#[test]
fn normalize_wire_api_accepts_common_spellings() {
    assert_eq!(normalize_wire_api("responses"), Some("responses"));
    assert_eq!(normalize_wire_api(" Responses "), Some("responses"));
    assert_eq!(normalize_wire_api("chat-completions"), Some("chat_completions"));
    assert_eq!(normalize_wire_api("chat_completions"), Some("chat_completions"));
    assert_eq!(normalize_wire_api("chatcompletions"), Some("chat_completions"));
    assert_eq!(normalize_wire_api("chat"), Some("chat_completions"));
    assert_eq!(normalize_wire_api(" CHAT "), Some("chat_completions"));
    assert_eq!(normalize_wire_api(" "), None);
    assert_eq!(normalize_wire_api("unknown"), None);
}

#[test]
fn default_config_validates() {
    validate_config(&Config::default()).unwrap();
}

#[test]
fn validate_config_accepts_boundary_and_explicit_success_paths() {
    let hash = "a".repeat(64);
    let mut config = Config::default();
    config.gateway.skeys = vec![
        GatewaySkey {
            expires_at: Some("2026-12-31T23:59:59Z".to_string()),
            ..gateway_skey(&hash.to_ascii_uppercase())
        },
        GatewaySkey { skey: Some("raw-skey".to_string()), ..gateway_skey("") },
    ];
    config.reliability.fallback_providers = vec![" backup ".to_string()];
    config.reliability.fallback_api_keys.insert("backup".to_string(), "key".to_string());
    config.autonomy.shell_env_passthrough =
        vec!["VALID_NAME".to_string(), "_ALSO_VALID2".to_string()];
    config.autonomy.non_cli_excluded_tools = vec!["tool-name".to_string(), "tool_name".to_string()];
    config.security.otp.cache_valid_secs = config.security.otp.token_ttl_secs;
    config.security.otp.gated_actions = vec!["bash".to_string(), "file_write".to_string()];
    config.security.otp.gated_domains = vec!["example.com".to_string()];
    config.security.otp.gated_domain_categories = vec!["banking".to_string()];
    config.security.syscall_anomaly.baseline_syscalls =
        vec!["read".to_string(), "syscall#1".to_string()];
    config.security.semantic_guard_threshold = 0.0;
    config.plugins.enabled = vec!["plugin".to_string()];
    config.plugins.sources = vec!["https://plugins.example/index.json".to_string()];
    config.model_routes = vec![model_route()];
    config.default_provider = Some("custom:https://api.example/v1".to_string());
    config.provider_api = Some(ProviderApiMode::OpenAiResponses);
    config.embedding_routes = vec![embedding_route()];
    config.model_providers.insert(
        "custom".to_string(),
        ModelProviderConfig {
            name: Some("custom".to_string()),
            base_url: Some("https://api.example/v1".to_string()),
            wire_api: Some("chat_completions".to_string()),
            ..Default::default()
        },
    );
    config.model_providers.insert(
        "base-only".to_string(),
        ModelProviderConfig {
            base_url: Some("http://api.example/v1".to_string()),
            ..Default::default()
        },
    );
    config.model_providers.insert(
        "name-only".to_string(),
        ModelProviderConfig {
            name: Some("openai".to_string()),
            base_url: Some(" ".to_string()),
            wire_api: Some(" ".to_string()),
            ..Default::default()
        },
    );
    config.model_providers.insert(
        "wire-api-absent".to_string(),
        ModelProviderConfig { name: Some("openai".to_string()), ..Default::default() },
    );
    config.proxy.enabled = true;
    config.proxy.http_proxy = Some("http://127.0.0.1:8080".to_string());
    config.proxy.scope = ProxyScope::Services;
    config.proxy.services = vec!["tool.browser".to_string()];
    validate_config(&config).unwrap();

    config.security.semantic_guard_threshold = 1.0;
    validate_config(&config).unwrap();
}

#[test]
fn validate_config_rejects_blank_gateway_host_and_zero_limits() {
    invalid_with(|config| config.gateway.host = " ".to_string(), "gateway.host");
    invalid_with(|config| config.scheduler.max_concurrent = 0, "scheduler.max_concurrent");
    invalid_with(|config| config.autonomy.max_actions_per_hour = 0, "max_actions_per_hour");
}

#[test]
fn validate_config_rejects_invalid_gateway_skeys() {
    invalid_with(|config| config.gateway.skeys = vec![gateway_skey("abc")], "SHA-256 hex hash");

    let hash = "a".repeat(64);
    invalid_with(
        |config| {
            config.gateway.skeys =
                vec![gateway_skey(&hash.to_ascii_uppercase()), gateway_skey(&hash)];
        },
        "duplicate skey_hash",
    );

    invalid_with(
        |config| {
            let mut skey = gateway_skey(&hash);
            skey.expires_at = Some("tomorrow".to_string());
            config.gateway.skeys = vec![skey];
        },
        "expires_at must be RFC3339",
    );
}

#[test]
fn validate_config_accepts_blank_optional_gateway_skey_expiration() {
    let mut skey = gateway_skey(&"a".repeat(64));
    skey.expires_at = Some(" \t ".to_string());
    let mut config = Config::default();
    config.gateway.skeys = vec![skey];

    validate_config(&config).unwrap();
}

#[test]
fn validate_config_rejects_non_hex_gateway_skey_hash_at_required_length() {
    invalid_with(
        |config| config.gateway.skeys = vec![gateway_skey(&"g".repeat(64))],
        "SHA-256 hex hash",
    );
}

#[test]
fn validate_config_rejects_whitespace_only_raw_skey_with_invalid_hash() {
    invalid_with(
        |config| {
            let mut skey = gateway_skey("invalid");
            skey.skey = Some(" \t ".to_string());
            config.gateway.skeys = vec![skey];
        },
        "SHA-256 hex hash",
    );
}

#[test]
fn validate_config_rejects_invalid_fallback_entries() {
    invalid_with(
        |config| {
            config.reliability.fallback_api_keys.insert(" ".to_string(), "key".to_string());
        },
        "contains an empty key",
    );

    invalid_with(
        |config| {
            config.reliability.fallback_providers.push("backup".to_string());
            config.reliability.fallback_api_keys.insert("backup".to_string(), " ".to_string());
        },
        "fallback_api_keys.backup must not be empty",
    );

    invalid_with(
        |config| {
            config
                .reliability
                .fallback_api_keys
                .insert("missing-provider".to_string(), "key".to_string());
        },
        "no matching entry",
    );
}

#[test]
fn validate_config_accepts_trimmed_fallback_api_key_entry() {
    let mut config = Config::default();
    config.reliability.fallback_providers = vec![" backup ".to_string()];
    config.reliability.fallback_api_keys.insert(" backup ".to_string(), " key ".to_string());

    validate_config(&config).unwrap();
}

#[test]
fn validate_config_rejects_invalid_autonomy_lists() {
    invalid_with(
        |config| config.autonomy.shell_env_passthrough.push("1BAD".to_string()),
        "shell_env_passthrough",
    );
    invalid_with(
        |config| config.autonomy.non_cli_excluded_tools.push(" ".to_string()),
        "non_cli_excluded_tools",
    );
    invalid_with(
        |config| config.autonomy.non_cli_excluded_tools.push("bad tool".to_string()),
        "contains invalid characters",
    );
    invalid_with(
        |config| {
            config.autonomy.non_cli_excluded_tools = vec!["tool".to_string(), "tool".to_string()];
        },
        "duplicate entry",
    );
}

#[test]
fn validate_config_rejects_trimmed_duplicate_non_cli_tool_names() {
    invalid_with(
        |config| {
            config.autonomy.non_cli_excluded_tools =
                vec!["tool-name".to_string(), " tool-name ".to_string()];
        },
        "duplicate entry",
    );
}

#[test]
fn validate_config_rejects_invalid_otp_settings() {
    invalid_with(|config| config.security.otp.token_ttl_secs = 0, "token_ttl_secs");
    invalid_with(
        |config| config.security.otp.cache_valid_secs = 0,
        "cache_valid_secs must be greater than 0",
    );
    invalid_with(
        |config| {
            config.security.otp.token_ttl_secs = 60;
            config.security.otp.cache_valid_secs = 30;
        },
        "cache_valid_secs must be greater than or equal",
    );
    invalid_with(|config| config.security.otp.gated_actions.push(" ".to_string()), "gated_actions");
    invalid_with(
        |config| config.security.otp.gated_actions.push("bad action".to_string()),
        "contains invalid characters",
    );
    invalid_with(
        |config| config.security.otp.gated_domains.push("bad domain.com".to_string()),
        "Invalid security.otp.gated_domains",
    );
    invalid_with(
        |config| config.security.otp.gated_domain_categories.push("unknown".to_string()),
        "Invalid security.otp.gated_domains",
    );
}

#[test]
fn validate_config_rejects_invalid_security_settings() {
    invalid_with(|config| config.security.estop.state_file = " ".to_string(), "estop.state_file");
    invalid_with(
        |config| config.security.syscall_anomaly.max_denied_events_per_minute = 0,
        "max_denied_events_per_minute",
    );
    invalid_with(
        |config| config.security.syscall_anomaly.max_total_events_per_minute = 0,
        "max_total_events_per_minute",
    );
    invalid_with(
        |config| {
            config.security.syscall_anomaly.max_denied_events_per_minute = 10;
            config.security.syscall_anomaly.max_total_events_per_minute = 5;
        },
        "less than or equal",
    );
    invalid_with(
        |config| config.security.syscall_anomaly.max_alerts_per_minute = 0,
        "max_alerts_per_minute",
    );
    invalid_with(
        |config| config.security.syscall_anomaly.alert_cooldown_secs = 0,
        "alert_cooldown_secs",
    );
    invalid_with(|config| config.security.syscall_anomaly.log_path = " ".to_string(), "log_path");
    invalid_with(
        |config| config.security.syscall_anomaly.baseline_syscalls.push(" ".to_string()),
        "baseline_syscalls",
    );
    invalid_with(
        |config| config.security.syscall_anomaly.baseline_syscalls.push("open/read".to_string()),
        "contains invalid characters",
    );
    invalid_with(
        |config| config.security.semantic_guard_collection = " ".to_string(),
        "semantic_guard_collection",
    );
    invalid_with(
        |config| config.security.semantic_guard_threshold = -0.1,
        "semantic_guard_threshold",
    );
    invalid_with(
        |config| config.security.semantic_guard_threshold = 1.1,
        "semantic_guard_threshold",
    );
}

#[test]
fn validate_config_rejects_nan_semantic_guard_threshold() {
    invalid_with(
        |config| config.security.semantic_guard_threshold = f64::NAN,
        "semantic_guard_threshold",
    );
}

#[test]
fn validate_config_rejects_invalid_scheduler_and_routes() {
    invalid_with(|config| config.scheduler.max_tasks = 0, "scheduler.max_tasks");

    invalid_with(|config| config.plugins.enabled.push(" ".to_string()), "plugins.enabled[0]");
    invalid_with(|config| config.plugins.sources.push(" ".to_string()), "plugins.sources[0]");

    invalid_with(
        |config| {
            let mut route = model_route();
            route.hint = " ".to_string();
            config.model_routes = vec![route];
        },
        "model_routes[0].hint",
    );
    invalid_with(
        |config| {
            let mut route = model_route();
            route.provider = " ".to_string();
            config.model_routes = vec![route];
        },
        "model_routes[0].provider",
    );
    invalid_with(
        |config| {
            let mut route = model_route();
            route.model = " ".to_string();
            config.model_routes = vec![route];
        },
        "model_routes[0].model",
    );
    invalid_with(
        |config| {
            let mut route = model_route();
            route.max_tokens = Some(0);
            config.model_routes = vec![route];
        },
        "max_tokens",
    );
    invalid_with(
        |config| config.provider_api = Some(ProviderApiMode::OpenAiResponses),
        "provider_api is only valid",
    );
    invalid_with(
        |config| {
            let mut route = embedding_route();
            route.hint = " ".to_string();
            config.embedding_routes = vec![route];
        },
        "embedding_routes[0].hint",
    );
    invalid_with(
        |config| {
            let mut route = embedding_route();
            route.provider = " ".to_string();
            config.embedding_routes = vec![route];
        },
        "embedding_routes[0].provider",
    );
    invalid_with(
        |config| {
            let mut route = embedding_route();
            route.model = " ".to_string();
            config.embedding_routes = vec![route];
        },
        "embedding_routes[0].model",
    );
}

#[test]
fn validate_config_rejects_invalid_model_provider_profiles() {
    invalid_with(
        |config| {
            config.model_providers.insert(" ".to_string(), Default::default());
        },
        "empty profile name",
    );
    invalid_with(
        |config| {
            config.model_providers.insert("blank".to_string(), Default::default());
        },
        "must define at least one",
    );
    invalid_with(
        |config| {
            config.model_providers.insert(
                "bad-url".to_string(),
                ModelProviderConfig {
                    base_url: Some("not a url".to_string()),
                    ..Default::default()
                },
            );
        },
        "not a valid URL",
    );
    invalid_with(
        |config| {
            config.model_providers.insert(
                "bad-scheme".to_string(),
                ModelProviderConfig {
                    base_url: Some("ftp://example".to_string()),
                    ..Default::default()
                },
            );
        },
        "http/https",
    );
    invalid_with(
        |config| {
            config.model_providers.insert(
                "bad-wire".to_string(),
                ModelProviderConfig {
                    name: Some("openai".to_string()),
                    wire_api: Some("legacy".to_string()),
                    ..Default::default()
                },
            );
        },
        "wire_api",
    );
}

#[test]
fn validate_config_rejects_ollama_cloud_without_remote_endpoint_or_key() {
    let (_env_lock, _env_restore) = clear_cloud_env();
    let mut config = Config::default();
    config.default_provider = Some("ollama".to_string());
    config.default_model = Some("llama3:cloud".to_string());

    assert_invalid(config.clone(), "api_url is local");

    config.api_url = Some("http://localhost:11434".to_string());
    assert_invalid(config.clone(), "api_url is local");

    config.api_url = Some(" http://0.0.0.0:11434 ".to_string());
    assert_invalid(config.clone(), "api_url is local");

    config.api_url = Some("https://ollama.com".to_string());
    config.api_key = Some(" ".to_string());
    assert_invalid(config.clone(), "no API key");

    config.api_key = Some("key".to_string());
    validate_config(&config).unwrap();

    config.api_key = None;
    unsafe {
        std::env::set_var("OLLAMA_API_KEY", "env-key");
    }
    validate_config(&config).unwrap();
}

#[test]
fn validate_config_rejects_ollama_cloud_blank_and_loopback_endpoint_aliases() {
    let (_env_lock, _env_restore) = clear_cloud_env();
    let mut config = Config::default();
    config.default_provider = Some("ollama".to_string());
    config.default_model = Some("llama3:cloud".to_string());

    for api_url in [" ", "http://127.0.0.1:11434", "http://[::1]:11434"] {
        config.api_url = Some(api_url.to_string());
        assert!(
            validate_config(&config).unwrap_err().to_string().contains("api_url is local"),
            "expected local endpoint rejection for {api_url}"
        );
    }
}

#[test]
fn validate_config_accepts_each_ollama_cloud_environment_credential() {
    let (_env_lock, _env_restore) = clear_cloud_env();
    let mut config = Config::default();
    config.default_provider = Some(" OLLAMA ".to_string());
    config.default_model = Some(" llama3:cloud ".to_string());
    config.api_url = Some("https://ollama.com".to_string());

    for key in CLOUD_ENV_KEYS {
        unsafe {
            std::env::set_var(key, "env-key");
        }
        validate_config(&config).unwrap();
        unsafe {
            std::env::remove_var(key);
        }
    }

    unsafe {
        std::env::set_var("OLLAMA_API_KEY", " ");
        std::env::set_var("VIBEWINDOW_API_KEY", "fallback-key");
    }
    validate_config(&config).unwrap();
}

#[test]
fn validate_config_accepts_local_ollama_model_without_cloud_credential() {
    let mut config = Config::default();
    config.default_provider = Some("ollama".to_string());
    config.default_model = Some("llama3".to_string());
    config.api_url = None;
    config.api_key = None;

    validate_config(&config).unwrap();
}

#[test]
fn validate_config_rejects_proxy_errors() {
    invalid_with(
        |config| config.proxy.enabled = true,
        "Proxy is enabled but no proxy URL is configured",
    );
}

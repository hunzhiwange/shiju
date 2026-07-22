//! 代理配置工具测试模块
//!
//! 本模块包含 ProxyConfigTool 的单元测试，验证代理配置的各种操作场景：
//! - 列出已知的代理服务键
//! - 设置服务级别的代理配置
//! - 代理设置的读取和清除操作
//! - 配置项的往返测试

use super::super::*;
use crate::Config;
use crate::app::agent::config::{
    ProxyConfig, ProxyScope, runtime_proxy_config, schema::ConfigExt, set_runtime_proxy_config,
};
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use crate::app::agent::util::MaybeSet;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::sync::{Arc, Mutex, MutexGuard};
use tempfile::TempDir;

static PROXY_ENV_LOCK: Mutex<()> = Mutex::new(());

const PROXY_ENV_KEYS: [&str; 8] = [
    "HTTP_PROXY",
    "http_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "ALL_PROXY",
    "all_proxy",
    "NO_PROXY",
    "no_proxy",
];

struct ProxyEnvGuard {
    _global_lock: MutexGuard<'static, ()>,
    _runtime_proxy_lock: MutexGuard<'static, ()>,
    _lock: MutexGuard<'static, ()>,
    previous: Vec<(&'static str, Option<OsString>)>,
    runtime_proxy: ProxyConfig,
}

impl ProxyEnvGuard {
    fn clear() -> Self {
        let global_lock =
            crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner());
        let runtime_proxy_lock = crate::test_support::global_runtime_proxy_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let lock = PROXY_ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous = PROXY_ENV_KEYS.iter().map(|&key| (key, std::env::var_os(key))).collect();
        let runtime_proxy = runtime_proxy_config();
        for key in PROXY_ENV_KEYS {
            unsafe { std::env::remove_var(key) };
        }
        Self {
            _global_lock: global_lock,
            _runtime_proxy_lock: runtime_proxy_lock,
            _lock: lock,
            previous,
            runtime_proxy,
        }
    }

    fn set(&self, key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }
}

impl Drop for ProxyEnvGuard {
    fn drop(&mut self) {
        set_runtime_proxy_config(self.runtime_proxy.clone());
        for (key, value) in &self.previous {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}

/// 创建测试用的安全策略
///
/// 返回一个配置为监督模式（Supervised）的安全策略实例，
/// 使用系统临时目录作为工作空间。
fn test_security() -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        workspace_dir: std::env::temp_dir(),
        ..SecurityPolicy::default()
    })
}

/// 创建测试用的配置实例
///
/// # 参数
/// - `tmp`: 临时目录引用，用于创建配置文件和工作空间
///
/// # 返回值
/// 返回一个已保存的配置实例的 Arc 智能指针
async fn test_config(tmp: &TempDir) -> Arc<Config> {
    std::fs::create_dir_all(tmp.path().join("workspace")).unwrap();
    let config: Config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.save().await.unwrap();
    Arc::new(config)
}

/// 测试 list_services 操作是否能返回已知的服务键
///
/// 验证当执行 "list_services" 动作时，返回的结果中包含预期的服务标识符。
#[tokio::test]
async fn list_services_action_returns_known_keys() {
    // 创建临时测试环境
    let tmp = TempDir::new().unwrap();
    let tool = ProxyConfigTool::new(test_config(&tmp).await, test_security());

    // 执行列出服务操作
    let result = tool.execute(json!({"action": "list_services"})).await.unwrap();

    // 验证操作成功且包含预期的服务键
    assert!(result.success);
    assert!(result.output.contains("provider.openai"));
    assert!(result.output.contains("tool.http_request"));
}

/// 测试使用 services 作用域时必须提供 services 条目
///
/// 验证当 scope 设置为 "services" 时，如果 services 数组为空，
/// 操作应该失败并返回相应的错误信息。
#[tokio::test]
async fn set_scope_services_requires_services_entries() {
    let tmp = TempDir::new().unwrap();
    let tool = ProxyConfigTool::new(test_config(&tmp).await, test_security());

    // 尝试设置服务级别代理，但 services 数组为空
    let result = tool
        .execute(json!({
            "action": "set",
            "enabled": true,
            "scope": "services",
            "http_proxy": "http://127.0.0.1:7890",
            "services": []
        }))
        .await
        .unwrap();

    // 验证操作失败且错误信息提到 scope='services'
    assert!(!result.success);
    assert!(result.error.unwrap_or_default().contains("proxy.scope='services'"));
}

/// 测试代理设置的往返（set 和 get）操作
///
/// 验证：
/// 1. 设置服务级别代理配置能够成功
/// 2. 通过 get 操作能够读取到之前设置的配置
/// 3. 读取的配置中包含预期的服务标识符
#[tokio::test]
async fn set_and_get_round_trip_proxy_scope() {
    let _env = ProxyEnvGuard::clear();
    let tmp = TempDir::new().unwrap();
    let tool = ProxyConfigTool::new(test_config(&tmp).await, test_security());

    // 设置服务级别的代理配置
    let set_result = tool
        .execute(json!({
            "action": "set",
            "scope": "services",
            "http_proxy": "http://127.0.0.1:7890",
            "services": ["provider.openai", "tool.http_request"]
        }))
        .await
        .unwrap();
    assert!(set_result.success, "{:?}", set_result.error);

    // 读取配置并验证
    let get_result = tool.execute(json!({"action": "get"})).await.unwrap();
    assert!(get_result.success);
    assert!(get_result.output.contains("provider.openai"));
    assert!(get_result.output.contains("services"));
}

/// 测试设置 null 值可以清除已存在的代理配置
///
/// 验证：
/// 1. 能够成功设置 http_proxy
/// 2. 通过设置 http_proxy 为 null 可以清除现有值
/// 3. 清除后读取配置，确认代理值确实为 null
#[tokio::test]
async fn set_null_proxy_url_clears_existing_value() {
    let _env = ProxyEnvGuard::clear();
    let tmp = TempDir::new().unwrap();
    let tool = ProxyConfigTool::new(test_config(&tmp).await, test_security());

    // 首先设置一个代理值
    let set_result = tool
        .execute(json!({
            "action": "set",
            "http_proxy": "http://127.0.0.1:7890"
        }))
        .await
        .unwrap();
    assert!(set_result.success, "{:?}", set_result.error);

    // 清除代理值（设置为 null）
    let clear_result = tool
        .execute(json!({
            "action": "set",
            "http_proxy": null
        }))
        .await
        .unwrap();
    assert!(clear_result.success, "{:?}", clear_result.error);

    // 验证配置已被清除
    let get_result = tool.execute(json!({"action": "get"})).await.unwrap();
    assert!(get_result.success);

    // 解析 JSON 输出并验证代理值为 null
    let parsed: Value = serde_json::from_str(&get_result.output).unwrap();
    assert!(parsed["proxy"]["http_proxy"].is_null());
    assert!(parsed["runtime_proxy"]["http_proxy"].is_null());
}

#[tokio::test]
async fn set_environment_scope_applies_all_proxy_fields_to_process_environment() {
    let env = ProxyEnvGuard::clear();
    let tmp = TempDir::new().unwrap();
    let tool = ProxyConfigTool::new(test_config(&tmp).await, test_security());

    let result = tool
        .execute(json!({
            "action": "set",
            "enabled": true,
            "scope": "environment",
            "http_proxy": " http://127.0.0.1:7890 ",
            "https_proxy": "https://127.0.0.1:7891",
            "all_proxy": "socks5://127.0.0.1:7892",
            "no_proxy": " localhost,127.0.0.1,localhost "
        }))
        .await
        .unwrap();
    let output: Value = serde_json::from_str(&result.output).unwrap();

    assert!(result.success, "{:?}", result.error);
    assert_eq!(output["proxy"]["scope"], "environment");
    assert_eq!(output["proxy"]["http_proxy"], "http://127.0.0.1:7890");
    assert_eq!(output["proxy"]["https_proxy"], "https://127.0.0.1:7891");
    assert_eq!(output["proxy"]["all_proxy"], "socks5://127.0.0.1:7892");
    assert_eq!(output["proxy"]["no_proxy"], json!(["127.0.0.1", "localhost"]));
    assert_eq!(std::env::var("HTTP_PROXY").as_deref(), Ok("http://127.0.0.1:7890"));
    assert_eq!(std::env::var("HTTPS_PROXY").as_deref(), Ok("https://127.0.0.1:7891"));
    assert_eq!(std::env::var("ALL_PROXY").as_deref(), Ok("socks5://127.0.0.1:7892"));
    assert_eq!(std::env::var("NO_PROXY").as_deref(), Ok("127.0.0.1,localhost"));
    assert_eq!(std::env::var("http_proxy").as_deref(), Ok("http://127.0.0.1:7890"));
    assert_eq!(std::env::var("https_proxy").as_deref(), Ok("https://127.0.0.1:7891"));
    assert_eq!(std::env::var("all_proxy").as_deref(), Ok("socks5://127.0.0.1:7892"));
    assert_eq!(std::env::var("no_proxy").as_deref(), Ok("127.0.0.1,localhost"));
    drop(env);
}

#[tokio::test]
async fn set_switching_from_environment_scope_clears_previous_process_environment() {
    let _env = ProxyEnvGuard::clear();
    let tmp = TempDir::new().unwrap();
    let tool = ProxyConfigTool::new(test_config(&tmp).await, test_security());

    let environment_result = tool
        .execute(json!({
            "action": "set",
            "enabled": true,
            "scope": "environment",
            "http_proxy": "http://127.0.0.1:7890"
        }))
        .await
        .unwrap();
    assert!(environment_result.success, "{:?}", environment_result.error);
    assert_eq!(std::env::var("HTTP_PROXY").as_deref(), Ok("http://127.0.0.1:7890"));

    let vibewindow_result = tool
        .execute(json!({
            "action": "set",
            "scope": "vibewindow"
        }))
        .await
        .unwrap();
    let output: Value = serde_json::from_str(&vibewindow_result.output).unwrap();

    assert!(vibewindow_result.success, "{:?}", vibewindow_result.error);
    assert_eq!(output["proxy"]["scope"], "vibewindow");
    assert!(output["environment"]["HTTP_PROXY"].is_null());
    for key in PROXY_ENV_KEYS {
        assert!(std::env::var_os(key).is_none(), "{key} should be cleared after scope switch");
    }
}

#[tokio::test]
async fn set_null_values_clear_https_and_all_proxy_fields() {
    let _env = ProxyEnvGuard::clear();
    let tmp = TempDir::new().unwrap();
    let tool = ProxyConfigTool::new(test_config(&tmp).await, test_security());

    let set_result = tool
        .execute(json!({
            "action": "set",
            "https_proxy": "https://127.0.0.1:7891",
            "all_proxy": "socks5://127.0.0.1:7892"
        }))
        .await
        .unwrap();
    assert!(set_result.success, "{:?}", set_result.error);

    let clear_result = tool
        .execute(json!({
            "action": "set",
            "https_proxy": null,
            "all_proxy": null
        }))
        .await
        .unwrap();
    let output: Value = serde_json::from_str(&clear_result.output).unwrap();

    assert!(clear_result.success, "{:?}", clear_result.error);
    assert!(!output["proxy"]["enabled"].as_bool().unwrap());
    assert!(output["proxy"]["https_proxy"].is_null());
    assert!(output["proxy"]["all_proxy"].is_null());
}

#[test]
fn parse_scope_accepts_aliases_case_and_surrounding_whitespace() {
    for (raw, expected) in [
        (" environment ", ProxyScope::Environment),
        ("ENV", ProxyScope::Environment),
        ("vibewindow", ProxyScope::Vibewindow),
        (" Internal ", ProxyScope::Vibewindow),
        ("CORE", ProxyScope::Vibewindow),
        ("services", ProxyScope::Services),
        (" SERVICE ", ProxyScope::Services),
    ] {
        assert_eq!(ProxyConfigTool::parse_scope(raw), Some(expected));
    }

    assert_eq!(ProxyConfigTool::parse_scope("unsupported"), None);
}

#[test]
fn parse_string_list_normalizes_strings_and_arrays() {
    let from_string =
        ProxyConfigTool::parse_string_list(&json!(" alpha, beta ,, gamma "), "services").unwrap();
    assert_eq!(from_string, vec!["alpha", "beta", "gamma"]);

    let from_array =
        ProxyConfigTool::parse_string_list(&json!([" alpha ", "", " beta", "   "]), "services")
            .unwrap();
    assert_eq!(from_array, vec!["alpha", "beta"]);
}

#[test]
fn parse_string_list_rejects_non_string_inputs() {
    let mixed_array_error =
        ProxyConfigTool::parse_string_list(&json!(["valid", 7]), "services").unwrap_err();
    assert_eq!(mixed_array_error.to_string(), "'services' array must only contain strings");

    let scalar_error =
        ProxyConfigTool::parse_string_list(&json!({"selector": "provider.openai"}), "services")
            .unwrap_err();
    assert_eq!(scalar_error.to_string(), "'services' must be a string or string[]");
}

#[test]
fn parse_optional_string_update_distinguishes_unset_null_and_value() {
    assert_eq!(
        ProxyConfigTool::parse_optional_string_update(&json!({}), "http_proxy").unwrap(),
        MaybeSet::Unset
    );
    assert_eq!(
        ProxyConfigTool::parse_optional_string_update(&json!({"http_proxy": null}), "http_proxy")
            .unwrap(),
        MaybeSet::Null
    );
    assert_eq!(
        ProxyConfigTool::parse_optional_string_update(&json!({"http_proxy": "   "}), "http_proxy")
            .unwrap(),
        MaybeSet::Null
    );
    assert_eq!(
        ProxyConfigTool::parse_optional_string_update(
            &json!({"http_proxy": " http://127.0.0.1:7890 "}),
            "http_proxy"
        )
        .unwrap(),
        MaybeSet::Set("http://127.0.0.1:7890".to_string())
    );
}

#[test]
fn parse_optional_string_update_rejects_non_string_value() {
    let error =
        ProxyConfigTool::parse_optional_string_update(&json!({"http_proxy": false}), "http_proxy")
            .unwrap_err();

    assert_eq!(error.to_string(), "'http_proxy' must be a string or null");
}

#[test]
fn proxy_json_returns_normalized_lists_and_serialized_scope() {
    let proxy = ProxyConfig {
        enabled: true,
        http_proxy: Some("http://127.0.0.1:7890".to_string()),
        https_proxy: None,
        all_proxy: Some("socks5://127.0.0.1:7891".to_string()),
        no_proxy: vec![" localhost ".to_string(), "example.com,localhost".to_string()],
        scope: ProxyScope::Services,
        services: vec![
            " Tool.HTTP_Request ".to_string(),
            "provider.openai".to_string(),
            "tool.http_request".to_string(),
        ],
    };

    assert_eq!(
        ProxyConfigTool::proxy_json(&proxy),
        json!({
            "enabled": true,
            "scope": "services",
            "http_proxy": "http://127.0.0.1:7890",
            "https_proxy": null,
            "all_proxy": "socks5://127.0.0.1:7891",
            "no_proxy": ["example.com", "localhost"],
            "services": ["provider.openai", "tool.http_request"],
        })
    );
}

#[test]
fn env_snapshot_always_contains_supported_proxy_keys() {
    let snapshot = ProxyConfigTool::env_snapshot();
    let object = snapshot.as_object().unwrap();

    assert_eq!(object.len(), 4);
    for key in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "NO_PROXY"] {
        assert!(object.contains_key(key));
    }
}

#[test]
fn tool_metadata_and_schema_describe_supported_actions() {
    let tool = ProxyConfigTool::new(Arc::new(Config::default()), test_security());

    assert_eq!(tool.name(), "proxy_config");
    assert!(tool.description().contains("environment | vibewindow | services"));

    let schema = tool.parameters_schema();
    assert_eq!(schema["type"], "object");
    assert_eq!(
        schema["properties"]["action"]["enum"],
        json!(["get", "set", "disable", "list_services", "apply_env", "clear_env"])
    );
    assert_eq!(schema["properties"]["action"]["default"], "get");
    assert_eq!(schema["properties"]["http_proxy"]["type"], json!(["string", "null"]));
}

#[tokio::test]
async fn action_routing_is_case_insensitive_for_read_operations() {
    let tool = ProxyConfigTool::new(Arc::new(Config::default()), test_security());

    let result = tool.execute(json!({"action": "LIST_SERVICES"})).await.unwrap();

    assert!(result.success);
    assert!(result.error.is_none());
    assert!(result.output.contains("provider.openai"));
}

#[tokio::test]
async fn unknown_action_returns_a_descriptive_error() {
    let tool = ProxyConfigTool::new(Arc::new(Config::default()), test_security());

    let error = tool
        .execute(json!({"action": "rotate"}))
        .await
        .err()
        .expect("unknown actions must return an error");

    assert_eq!(
        error.to_string(),
        "Unknown action 'rotate'. Valid: get, set, disable, list_services, apply_env, clear_env"
    );
}

#[tokio::test]
async fn read_only_policy_rejects_write_action_before_dispatch() {
    let security =
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() });
    let tool = ProxyConfigTool::new(Arc::new(Config::default()), security);

    let result = tool.execute(json!({"action": "apply_env"})).await.unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(result.error.as_deref(), Some("Action blocked: autonomy is read-only"));
}

#[tokio::test]
async fn exhausted_action_budget_rejects_write_action_before_dispatch() {
    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        max_actions_per_hour: 0,
        ..SecurityPolicy::default()
    });
    let tool = ProxyConfigTool::new(Arc::new(Config::default()), security);

    let result = tool.execute(json!({"action": "apply_env"})).await.unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert_eq!(result.error.as_deref(), Some("Action blocked: rate limit exceeded"));
}

#[tokio::test]
async fn missing_action_defaults_to_get_and_returns_all_snapshots() {
    let tmp = TempDir::new().unwrap();
    let tool = ProxyConfigTool::new(test_config(&tmp).await, test_security());

    let result = tool.execute(json!({})).await.unwrap();
    let output: Value = serde_json::from_str(&result.output).unwrap();

    assert!(result.success);
    assert!(output.get("proxy").is_some());
    assert!(output.get("runtime_proxy").is_some());
    assert!(output.get("environment").is_some());
}

#[tokio::test]
async fn set_rejects_non_boolean_enabled_before_mutating_configuration() {
    let tmp = TempDir::new().unwrap();
    let tool = ProxyConfigTool::new(test_config(&tmp).await, test_security());

    let result = tool.execute(json!({"action": "set", "enabled": "yes"})).await.unwrap();

    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("'enabled' must be a boolean"));
}

#[tokio::test]
async fn set_rejects_non_string_and_unknown_scopes() {
    for (scope, expected) in [
        (json!(7), "'scope' must be a string"),
        (json!("external"), "Invalid scope 'external'. Use environment|vibewindow|services"),
    ] {
        let tmp = TempDir::new().unwrap();
        let result = ProxyConfigTool::new(test_config(&tmp).await, test_security())
            .execute(json!({"action": "set", "scope": scope}))
            .await
            .unwrap();

        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some(expected));
    }
}

#[tokio::test]
async fn apply_env_rejects_disabled_proxy_without_changing_process_environment() {
    let tmp = TempDir::new().unwrap();
    let tool = ProxyConfigTool::new(test_config(&tmp).await, test_security());

    let result = tool.execute(json!({"action": "apply_env"})).await.unwrap();

    assert!(!result.success);
    assert_eq!(
        result.error.as_deref(),
        Some("Proxy is disabled. Use action 'set' with enabled=true first")
    );
}

#[tokio::test]
async fn apply_env_rejects_enabled_non_environment_scope() {
    let _env = ProxyEnvGuard::clear();
    let tmp = TempDir::new().unwrap();
    let tool = ProxyConfigTool::new(test_config(&tmp).await, test_security());

    let set_result = tool
        .execute(json!({
            "action": "set",
            "enabled": true,
            "scope": "vibewindow",
            "http_proxy": "http://127.0.0.1:7890"
        }))
        .await
        .unwrap();
    assert!(set_result.success, "{:?}", set_result.error);

    let result = tool.execute(json!({"action": "apply_env"})).await.unwrap();

    assert!(!result.success);
    assert_eq!(
        result.error.as_deref(),
        Some("apply_env only works when proxy.scope is 'environment' (current: Vibewindow)")
    );
    for key in PROXY_ENV_KEYS {
        assert!(std::env::var_os(key).is_none(), "{key} should not be set by rejected apply_env");
    }
}

#[tokio::test]
async fn apply_env_action_applies_saved_environment_proxy_to_process_environment() {
    let _env = ProxyEnvGuard::clear();
    let tmp = TempDir::new().unwrap();
    let tool = ProxyConfigTool::new(test_config(&tmp).await, test_security());

    let set_result = tool
        .execute(json!({
            "action": "set",
            "enabled": true,
            "scope": "environment",
            "http_proxy": "http://127.0.0.1:7890",
            "no_proxy": [" localhost ", "127.0.0.1"]
        }))
        .await
        .unwrap();
    assert!(set_result.success, "{:?}", set_result.error);
    for key in PROXY_ENV_KEYS {
        unsafe { std::env::remove_var(key) };
    }

    let result = tool.execute(json!({"action": "apply_env"})).await.unwrap();
    let output: Value = serde_json::from_str(&result.output).unwrap();

    assert!(result.success, "{:?}", result.error);
    assert_eq!(output["message"], "Proxy environment variables applied");
    assert_eq!(output["environment"]["HTTP_PROXY"], "http://127.0.0.1:7890");
    assert_eq!(output["environment"]["NO_PROXY"], "127.0.0.1,localhost");
    assert_eq!(std::env::var("HTTP_PROXY").as_deref(), Ok("http://127.0.0.1:7890"));
    assert_eq!(std::env::var("NO_PROXY").as_deref(), Ok("127.0.0.1,localhost"));
}

#[tokio::test]
async fn clear_env_action_removes_uppercase_and_lowercase_proxy_variables() {
    let env = ProxyEnvGuard::clear();
    env.set("HTTP_PROXY", "http://127.0.0.1:7890");
    env.set("http_proxy", "http://127.0.0.1:7890");
    env.set("HTTPS_PROXY", "https://127.0.0.1:7891");
    env.set("https_proxy", "https://127.0.0.1:7891");
    env.set("ALL_PROXY", "socks5://127.0.0.1:7892");
    env.set("all_proxy", "socks5://127.0.0.1:7892");
    env.set("NO_PROXY", "localhost");
    env.set("no_proxy", "localhost");
    let tmp = TempDir::new().unwrap();
    let tool = ProxyConfigTool::new(test_config(&tmp).await, test_security());

    let result = tool.execute(json!({"action": "clear_env"})).await.unwrap();
    let output: Value = serde_json::from_str(&result.output).unwrap();

    assert!(result.success, "{:?}", result.error);
    assert_eq!(output["message"], "Proxy environment variables cleared");
    assert!(output["environment"]["HTTP_PROXY"].is_null());
    assert!(output["environment"]["HTTPS_PROXY"].is_null());
    assert!(output["environment"]["ALL_PROXY"].is_null());
    assert!(output["environment"]["NO_PROXY"].is_null());
    for key in PROXY_ENV_KEYS {
        assert!(std::env::var_os(key).is_none(), "{key} should be cleared");
    }
    drop(env);
}

#[tokio::test]
async fn disable_clears_environment_proxy_by_default() {
    let _env = ProxyEnvGuard::clear();
    let tmp = TempDir::new().unwrap();
    let tool = ProxyConfigTool::new(test_config(&tmp).await, test_security());

    let set_result = tool
        .execute(json!({
            "action": "set",
            "enabled": true,
            "scope": "environment",
            "http_proxy": "http://127.0.0.1:7890"
        }))
        .await
        .unwrap();
    assert!(set_result.success, "{:?}", set_result.error);

    let result = tool.execute(json!({"action": "disable"})).await.unwrap();
    let output: Value = serde_json::from_str(&result.output).unwrap();

    assert!(result.success, "{:?}", result.error);
    assert_eq!(output["message"], "Proxy disabled");
    assert_eq!(output["proxy"]["enabled"], false);
    assert!(output["environment"]["HTTP_PROXY"].is_null());
    for key in PROXY_ENV_KEYS {
        assert!(std::env::var_os(key).is_none(), "{key} should be cleared by disable");
    }
}

#[tokio::test]
async fn disable_respects_clear_env_false_override() {
    let _env = ProxyEnvGuard::clear();
    let tmp = TempDir::new().unwrap();
    let tool = ProxyConfigTool::new(test_config(&tmp).await, test_security());

    let set_result = tool
        .execute(json!({
            "action": "set",
            "enabled": true,
            "scope": "environment",
            "http_proxy": "http://127.0.0.1:7890"
        }))
        .await
        .unwrap();
    assert!(set_result.success, "{:?}", set_result.error);

    let result = tool.execute(json!({"action": "disable", "clear_env": false})).await.unwrap();
    let output: Value = serde_json::from_str(&result.output).unwrap();

    assert!(result.success, "{:?}", result.error);
    assert_eq!(output["proxy"]["enabled"], false);
    assert_eq!(output["environment"]["HTTP_PROXY"], "http://127.0.0.1:7890");
    assert_eq!(std::env::var("HTTP_PROXY").as_deref(), Ok("http://127.0.0.1:7890"));
}

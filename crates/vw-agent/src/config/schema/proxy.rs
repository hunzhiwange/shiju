//! 代理配置的校验、运行时应用和 HTTP client 构建支持。
//!
//! 本模块负责把持久化的 `ProxyConfig` 转换为进程环境变量和 `reqwest` client
//! 配置。代理可能影响外部网络访问范围，因此这里集中校验 URL scheme、服务选择器
//! 和运行时缓存失效，避免无效配置被静默扩散到所有网络调用。

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

pub use vw_config_types::proxy::*;

static RUNTIME_PROXY_CONFIG: OnceLock<RwLock<ProxyConfig>> = OnceLock::new();
static RUNTIME_PROXY_CLIENT_CACHE: OnceLock<RwLock<HashMap<String, reqwest::Client>>> =
    OnceLock::new();

/// 校验代理配置是否可安全应用。
///
/// 参数 `config` 是待检查的代理配置。返回 `Ok(())` 表示 URL、服务选择器和作用域
/// 组合均有效；返回错误时会指出具体字段，调用方应阻止该配置进入运行时状态。
pub fn validate_proxy_config(config: &ProxyConfig) -> Result<()> {
    for (field, value) in [
        ("http_proxy", config.http_proxy.as_deref()),
        ("https_proxy", config.https_proxy.as_deref()),
        ("all_proxy", config.all_proxy.as_deref()),
    ] {
        if let Some(url) = normalize_proxy_url_option(value) {
            validate_proxy_url(field, &url)?;
        }
    }

    for selector in config.normalized_services() {
        if !is_supported_proxy_service_selector(&selector) {
            anyhow::bail!(
                "Unsupported proxy service selector '{selector}'. Use tool `proxy_config` action `list_services` for valid values"
            );
        }
    }

    if config.enabled && !config.has_any_proxy_url() {
        anyhow::bail!(
            "Proxy is enabled but no proxy URL is configured. Set at least one of http_proxy, https_proxy, or all_proxy"
        );
    }

    if config.enabled
        && config.scope == ProxyScope::Services
        && config.normalized_services().is_empty()
    {
        anyhow::bail!(
            "proxy.scope='services' requires a non-empty proxy.services list when proxy is enabled"
        );
    }

    Ok(())
}

/// 将代理配置应用到指定的 `reqwest::ClientBuilder`。
///
/// 参数 `config` 提供代理 URL、no_proxy 和作用域；`builder` 是待增强的 client
/// builder；`service_key` 用于判断该服务是否在代理作用范围内。函数返回更新后的
/// builder。无效代理 URL 在这里会被记录并忽略，因为保存配置前应已通过
/// [`validate_proxy_config`] 做强校验。
pub fn apply_proxy_to_reqwest_builder(
    config: &ProxyConfig,
    mut builder: reqwest::ClientBuilder,
    service_key: &str,
) -> reqwest::ClientBuilder {
    if !config.should_apply_to_service(service_key) {
        return builder;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let no_proxy = no_proxy_value(config);

        // reqwest 按添加顺序叠加代理规则；保持 all/http/https 的显式顺序，
        // 让更具体的协议代理仍可覆盖通用代理。
        if let Some(url) = normalize_proxy_url_option(config.all_proxy.as_deref()) {
            match reqwest::Proxy::all(&url) {
                Ok(proxy) => {
                    builder = builder.proxy(apply_no_proxy(proxy, no_proxy.clone()));
                }
                Err(error) => {
                    tracing::warn!(
                        proxy_url = %url,
                        service_key,
                        "Ignoring invalid all_proxy URL: {error}"
                    );
                }
            }
        }

        if let Some(url) = normalize_proxy_url_option(config.http_proxy.as_deref()) {
            match reqwest::Proxy::http(&url) {
                Ok(proxy) => {
                    builder = builder.proxy(apply_no_proxy(proxy, no_proxy.clone()));
                }
                Err(error) => {
                    tracing::warn!(
                        proxy_url = %url,
                        service_key,
                        "Ignoring invalid http_proxy URL: {error}"
                    );
                }
            }
        }

        if let Some(url) = normalize_proxy_url_option(config.https_proxy.as_deref()) {
            match reqwest::Proxy::https(&url) {
                Ok(proxy) => {
                    builder = builder.proxy(apply_no_proxy(proxy, no_proxy));
                }
                Err(error) => {
                    tracing::warn!(
                        proxy_url = %url,
                        service_key,
                        "Ignoring invalid https_proxy URL: {error}"
                    );
                }
            }
        }
    }

    builder
}

/// 将代理配置写入当前进程环境变量。
///
/// 参数 `config` 提供 HTTP/HTTPS/ALL/NO_PROXY 值。函数没有返回值；空值会清除对应
/// 环境变量，避免旧代理设置残留到后续子进程。
pub fn apply_proxy_to_process_env(config: &ProxyConfig) {
    set_proxy_env_pair("HTTP_PROXY", config.http_proxy.as_deref());
    set_proxy_env_pair("HTTPS_PROXY", config.https_proxy.as_deref());
    set_proxy_env_pair("ALL_PROXY", config.all_proxy.as_deref());

    let no_proxy_joined = {
        let list = config.normalized_no_proxy();
        (!list.is_empty()).then(|| list.join(","))
    };
    set_proxy_env_pair("NO_PROXY", no_proxy_joined.as_deref());
}

/// 清除当前进程中的代理环境变量。
///
/// 同时清理大小写两种常见变量名，确保后续命令或子进程不会继承已禁用的代理。
pub fn clear_proxy_env() {
    clear_proxy_env_pair("HTTP_PROXY");
    clear_proxy_env_pair("HTTPS_PROXY");
    clear_proxy_env_pair("ALL_PROXY");
    clear_proxy_env_pair("NO_PROXY");
}

#[cfg(not(target_arch = "wasm32"))]
fn no_proxy_value(config: &ProxyConfig) -> Option<reqwest::NoProxy> {
    let joined = {
        let list = config.normalized_no_proxy();
        (!list.is_empty()).then(|| list.join(","))
    };
    joined.as_deref().and_then(reqwest::NoProxy::from_string)
}

#[cfg(not(target_arch = "wasm32"))]
fn apply_no_proxy(proxy: reqwest::Proxy, no_proxy: Option<reqwest::NoProxy>) -> reqwest::Proxy {
    proxy.no_proxy(no_proxy)
}

fn validate_proxy_url(field: &str, url: &str) -> Result<()> {
    let parsed = reqwest::Url::parse(url)
        .with_context(|| format!("Invalid {field} URL: '{url}' is not a valid URL"))?;

    // 只允许 reqwest 明确支持且代理语义清晰的 scheme，防止配置把网络请求导向
    // 未预期的传输协议。
    match parsed.scheme() {
        "http" | "https" | "socks5" | "socks5h" => {}
        scheme => {
            anyhow::bail!(
                "Invalid {field} URL scheme '{scheme}'. Allowed: http, https, socks5, socks5h"
            );
        }
    }

    if parsed.host_str().is_none() {
        anyhow::bail!("Invalid {field} URL: host is required");
    }

    Ok(())
}

fn set_proxy_env_pair(key: &str, value: Option<&str>) {
    let lowercase_key = key.to_ascii_lowercase();
    if let Some(value) = value.and_then(|candidate| normalize_proxy_url_option(Some(candidate))) {
        // Rust 2024 将环境变量修改标为 unsafe，因为多线程进程中环境变量是全局状态。
        // 这里仍集中写入大小写两份键，便于不同工具链读取同一份代理设置。
        unsafe {
            std::env::set_var(key, &value);
            std::env::set_var(&lowercase_key, value);
        }
    } else {
        // 清理分支同样要覆盖大小写键，避免之前的代理配置通过环境变量泄漏到子进程。
        unsafe {
            std::env::remove_var(key);
            std::env::remove_var(&lowercase_key);
        }
    }
}

fn clear_proxy_env_pair(key: &str) {
    unsafe {
        std::env::remove_var(key);
        std::env::remove_var(&key.to_ascii_lowercase());
    }
}

fn runtime_proxy_state() -> &'static RwLock<ProxyConfig> {
    RUNTIME_PROXY_CONFIG.get_or_init(|| RwLock::new(ProxyConfig::default()))
}

fn runtime_proxy_client_cache() -> &'static RwLock<HashMap<String, reqwest::Client>> {
    RUNTIME_PROXY_CLIENT_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn clear_runtime_proxy_client_cache() {
    match runtime_proxy_client_cache().write() {
        Ok(mut guard) => {
            guard.clear();
        }
        Err(poisoned) => {
            poisoned.into_inner().clear();
        }
    }
}

fn runtime_proxy_cache_key(
    service_key: &str,
    timeout_secs: Option<u64>,
    connect_timeout_secs: Option<u64>,
) -> String {
    format!(
        "{}|timeout={}|connect_timeout={}",
        service_key.trim().to_ascii_lowercase(),
        timeout_secs.map(|value| value.to_string()).unwrap_or_else(|| "none".to_string()),
        connect_timeout_secs.map(|value| value.to_string()).unwrap_or_else(|| "none".to_string())
    )
}

fn runtime_proxy_cached_client(cache_key: &str) -> Option<reqwest::Client> {
    match runtime_proxy_client_cache().read() {
        Ok(guard) => guard.get(cache_key).cloned(),
        Err(poisoned) => poisoned.into_inner().get(cache_key).cloned(),
    }
}

fn set_runtime_proxy_cached_client(cache_key: String, client: reqwest::Client) {
    match runtime_proxy_client_cache().write() {
        Ok(mut guard) => {
            guard.insert(cache_key, client);
        }
        Err(poisoned) => {
            poisoned.into_inner().insert(cache_key, client);
        }
    }
}

/// 更新运行时代理配置并清空 client 缓存。
///
/// 参数 `config` 会替换全局运行时配置。函数不返回错误；锁中毒时仍取回内部值继续
/// 替换，保证用户显式更新代理后不会继续复用旧配置。
pub fn set_runtime_proxy_config(config: ProxyConfig) {
    match runtime_proxy_state().write() {
        Ok(mut guard) => {
            *guard = config;
        }
        Err(poisoned) => {
            *poisoned.into_inner() = config;
        }
    }

    clear_runtime_proxy_client_cache();
}

/// 读取当前运行时代理配置快照。
///
/// 返回克隆后的 `ProxyConfig`，调用方可以安全持有该快照而不占用全局读锁。
pub fn runtime_proxy_config() -> ProxyConfig {
    match runtime_proxy_state().read() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

/// 将当前运行时代理配置应用到 `reqwest::ClientBuilder`。
///
/// 参数 `builder` 是待配置的 builder，`service_key` 用于服务作用域匹配。返回值为
/// 应用代理后的 builder。
pub fn apply_runtime_proxy_to_builder(
    builder: reqwest::ClientBuilder,
    service_key: &str,
) -> reqwest::ClientBuilder {
    apply_proxy_to_reqwest_builder(&runtime_proxy_config(), builder, service_key)
}

/// 构建或复用指定服务的运行时代理 client。
///
/// 参数 `service_key` 用作代理作用域匹配和缓存 key 的一部分。返回可直接使用的
/// `reqwest::Client`；构建失败时记录告警并回退为默认 client。
pub fn build_runtime_proxy_client(service_key: &str) -> reqwest::Client {
    let cache_key = runtime_proxy_cache_key(service_key, None, None);
    if let Some(client) = runtime_proxy_cached_client(&cache_key) {
        return client;
    }

    let builder = apply_runtime_proxy_to_builder(reqwest::Client::builder(), service_key);
    let client = builder.build().unwrap_or_else(|error| {
        tracing::warn!(service_key, "Failed to build proxied client: {error}");
        reqwest::Client::new()
    });
    set_runtime_proxy_cached_client(cache_key, client.clone());
    client
}

/// 构建或复用带超时设置的运行时代理 client。
///
/// 参数 `service_key` 用于代理作用域匹配；`timeout_secs` 与
/// `connect_timeout_secs` 分别控制总超时和连接超时。返回构建好的 client；如果
/// builder 构建失败，会记录告警并回退为默认 client。
pub fn build_runtime_proxy_client_with_timeouts(
    service_key: &str,
    timeout_secs: u64,
    connect_timeout_secs: u64,
) -> reqwest::Client {
    let cache_key =
        runtime_proxy_cache_key(service_key, Some(timeout_secs), Some(connect_timeout_secs));
    if let Some(client) = runtime_proxy_cached_client(&cache_key) {
        return client;
    }

    let builder = reqwest::Client::builder();
    #[cfg(not(target_arch = "wasm32"))]
    let builder = builder
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .connect_timeout(std::time::Duration::from_secs(connect_timeout_secs));
    let builder = apply_runtime_proxy_to_builder(builder, service_key);
    let client = builder.build().unwrap_or_else(|error| {
        tracing::warn!(service_key, "Failed to build proxied timeout client: {error}");
        reqwest::Client::new()
    });
    set_runtime_proxy_cached_client(cache_key, client.clone());
    client
}

/// 解析用户输入的代理作用域。
///
/// 返回 `Some(ProxyScope)` 表示识别成功；未知文本返回 `None`，由调用方决定如何向
/// 用户报告错误。
pub fn parse_proxy_scope(raw: &str) -> Option<ProxyScope> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "environment" | "env" => Some(ProxyScope::Environment),
        "vibewindow" | "internal" | "core" => Some(ProxyScope::Vibewindow),
        "services" | "service" => Some(ProxyScope::Services),
        _ => None,
    }
}

/// 解析用户输入的代理启用状态。
///
/// 支持常见布尔字符串；无法识别时返回 `None`，避免把拼写错误静默解释成 false。
pub fn parse_proxy_enabled(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
#[path = "proxy_tests.rs"]
mod proxy_tests;

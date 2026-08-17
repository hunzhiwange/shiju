//! 定义代理配置类型。
//! 模块只描述代理输入形态，不负责网络连接策略。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// SUPPORTED_PROXY_SERVICE_KEYS 是该模块对外使用的常量值。
pub const SUPPORTED_PROXY_SERVICE_KEYS: &[&str] = &[
    "provider.anthropic",
    "provider.compatible",
    "provider.copilot",
    "provider.gemini",
    "provider.glm",
    "provider.ollama",
    "provider.openai",
    "provider.openrouter",
    "channel.dingtalk",
    "channel.discord",
    "channel.feishu",
    "channel.lark",
    "channel.matrix",
    "channel.mattermost",
    "channel.nextcloud_talk",
    "channel.qq",
    "channel.signal",
    "channel.slack",
    "channel.telegram",
    "channel.wati",
    "channel.whatsapp",
    "tool.browser",
    "tool.http_request",
    "memory.embeddings",
    "tunnel.custom",
];

/// SUPPORTED_PROXY_SERVICE_SELECTORS 是该模块对外使用的常量值。
pub const SUPPORTED_PROXY_SERVICE_SELECTORS: &[&str] =
    &["provider.*", "channel.*", "tool.*", "memory.*", "tunnel.*"];

/// ProxyScope 描述该模块对外暴露的离散状态。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProxyScope {
    Environment,
    #[default]
    Vibewindow,
    Services,
}

/// ProxyConfig 表示该模块对外暴露的结构化状态。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProxyConfig {
    /// enabled 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub enabled: bool,
    /// http_proxy 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub http_proxy: Option<String>,
    /// https_proxy 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub https_proxy: Option<String>,
    /// all_proxy 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub all_proxy: Option<String>,
    /// no_proxy 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub no_proxy: Vec<String>,
    /// scope 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub scope: ProxyScope,
    /// services 字段保存该结构体对外暴露的同名状态。
    #[serde(default)]
    pub services: Vec<String>,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            http_proxy: None,
            https_proxy: None,
            all_proxy: None,
            no_proxy: Vec::new(),
            scope: ProxyScope::Vibewindow,
            services: Vec::new(),
        }
    }
}

impl ProxyConfig {
    pub fn supported_service_keys() -> &'static [&'static str] {
        SUPPORTED_PROXY_SERVICE_KEYS
    }

    pub fn supported_service_selectors() -> &'static [&'static str] {
        SUPPORTED_PROXY_SERVICE_SELECTORS
    }

    pub fn has_any_proxy_url(&self) -> bool {
        normalize_proxy_url_option(self.http_proxy.as_deref()).is_some()
            || normalize_proxy_url_option(self.https_proxy.as_deref()).is_some()
            || normalize_proxy_url_option(self.all_proxy.as_deref()).is_some()
    }

    pub fn normalized_services(&self) -> Vec<String> {
        normalize_service_list(self.services.clone())
    }

    pub fn normalized_no_proxy(&self) -> Vec<String> {
        normalize_no_proxy_list(self.no_proxy.clone())
    }

    pub fn should_apply_to_service(&self, service_key: &str) -> bool {
        if !self.enabled {
            return false;
        }

        match self.scope {
            ProxyScope::Environment => false,
            ProxyScope::Vibewindow => true,
            ProxyScope::Services => {
                let service_key = service_key.trim().to_ascii_lowercase();
                if service_key.is_empty() {
                    return false;
                }
                self.normalized_services()
                    .iter()
                    .any(|selector| service_selector_matches(selector, &service_key))
            }
        }
    }
}

/// 执行 normalize_proxy_url_option 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn normalize_proxy_url_option(raw: Option<&str>) -> Option<String> {
    let value = raw?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

/// 执行 normalize_no_proxy_list 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn normalize_no_proxy_list(values: Vec<String>) -> Vec<String> {
    normalize_comma_values(values)
}

/// 执行 normalize_service_list 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn normalize_service_list(values: Vec<String>) -> Vec<String> {
    let mut normalized = normalize_comma_values(values)
        .into_iter()
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized.dedup();
    normalized
}

/// 执行 normalize_comma_values 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn normalize_comma_values(values: Vec<String>) -> Vec<String> {
    let mut output = Vec::new();
    for value in values {
        for part in value.split(',') {
            let normalized = part.trim();
            if normalized.is_empty() {
                continue;
            }
            output.push(normalized.to_string());
        }
    }
    output.sort_unstable();
    output.dedup();
    output
}

/// 执行 is_supported_proxy_service_selector 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn is_supported_proxy_service_selector(selector: &str) -> bool {
    if SUPPORTED_PROXY_SERVICE_KEYS.iter().any(|known| known.eq_ignore_ascii_case(selector)) {
        return true;
    }

    SUPPORTED_PROXY_SERVICE_SELECTORS.iter().any(|known| known.eq_ignore_ascii_case(selector))
}

/// 执行 service_selector_matches 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn service_selector_matches(selector: &str, service_key: &str) -> bool {
    if selector == service_key {
        return true;
    }

    if let Some(prefix) = selector.strip_suffix(".*") {
        return service_key.starts_with(prefix)
            && service_key.strip_prefix(prefix).is_some_and(|suffix| suffix.starts_with('.'));
    }

    false
}
#[cfg(test)]
#[path = "proxy_tests.rs"]
mod proxy_tests;

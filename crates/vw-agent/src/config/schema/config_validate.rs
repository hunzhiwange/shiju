//! agent 配置的运行时校验规则。
//!
//! 校验逻辑集中在这里，负责拒绝空字段、越界数值、无效 provider/profile、错误代理配置和不安全安全策略。
//! 这些检查在配置加载和显式校验时执行，确保调用方不会在无效配置上继续启动运行时。

use anyhow::{Context, Result};

use vw_config_types::config::Config;

use crate::app::agent::config::schema::channels::is_valid_env_var_name;
use crate::app::agent::config::schema::proxy::validate_proxy_config;
use crate::app::agent::security::DomainMatcher;

/// 将用户配置的 wire API 名称归一化为内部稳定标识。
///
/// 支持 Responses API 与 Chat Completions API 的常见拼写。未知值返回 `None`，由调用方决定是否报错。
pub(crate) fn normalize_wire_api(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "responses" => Some("responses"),
        "chat_completions" | "chat-completions" | "chat" | "chatcompletions" => {
            Some("chat_completions")
        }
        _ => None,
    }
}

fn is_local_ollama_endpoint(api_url: Option<&str>) -> bool {
    let Some(raw) = api_url.map(str::trim).filter(|value| !value.is_empty()) else {
        return true;
    };

    reqwest::Url::parse(raw)
        .ok()
        .and_then(|url| {
            url.host_str()
                .map(|host| host.trim_start_matches('[').trim_end_matches(']').to_ascii_lowercase())
        })
        .is_some_and(|host| matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1" | "0.0.0.0"))
}

fn has_ollama_cloud_credential(config_api_key: Option<&str>) -> bool {
    let config_key_present = config_api_key.map(str::trim).is_some_and(|value| !value.is_empty());
    if config_key_present {
        return true;
    }

    ["OLLAMA_API_KEY", "VIBEWINDOW_API_KEY", "API_KEY"]
        .iter()
        .any(|name| std::env::var(name).ok().is_some_and(|value| !value.trim().is_empty()))
}

/// 校验配置对象是否满足运行时约束。
///
/// 返回 `Ok(())` 表示配置可以继续用于启动 agent。任何必填字段为空、数值为零或越界、fallback key 与
/// fallback provider 不匹配、provider profile 不完整、Ollama cloud 配置缺少远端地址或凭据、安全域规则无效、
/// 代理配置无效等情况都会返回错误。
///
/// 该函数只读取配置和必要的环境变量，不修改配置对象。安全相关字段默认采用显式拒绝策略，避免无效配置被
/// 解释成更宽松的运行时行为。
pub fn validate_config(config: &Config) -> Result<()> {
    if config.gateway.host.trim().is_empty() {
        anyhow::bail!("gateway.host must not be empty");
    }
    let mut seen_skey_hashes = std::collections::HashSet::new();
    for (index, skey) in config.gateway.skeys.iter().enumerate() {
        let hash = skey.skey_hash.trim();
        let raw_skey_present =
            skey.skey.as_deref().map(str::trim).is_some_and(|value| !value.is_empty());
        if !raw_skey_present
            && (hash.len() != 64 || !hash.chars().all(|value| value.is_ascii_hexdigit()))
        {
            anyhow::bail!("gateway.skeys[{index}].skey_hash must be a SHA-256 hex hash");
        }
        if !hash.is_empty() && !seen_skey_hashes.insert(hash.to_ascii_lowercase()) {
            anyhow::bail!("gateway.skeys contains duplicate skey_hash: {hash}");
        }
        if let Some(expires_at) =
            skey.expires_at.as_deref().map(str::trim).filter(|value| !value.is_empty())
        {
            chrono::DateTime::parse_from_rfc3339(expires_at)
                .with_context(|| format!("gateway.skeys[{index}].expires_at must be RFC3339"))?;
        }
    }

    if config.autonomy.max_actions_per_hour == 0 {
        anyhow::bail!("autonomy.max_actions_per_hour must be greater than 0");
    }
    for (i, env_name) in config.autonomy.shell_env_passthrough.iter().enumerate() {
        if !is_valid_env_var_name(env_name) {
            anyhow::bail!(
                "autonomy.shell_env_passthrough[{i}] is invalid ({env_name}); expected [A-Za-z_][A-Za-z0-9_]*"
            );
        }
    }
    let mut seen_non_cli_excluded = std::collections::HashSet::new();
    for (i, tool_name) in config.autonomy.non_cli_excluded_tools.iter().enumerate() {
        let normalized = tool_name.trim();
        if normalized.is_empty() {
            anyhow::bail!("autonomy.non_cli_excluded_tools[{i}] must not be empty");
        }
        if !normalized.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            anyhow::bail!(
                "autonomy.non_cli_excluded_tools[{i}] contains invalid characters: {normalized}"
            );
        }
        if !seen_non_cli_excluded.insert(normalized.to_string()) {
            anyhow::bail!("autonomy.non_cli_excluded_tools contains duplicate entry: {normalized}");
        }
    }

    if config.security.otp.token_ttl_secs == 0 {
        anyhow::bail!("security.otp.token_ttl_secs must be greater than 0");
    }
    if config.security.otp.cache_valid_secs == 0 {
        anyhow::bail!("security.otp.cache_valid_secs must be greater than 0");
    }
    if config.security.otp.cache_valid_secs < config.security.otp.token_ttl_secs {
        anyhow::bail!(
            "security.otp.cache_valid_secs must be greater than or equal to security.otp.token_ttl_secs"
        );
    }
    for (i, action) in config.security.otp.gated_actions.iter().enumerate() {
        let normalized = action.trim();
        if normalized.is_empty() {
            anyhow::bail!("security.otp.gated_actions[{i}] must not be empty");
        }
        if !normalized.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            anyhow::bail!(
                "security.otp.gated_actions[{i}] contains invalid characters: {normalized}"
            );
        }
    }
    DomainMatcher::new(
        &config.security.otp.gated_domains,
        &config.security.otp.gated_domain_categories,
    )
    .with_context(
        || "Invalid security.otp.gated_domains or security.otp.gated_domain_categories",
    )?;
    // 安全策略的域名匹配器必须能完整构建；解析失败时不能降级为“无域名限制”。
    if config.security.estop.state_file.trim().is_empty() {
        anyhow::bail!("security.estop.state_file must not be empty");
    }
    if config.security.syscall_anomaly.max_denied_events_per_minute == 0 {
        anyhow::bail!(
            "security.syscall_anomaly.max_denied_events_per_minute must be greater than 0"
        );
    }
    if config.security.syscall_anomaly.max_total_events_per_minute == 0 {
        anyhow::bail!(
            "security.syscall_anomaly.max_total_events_per_minute must be greater than 0"
        );
    }
    if config.security.syscall_anomaly.max_denied_events_per_minute
        > config.security.syscall_anomaly.max_total_events_per_minute
    {
        anyhow::bail!(
            "security.syscall_anomaly.max_denied_events_per_minute must be less than or equal to security.syscall_anomaly.max_total_events_per_minute"
        );
    }
    if config.security.syscall_anomaly.max_alerts_per_minute == 0 {
        anyhow::bail!("security.syscall_anomaly.max_alerts_per_minute must be greater than 0");
    }
    if config.security.syscall_anomaly.alert_cooldown_secs == 0 {
        anyhow::bail!("security.syscall_anomaly.alert_cooldown_secs must be greater than 0");
    }
    if config.security.syscall_anomaly.log_path.trim().is_empty() {
        anyhow::bail!("security.syscall_anomaly.log_path must not be empty");
    }
    for (i, syscall_name) in config.security.syscall_anomaly.baseline_syscalls.iter().enumerate() {
        let normalized = syscall_name.trim();
        if normalized.is_empty() {
            anyhow::bail!("security.syscall_anomaly.baseline_syscalls[{i}] must not be empty");
        }
        if !normalized.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '#') {
            anyhow::bail!(
                "security.syscall_anomaly.baseline_syscalls[{i}] contains invalid characters: {normalized}"
            );
        }
    }
    if config.security.semantic_guard_collection.trim().is_empty() {
        anyhow::bail!("security.semantic_guard_collection must not be empty");
    }
    if !(0.0..=1.0).contains(&config.security.semantic_guard_threshold) {
        anyhow::bail!("security.semantic_guard_threshold must be between 0.0 and 1.0");
    }

    if config.scheduler.max_concurrent == 0 {
        anyhow::bail!("scheduler.max_concurrent must be greater than 0");
    }
    if config.scheduler.max_tasks == 0 {
        anyhow::bail!("scheduler.max_tasks must be greater than 0");
    }

    for (i, plugin_name) in config.plugins.enabled.iter().enumerate() {
        if plugin_name.trim().is_empty() {
            anyhow::bail!("plugins.enabled[{i}] must not be empty");
        }
    }
    for (i, source) in config.plugins.sources.iter().enumerate() {
        if source.trim().is_empty() {
            anyhow::bail!("plugins.sources[{i}] must not be empty");
        }
    }

    for (i, route) in config.model_routes.iter().enumerate() {
        if route.hint.trim().is_empty() {
            anyhow::bail!("model_routes[{i}].hint must not be empty");
        }
        if route.provider.trim().is_empty() {
            anyhow::bail!("model_routes[{i}].provider must not be empty");
        }
        if route.model.trim().is_empty() {
            anyhow::bail!("model_routes[{i}].model must not be empty");
        }
        if route.max_tokens == Some(0) {
            anyhow::bail!("model_routes[{i}].max_tokens must be greater than 0");
        }
    }

    if config.provider_api.is_some()
        && !config
            .default_provider
            .as_deref()
            .is_some_and(|provider| provider.starts_with("custom:"))
    {
        anyhow::bail!(
            "provider_api is only valid when default_provider uses the custom:<url> format"
        );
    }

    for (i, route) in config.embedding_routes.iter().enumerate() {
        if route.hint.trim().is_empty() {
            anyhow::bail!("embedding_routes[{i}].hint must not be empty");
        }
        if route.provider.trim().is_empty() {
            anyhow::bail!("embedding_routes[{i}].provider must not be empty");
        }
        if route.model.trim().is_empty() {
            anyhow::bail!("embedding_routes[{i}].model must not be empty");
        }
    }

    for (profile_key, profile) in &config.model_providers {
        let profile_name = profile_key.trim();
        if profile_name.is_empty() {
            anyhow::bail!("model_providers contains an empty profile name");
        }

        let has_name =
            profile.name.as_deref().map(str::trim).is_some_and(|value| !value.is_empty());
        let has_base_url =
            profile.base_url.as_deref().map(str::trim).is_some_and(|value| !value.is_empty());

        if !has_name && !has_base_url {
            // profile 至少要能指向已知 provider 名称或自定义地址，否则后续解析没有确定目标。
            anyhow::bail!(
                "model_providers.{profile_name} must define at least one of `name` or `base_url`"
            );
        }

        if let Some(base_url) = profile.base_url.as_deref().map(str::trim) {
            if !base_url.is_empty() {
                let parsed = reqwest::Url::parse(base_url).with_context(|| {
                    format!("model_providers.{profile_name}.base_url is not a valid URL")
                })?;
                if !matches!(parsed.scheme(), "http" | "https") {
                    anyhow::bail!("model_providers.{profile_name}.base_url must use http/https");
                }
            }
        }

        if let Some(wire_api) = profile.wire_api.as_deref().map(str::trim) {
            if !wire_api.is_empty() && normalize_wire_api(wire_api).is_none() {
                anyhow::bail!(
                    "model_providers.{profile_name}.wire_api must be one of: responses, chat_completions"
                );
            }
        }
    }

    if config
        .default_provider
        .as_deref()
        .is_some_and(|provider| provider.trim().eq_ignore_ascii_case("ollama"))
        && config.default_model.as_deref().is_some_and(|model| model.trim().ends_with(":cloud"))
    {
        // Ollama cloud 模型不能落到本地默认端点，否则用户以为在调用云模型，实际却请求了本地服务。
        if is_local_ollama_endpoint(config.api_url.as_deref()) {
            anyhow::bail!(
                "default_model uses ':cloud' with provider 'ollama', but api_url is local or unset. Set api_url to a remote Ollama endpoint (for example https://ollama.com)."
            );
        }

        if !has_ollama_cloud_credential(config.api_key.as_deref()) {
            anyhow::bail!(
                "default_model uses ':cloud' with provider 'ollama', but no API key is configured. Set api_key or OLLAMA_API_KEY."
            );
        }
    }

    validate_proxy_config(&config.proxy)?;

    Ok(())
}

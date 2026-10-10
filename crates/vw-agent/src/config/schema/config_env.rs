//! 配置环境变量覆盖模块
//!
//! 本模块为 `Config` 提供 `apply_env_overrides` 方法的实现，用于从环境变量中读取配置覆盖值。
//!
//! ## 主要功能
//!
//! - 支持多种环境变量命名风格（VIBEWINDOW_* 前缀优先，回退到通用名称）
//! - 覆盖 API 密钥、Provider、模型、工作区目录等核心配置
//! - 配置网关端口、主机绑定和公开绑定权限
//! - 管理技能系统（目录提供方/提示模式）
//! - 控制推理能力、视觉支持和网络搜索功能
//! - 设置存储后端（Provider/连接 URL/超时）
//! - 完整的代理配置（启用标志/URL/作用域/服务列表/验证）
//!
//! ## 环境变量优先级
//!
//! 1. 带有 `VIBEWINDOW_` 前缀的环境变量（推荐）
//! 2. 通用名称的环境变量（兼容性支持）
//!
//! ## 安全考虑
//!
//! - 敏感值（如 API 密钥）从环境变量读取，不写入日志
//! - 无效的环境变量值会被忽略并发出警告
//! - 代理配置在应用前会进行验证

use vw_config_types::config::Config;

use crate::app::agent::config::schema::config::{
    apply_named_model_provider_profile, apply_workspace_override,
};
use crate::app::agent::config::schema::proxy::{
    ProxyScope, apply_proxy_to_process_env, normalize_no_proxy_list, normalize_proxy_url_option,
    normalize_service_list, parse_proxy_enabled, parse_proxy_scope, set_runtime_proxy_config,
    validate_proxy_config,
};
use crate::app::agent::config::schema::skills::{
    parse_skills_directory_provider, parse_skills_prompt_injection_mode,
};

pub fn apply_env_overrides(config: &mut Config) {
    // ==================== API 密钥配置 ====================
    // 优先级：VIBEWINDOW_API_KEY > API_KEY
    // 从环境变量读取 API 密钥，如果设置了且非空则覆盖配置中的 api_key 字段
    if let Ok(key) = std::env::var("VIBEWINDOW_API_KEY").or_else(|_| std::env::var("API_KEY")) {
        if !key.is_empty() {
            config.api_key = Some(key);
        }
    }

    // ==================== Provider 配置 ====================
    // Provider 覆盖的优先级规则：
    // 1) VIBEWINDOW_PROVIDER 始终优先（当设置时）
    // 2) VIBEWINDOW_MODEL_PROVIDER/MODEL_PROVIDER（Codex app-server 兼容风格）
    // 3) 遗留的 PROVIDER 仅在配置仍使用默认 provider 时生效
    if let Ok(provider) = std::env::var("VIBEWINDOW_PROVIDER") {
        if !provider.is_empty() {
            config.default_provider = Some(provider);
        }
    } else if let Ok(provider) =
        std::env::var("VIBEWINDOW_MODEL_PROVIDER").or_else(|_| std::env::var("MODEL_PROVIDER"))
    {
        if !provider.is_empty() {
            config.default_provider = Some(provider);
        }
    } else if let Ok(provider) = std::env::var("PROVIDER") {
        // 检查是否应该应用遗留的 PROVIDER 环境变量
        // 仅当配置的 default_provider 为空或为 "shiju"（默认值）时才应用
        let should_apply_legacy_provider = config
            .default_provider
            .as_deref()
            .map_or(true, |configured| configured.trim().eq_ignore_ascii_case("shiju"));
        if should_apply_legacy_provider && !provider.is_empty() {
            config.default_provider = Some(provider);
        }
    }

    // ==================== 模型配置 ====================
    // 优先级：VIBEWINDOW_MODEL > MODEL
    if let Ok(model) = std::env::var("VIBEWINDOW_MODEL").or_else(|_| std::env::var("MODEL")) {
        if !model.is_empty() {
            config.default_model = Some(model);
        }
    }

    // 应用命名的 provider 配置文件重映射（Codex app-server 兼容性）
    // 某些特定的 model_provider 值会触发配置文件切换
    apply_named_model_provider_profile(config);

    // ==================== 工作区配置 ====================
    // VIBEWINDOW_WORKSPACE: 覆盖工作区目录路径
    if let Ok(workspace) = std::env::var("VIBEWINDOW_WORKSPACE") {
        apply_workspace_override(config, &workspace);
    }

    // ==================== 技能系统配置 ====================
    // 技能提示模式覆盖：VIBEWINDOW_SKILLS_PROMPT_MODE
    // 有效值：full（完整模式）| compact（紧凑模式）
    if let Ok(mode) = std::env::var("VIBEWINDOW_SKILLS_PROMPT_MODE") {
        if !mode.trim().is_empty() {
            if let Some(parsed) = parse_skills_prompt_injection_mode(&mode) {
                config.skills.prompt_injection_mode = parsed;
            } else {
                tracing::warn!(
                    "忽略无效的 VIBEWINDOW_SKILLS_PROMPT_MODE 值（有效值：full|compact）"
                );
            }
        }
    }

    // 技能目录提供方覆盖：VIBEWINDOW_SKILLS_DIRECTORY_PROVIDER
    // 有效值：vibewindow|codex|claude|cursor
    if let Ok(provider) = std::env::var("VIBEWINDOW_SKILLS_DIRECTORY_PROVIDER") {
        if !provider.trim().is_empty() {
            if let Some(parsed) = parse_skills_directory_provider(&provider) {
                config.skills.directory_provider = parsed;
            } else {
                tracing::warn!(
                    "忽略无效的 VIBEWINDOW_SKILLS_DIRECTORY_PROVIDER 值（有效值：vibewindow|codex|claude|cursor）"
                );
            }
        }
    }

    // ==================== 网关配置 ====================
    // 网关端口：VIBEWINDOW_GATEWAY_PORT 或 PORT
    if let Ok(port_str) =
        std::env::var("VIBEWINDOW_GATEWAY_PORT").or_else(|_| std::env::var("PORT"))
    {
        if let Ok(port) = port_str.parse::<u16>() {
            config.gateway.port = port;
        }
    }

    // 网关主机：VIBEWINDOW_GATEWAY_HOST 或 HOST
    if let Ok(host) = std::env::var("VIBEWINDOW_GATEWAY_HOST").or_else(|_| std::env::var("HOST")) {
        if !host.is_empty() {
            config.gateway.host = host;
        }
    }

    // 允许公开绑定：VIBEWINDOW_ALLOW_PUBLIC_BIND
    // 当设置为 "1" 或 "true"（不区分大小写）时允许绑定到公开网络接口
    if let Ok(val) = std::env::var("VIBEWINDOW_ALLOW_PUBLIC_BIND") {
        config.gateway.allow_public_bind = val == "1" || val.eq_ignore_ascii_case("true");
    }

    // ==================== 温度配置 ====================
    // VIBEWINDOW_TEMPERATURE: 覆盖默认温度参数
    // 有效范围：0.0 到 2.0
    if let Ok(temp_str) = std::env::var("VIBEWINDOW_TEMPERATURE") {
        if let Ok(temp) = temp_str.parse::<f64>() {
            if (0.0..=2.0).contains(&temp) {
                config.default_temperature = temp;
            }
        }
    }

    // ==================== 推理功能配置 ====================
    // 推理启用覆盖：VIBEWINDOW_REASONING_ENABLED 或 REASONING_ENABLED
    // 支持多种布尔值表示：1|0|true|false|yes|no|on|off
    if let Ok(flag) = std::env::var("VIBEWINDOW_REASONING_ENABLED")
        .or_else(|_| std::env::var("REASONING_ENABLED"))
    {
        let normalized = flag.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "1" | "true" | "yes" | "on" => config.runtime.reasoning_enabled = Some(true),
            "0" | "false" | "no" | "off" => config.runtime.reasoning_enabled = Some(false),
            _ => {}
        }
    }

    // 已废弃的推理级别别名：VIBEWINDOW_REASONING_LEVEL 或 REASONING_LEVEL
    // 此环境变量已废弃，建议使用 config 中的 provider.reasoning_level 配置
    let alias_level = std::env::var("VIBEWINDOW_REASONING_LEVEL")
        .ok()
        .map(|value| ("VIBEWINDOW_REASONING_LEVEL", value))
        .or_else(|| std::env::var("REASONING_LEVEL").ok().map(|value| ("REASONING_LEVEL", value)));
    if let Some((env_name, level)) = alias_level {
        if let Some(normalized) = Config::normalize_reasoning_level_override(Some(&level), env_name)
        {
            tracing::warn!(
                env_name,
                reasoning_level = %normalized,
                "{env_name} 已废弃，建议使用 config 中的 provider.reasoning_level 配置"
            );
            config.runtime.reasoning_level = Some(normalized);
        }
    }

    // ==================== 视觉支持配置 ====================
    // 视觉支持覆盖：VIBEWINDOW_MODEL_SUPPORT_VISION 或 MODEL_SUPPORT_VISION
    if let Ok(flag) = std::env::var("VIBEWINDOW_MODEL_SUPPORT_VISION")
        .or_else(|_| std::env::var("MODEL_SUPPORT_VISION"))
    {
        let normalized = flag.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "1" | "true" | "yes" | "on" => config.model_support_vision = Some(true),
            "0" | "false" | "no" | "off" => config.model_support_vision = Some(false),
            _ => {}
        }
    }

    // ==================== 网络搜索配置 ====================
    // 网络搜索启用：VIBEWINDOW_WEB_SEARCH_ENABLED 或 WEB_SEARCH_ENABLED
    if let Ok(enabled) = std::env::var("VIBEWINDOW_WEB_SEARCH_ENABLED")
        .or_else(|_| std::env::var("WEB_SEARCH_ENABLED"))
    {
        config.web_search.enabled = enabled == "1" || enabled.eq_ignore_ascii_case("true");
    }

    // 网络搜索 Provider：VIBEWINDOW_WEB_SEARCH_PROVIDER 或 WEB_SEARCH_PROVIDER
    if let Ok(provider) = std::env::var("VIBEWINDOW_WEB_SEARCH_PROVIDER")
        .or_else(|_| std::env::var("WEB_SEARCH_PROVIDER"))
    {
        let provider = provider.trim();
        if !provider.is_empty() {
            config.web_search.provider = provider.to_string();
        }
    }

    // Brave API 密钥：VIBEWINDOW_BRAVE_API_KEY 或 BRAVE_API_KEY
    if let Ok(api_key) =
        std::env::var("VIBEWINDOW_BRAVE_API_KEY").or_else(|_| std::env::var("BRAVE_API_KEY"))
    {
        let api_key = api_key.trim();
        if !api_key.is_empty() {
            config.web_search.brave_api_key = Some(api_key.to_string());
        }
    }

    // 网络搜索最大结果数：VIBEWINDOW_WEB_SEARCH_MAX_RESULTS 或 WEB_SEARCH_MAX_RESULTS
    // 有效范围：1 到 10
    if let Ok(max_results) = std::env::var("VIBEWINDOW_WEB_SEARCH_MAX_RESULTS")
        .or_else(|_| std::env::var("WEB_SEARCH_MAX_RESULTS"))
    {
        if let Ok(max_results) = max_results.parse::<usize>() {
            if (1..=10).contains(&max_results) {
                config.web_search.max_results = max_results;
            }
        }
    }

    // 网络搜索超时：VIBEWINDOW_WEB_SEARCH_TIMEOUT_SECS 或 WEB_SEARCH_TIMEOUT_SECS
    if let Ok(timeout_secs) = std::env::var("VIBEWINDOW_WEB_SEARCH_TIMEOUT_SECS")
        .or_else(|_| std::env::var("WEB_SEARCH_TIMEOUT_SECS"))
    {
        if let Ok(timeout_secs) = timeout_secs.parse::<u64>() {
            if timeout_secs > 0 {
                config.web_search.timeout_secs = timeout_secs;
            }
        }
    }

    // ==================== 存储配置 ====================
    // 存储 Provider 密钥（可选后端覆盖）：VIBEWINDOW_STORAGE_PROVIDER
    if let Ok(provider) = std::env::var("VIBEWINDOW_STORAGE_PROVIDER") {
        let provider = provider.trim();
        if !provider.is_empty() {
            config.storage.provider.config.provider = provider.to_string();
        }
    }

    // 存储连接 URL（用于远程后端）：VIBEWINDOW_STORAGE_DB_URL
    if let Ok(db_url) = std::env::var("VIBEWINDOW_STORAGE_DB_URL") {
        let db_url = db_url.trim();
        if !db_url.is_empty() {
            config.storage.provider.config.db_url = Some(db_url.to_string());
        }
    }

    // 存储连接超时：VIBEWINDOW_STORAGE_CONNECT_TIMEOUT_SECS
    if let Ok(timeout_secs) = std::env::var("VIBEWINDOW_STORAGE_CONNECT_TIMEOUT_SECS") {
        if let Ok(timeout_secs) = timeout_secs.parse::<u64>() {
            if timeout_secs > 0 {
                config.storage.provider.config.connect_timeout_secs = Some(timeout_secs);
            }
        }
    }

    // ==================== 代理配置 ====================
    // 代理启用标志：VIBEWINDOW_PROXY_ENABLED
    let explicit_proxy_enabled =
        std::env::var("VIBEWINDOW_PROXY_ENABLED").ok().as_deref().and_then(parse_proxy_enabled);
    if let Some(enabled) = explicit_proxy_enabled {
        config.proxy.enabled = enabled;
    }

    // 代理 URL：VIBEWINDOW_* 优先，然后是通用 *PROXY 变量
    let mut proxy_url_overridden = false;
    if let Ok(proxy_url) =
        std::env::var("VIBEWINDOW_HTTP_PROXY").or_else(|_| std::env::var("HTTP_PROXY"))
    {
        config.proxy.http_proxy = normalize_proxy_url_option(Some(&proxy_url));
        proxy_url_overridden = true;
    }
    if let Ok(proxy_url) =
        std::env::var("VIBEWINDOW_HTTPS_PROXY").or_else(|_| std::env::var("HTTPS_PROXY"))
    {
        config.proxy.https_proxy = normalize_proxy_url_option(Some(&proxy_url));
        proxy_url_overridden = true;
    }
    if let Ok(proxy_url) =
        std::env::var("VIBEWINDOW_ALL_PROXY").or_else(|_| std::env::var("ALL_PROXY"))
    {
        config.proxy.all_proxy = normalize_proxy_url_option(Some(&proxy_url));
        proxy_url_overridden = true;
    }
    if let Ok(no_proxy) =
        std::env::var("VIBEWINDOW_NO_PROXY").or_else(|_| std::env::var("NO_PROXY"))
    {
        config.proxy.no_proxy = normalize_no_proxy_list(vec![no_proxy]);
    }

    // 如果未显式设置启用标志，但代理 URL 被覆盖且至少有一个有效的代理 URL，则自动启用代理
    if explicit_proxy_enabled.is_none() && proxy_url_overridden && config.proxy.has_any_proxy_url()
    {
        config.proxy.enabled = true;
    }

    // 代理作用域和服务选择器配置
    if let Ok(scope_raw) = std::env::var("VIBEWINDOW_PROXY_SCOPE") {
        if let Some(scope) = parse_proxy_scope(&scope_raw) {
            config.proxy.scope = scope;
        } else {
            tracing::warn!(
                scope = %scope_raw,
                "忽略无效的 VIBEWINDOW_PROXY_SCOPE 值（有效值：environment|vibewindow|services）"
            );
        }
    }

    if let Ok(services_raw) = std::env::var("VIBEWINDOW_PROXY_SERVICES") {
        config.proxy.services = normalize_service_list(vec![services_raw]);
    }

    // 验证代理配置，如果无效则禁用代理并记录警告
    if let Err(error) = validate_proxy_config(&config.proxy) {
        tracing::warn!("无效的代理配置已忽略: {error}");
        config.proxy.enabled = false;
    }

    // 如果代理启用且作用域为 Environment，则应用到进程环境变量
    // 这会影响进程中所有 HTTP 客户端的行为
    if config.proxy.enabled && config.proxy.scope == ProxyScope::Environment {
        apply_proxy_to_process_env(&config.proxy);
    }

    // 设置运行时代理配置，供其他组件使用
    set_runtime_proxy_config(config.proxy.clone());
}

//! 配置语义诊断检查。
//!
//! 本模块为 `doctor` 命令提供配置层面的健康检查：确认配置文件存在、provider 可创建、
//! 默认模型和温度范围合理，并对路由、embedding 与 agent 配置给出可操作的诊断项。

use super::DiagItem;
use crate::app::agent::config::Config;
use crate::app::agent::config::schema::ChannelsConfigExt;

/// 检查已加载配置的语义一致性。
///
/// 参数 `config` 是运行时加载后的配置快照，`items` 用于追加诊断结果。函数不返回
/// 错误；所有异常都会转换为 ok/warn/error 级别的 `DiagItem`，便于 doctor 命令一次
/// 展示完整问题列表。
pub(super) fn check_config_semantics(config: &Config, items: &mut Vec<DiagItem>) {
    let cat = "config";

    if config.config_path.exists() {
        items.push(DiagItem::ok(cat, format!("config file: {}", config.config_path.display())));
    } else {
        items.push(DiagItem::error(
            cat,
            format!("config file not found: {}", config.config_path.display()),
        ));
    }

    if let Some(ref provider) = config.default_provider {
        if let Some(reason) = provider_validation_error(provider) {
            items.push(DiagItem::error(
                cat,
                format!("default provider \"{provider}\" is invalid: {reason}"),
            ));
        } else {
            items.push(DiagItem::ok(cat, format!("provider \"{provider}\" is valid")));
        }
    } else {
        items.push(DiagItem::error(cat, "no default_provider configured"));
    }

    // Ollama 默认依赖本地服务，不要求 API key；其他 provider 缺少 key 时只警告，
    // 因为调用方仍可能通过环境变量或 provider 默认机制注入凭证。
    if config.default_provider.as_deref() != Some("ollama") {
        if config.api_key.is_some() {
            items.push(DiagItem::ok(cat, "API key configured"));
        } else {
            items.push(DiagItem::warn(
                cat,
                "no api_key set (may rely on env vars or provider defaults)",
            ));
        }
    }

    if config.default_model.is_some() {
        items.push(DiagItem::ok(
            cat,
            format!("default model: {}", config.default_model.as_deref().unwrap_or("?")),
        ));
    } else {
        items.push(DiagItem::warn(cat, "no default_model configured"));
    }

    if (0.0..=2.0).contains(&config.default_temperature) {
        items.push(DiagItem::ok(
            cat,
            format!("temperature {:.1} (valid range 0.0–2.0)", config.default_temperature),
        ));
    } else {
        items.push(DiagItem::error(
            cat,
            format!(
                "temperature {:.1} is out of range (expected 0.0–2.0)",
                config.default_temperature
            ),
        ));
    }

    let port = config.gateway.port;
    if port > 0 {
        items.push(DiagItem::ok(cat, format!("gateway port: {port}")));
    } else {
        items.push(DiagItem::error(cat, "gateway port is 0 (invalid)"));
    }

    for route in &config.model_routes {
        if route.hint.is_empty() {
            items.push(DiagItem::warn(cat, "model route with empty hint"));
        }
        if let Some(reason) = provider_validation_error(&route.provider) {
            items.push(DiagItem::warn(
                cat,
                format!(
                    "model route \"{}\" uses invalid provider \"{}\": {}",
                    route.hint, route.provider, reason
                ),
            ));
        }
        if route.model.is_empty() {
            items.push(DiagItem::warn(
                cat,
                format!("model route \"{}\" has empty model", route.hint),
            ));
        }
    }

    for route in &config.embedding_routes {
        if route.hint.trim().is_empty() {
            items.push(DiagItem::warn(cat, "embedding route with empty hint"));
        }
        if let Some(reason) = embedding_provider_validation_error(&route.provider) {
            items.push(DiagItem::warn(
                cat,
                format!(
                    "embedding route \"{}\" uses invalid provider \"{}\": {}",
                    route.hint, route.provider, reason
                ),
            ));
        }
        if route.model.trim().is_empty() {
            items.push(DiagItem::warn(
                cat,
                format!("embedding route \"{}\" has empty model", route.hint),
            ));
        }
        if route.dimensions.is_some_and(|value| value == 0) {
            items.push(DiagItem::warn(
                cat,
                format!("embedding route \"{}\" has invalid dimensions=0", route.hint),
            ));
        }
    }

    // memory.embedding_model 支持 hint 引用；这里提前提示缺失路由，避免运行时才发现
    // embedding 后端无法解析。
    if let Some(hint) = config
        .memory
        .embedding_model
        .strip_prefix("hint:")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !config.embedding_routes.iter().any(|route| route.hint.trim() == hint) {
            items.push(DiagItem::warn(
                cat,
                format!(
                    "memory.embedding_model uses hint \"{hint}\" but no matching [[embedding_routes]] entry exists"
                ),
            ));
        }
    }

    let has_channel = config.channels_config.channels().iter().any(|(_, enabled)| *enabled);
    if has_channel {
        items.push(DiagItem::ok(cat, "at least one channel configured"));
    } else {
        items.push(DiagItem::warn(cat, "no channels configured"));
    }

    let mut agent_names: Vec<_> = config.agents.keys().collect();
    agent_names.sort();
    for name in agent_names {
        let agent = config.agents.get(name).expect("agent key derived from map keys");
        if !agent.enabled {
            continue;
        }
        if let Some(reason) = provider_validation_error(&agent.provider) {
            items.push(DiagItem::warn(
                cat,
                format!(
                    "agent \"{name}\" uses invalid provider \"{}\": {}",
                    agent.provider, reason
                ),
            ));
        }
    }
}

/// 校验普通 LLM provider 名称是否能创建实例。
///
/// 参数 `name` 是配置中的 provider 名称。返回 `None` 表示有效；返回 `Some` 时包含
/// 首行错误原因，供诊断输出保持简短。
pub(super) fn provider_validation_error(name: &str) -> Option<String> {
    match crate::app::agent::providers::create_provider(name, None) {
        Ok(_) => None,
        Err(err) => Some(err.to_string().lines().next().unwrap_or("invalid provider").into()),
    }
}

/// 校验 embedding provider 配置。
///
/// 参数 `name` 支持 `none`、`openai`、`alibaba`、`alibaba-cn` 或 `custom:<url>`。
/// 返回 `None` 表示有效；返回 `Some` 时说明不支持的值、空 URL 或 URL scheme 错误。
/// 只允许 http/https，避免把 embedding 请求导向未定义的传输协议。
pub(super) fn embedding_provider_validation_error(name: &str) -> Option<String> {
    let normalized = name.trim();

    if normalized.eq_ignore_ascii_case("none")
        || normalized.eq_ignore_ascii_case("openai")
        || normalized.eq_ignore_ascii_case("alibaba")
        || normalized.eq_ignore_ascii_case("alibaba-cn")
    {
        return None;
    }

    let Some(url) = normalized.strip_prefix("custom:") else {
        return Some("supported values: none, openai, alibaba, alibaba-cn, custom:<url>".into());
    };

    let url = url.trim();
    if url.is_empty() {
        return Some("custom provider requires a non-empty URL after 'custom:'".into());
    }

    match reqwest::Url::parse(url) {
        Ok(parsed) if matches!(parsed.scheme(), "http" | "https") => None,
        Ok(parsed) => {
            Some(format!("custom provider URL must use http/https, got '{}'", parsed.scheme()))
        }
        Err(err) => Some(format!("invalid custom provider URL: {err}")),
    }
}

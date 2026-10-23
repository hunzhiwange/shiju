//! ACP 会话配置解析。
//!
//! 本模块把模型选项、用户覆盖项和内置 ACP agent 规格合并为可启动的 ACP 命令与
//! 会话选项。解析逻辑保持显式，便于在不同 agent 的兼容配置键之间做可审计映射。

use serde_json::Value;
use vw_acp::{
    AcpSessionOptions, AuthPolicy, DEFAULT_AGENT_NAME, NonInteractivePermissionPolicy,
    PermissionMode, resolve_agent_spec_with_overrides, resolve_compatible_config_id,
};

use crate::app::agent::config;
use crate::app::agent::provider::provider;

use super::{Error, ParsedAcpOptions, acp_option_error};

/// 规范化 ACP agent 配置。
///
/// `acp_agent_name` 是用户或模型选择的 agent 名称，`acp_cfg` 是配置层命令。返回值
/// 可能是原配置，也可能把历史 Claude Code ACP 包重写为当前兼容组件，同时保留用户
/// 配置的环境变量。
pub(crate) fn normalize_acp_agent_config(
    acp_agent_name: &str,
    acp_cfg: &config::schema::AcpAgentConfig,
) -> config::schema::AcpAgentConfig {
    let normalized_name = acp_agent_name.trim().to_ascii_lowercase();
    let first_arg = acp_cfg.args.first().map(|value| value.trim()).unwrap_or_default();
    let is_legacy_zed_claude = acp_cfg.command.trim() == "npx"
        && first_arg == "@zed-industries/claude-code-acp@latest"
        && normalized_name == "claude code";

    if !is_legacy_zed_claude {
        return acp_cfg.clone();
    }

    let Some(spec) = vw_acp::resolve_agent_spec(acp_agent_name) else {
        return acp_cfg.clone();
    };

    tracing::warn!(
        target: "vw_agent",
        acp_agent = %acp_agent_name,
        "rewriting legacy Claude Code ACP package to claude-agent-acp compatibility component"
    );

    // 只重写已知的历史包名，避免把用户自定义命令误认为需要兼容迁移。
    let mut normalized = to_schema_acp_config(spec);
    normalized.env = acp_cfg.env.clone();
    normalized
}

/// 构造用于展示或日志的 ACP 命令行。
///
/// `acp_cfg` 提供命令和参数。返回值会过滤空参数并用空格连接；它不是 shell 转义器，
/// 不应用作安全执行边界。
pub(crate) fn build_acp_command_line(acp_cfg: &config::schema::AcpAgentConfig) -> String {
    std::iter::once(acp_cfg.command.trim())
        .chain(
            acp_cfg
                .args
                .iter()
                .map(String::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .collect::<Vec<_>>()
        .join(" ")
}

/// 将配置中的 ACP 覆盖项转换为 `vw_acp` 规格表。
fn acp_override_specs(
    cfg: &config::schema::Config,
) -> std::collections::HashMap<String, vw_acp::AgentCommandSpec> {
    cfg.acp
        .iter()
        .map(|(name, spec)| {
            (
                name.clone(),
                vw_acp::AgentCommandSpec {
                    display_name: name.trim().to_string(),
                    command: spec.command.clone(),
                    args: spec.args.clone(),
                    env: spec.env.clone(),
                },
            )
        })
        .collect()
}

/// 将 `vw_acp` agent 规格转换为配置 schema 类型。
fn to_schema_acp_config(spec: vw_acp::AgentCommandSpec) -> config::schema::AcpAgentConfig {
    config::schema::AcpAgentConfig { command: spec.command, args: spec.args, env: spec.env }
}

/// 查找当前模型应使用的 ACP 命令配置。
///
/// `cfg` 提供用户覆盖项，`model` 提供模型默认偏好，`merged_options` 可通过
/// `acp_agent` 显式指定 agent。返回匹配到的 agent 名称和配置；没有任何可用规格时
/// 返回 `None`。
pub(crate) fn lookup_acp_command(
    cfg: &config::schema::Config,
    model: &provider::Model,
    merged_options: &Value,
) -> Option<(String, config::schema::AcpAgentConfig)> {
    let overrides = acp_override_specs(cfg);
    let explicit_preferred = merged_options
        .get("acp_agent")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());

    if let Some(preferred) = explicit_preferred {
        // 显式选项优先级最高，方便单次会话覆盖模型默认 agent。
        return resolve_agent_spec_with_overrides(preferred, Some(&overrides))
            .map(|spec| (preferred.to_string(), to_schema_acp_config(spec)));
    }

    let model_preferred = model.api.id.trim();
    if !model_preferred.is_empty() {
        if let Some(spec) = resolve_agent_spec_with_overrides(model_preferred, Some(&overrides)) {
            return Some((model_preferred.to_string(), to_schema_acp_config(spec)));
        }
    }

    resolve_agent_spec_with_overrides(DEFAULT_AGENT_NAME, Some(&overrides))
        .map(|spec| (DEFAULT_AGENT_NAME.to_string(), to_schema_acp_config(spec)))
}

/// 解析字符串形式的枚举选项。
///
/// `merged_options` 是合并后的模型选项，`key` 是选项名。缺失返回 `Ok(None)`；类型
/// 非字符串或枚举值非法时返回 ACP 选项错误。
fn parse_enum_option<T>(merged_options: &Value, key: &str) -> Result<Option<T>, Error>
where
    T: serde::de::DeserializeOwned,
{
    let Some(value) = merged_options.get(key) else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        return Err(acp_option_error(format!("{key} must be a string")));
    };
    serde_json::from_value::<T>(Value::String(value.to_string()))
        .map(Some)
        .map_err(|_| acp_option_error(format!("invalid {key}: {value}")))
}

/// 解析非空字符串选项。
///
/// 缺失或空白字符串返回 `None`，非字符串返回错误。
fn parse_string_option(merged_options: &Value, key: &str) -> Result<Option<String>, Error> {
    let Some(value) = merged_options.get(key) else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        return Err(acp_option_error(format!("{key} must be a string")));
    };
    let trimmed = value.trim();
    Ok((!trimmed.is_empty()).then(|| trimmed.to_string()))
}

/// 解析布尔选项。
///
/// 缺失时返回 `false`，非布尔值返回错误。
fn parse_bool_option(merged_options: &Value, key: &str) -> Result<bool, Error> {
    let Some(value) = merged_options.get(key) else {
        return Ok(false);
    };
    value.as_bool().ok_or_else(|| acp_option_error(format!("{key} must be a boolean")))
}

/// 解析字符串数组选项。
///
/// 空数组或全部为空白字符串时返回 `None`；数组中出现非字符串元素会返回错误。
fn parse_string_array_option(
    merged_options: &Value,
    key: &str,
) -> Result<Option<Vec<String>>, Error> {
    let Some(value) = merged_options.get(key) else {
        return Ok(None);
    };
    let Some(items) = value.as_array() else {
        return Err(acp_option_error(format!("{key} must be an array of strings")));
    };

    let mut parsed = Vec::new();
    for item in items {
        let Some(item) = item.as_str() else {
            return Err(acp_option_error(format!("{key} must be an array of strings")));
        };
        let trimmed = item.trim();
        if !trimmed.is_empty() {
            parsed.push(trimmed.to_string());
        }
    }

    Ok((!parsed.is_empty()).then_some(parsed))
}

/// 解析 i64 整数选项。
///
/// 支持 JSON signed/unsigned integer。超出 i64 范围或非整数值会返回错误。
fn parse_i64_option(merged_options: &Value, key: &str) -> Result<Option<i64>, Error> {
    let Some(value) = merged_options.get(key) else {
        return Ok(None);
    };
    if let Some(value) = value.as_i64() {
        return Ok(Some(value));
    }
    if let Some(value) = value.as_u64() {
        return i64::try_from(value)
            .map(Some)
            .map_err(|_| acp_option_error(format!("{key} is too large")));
    }
    Err(acp_option_error(format!("{key} must be an integer")))
}

/// 将 ACP session config 值转换为字符串。
///
/// `value` 可以是字符串、数字、布尔或 null；null 和空字符串表示不传该配置。对象或
/// 数组会返回错误，避免把结构化数据静默序列化成 agent 不理解的格式。
fn config_option_value(value: &Value, key: &str) -> Result<Option<String>, Error> {
    if value.is_null() {
        return Ok(None);
    }
    if let Some(value) = value.as_str() {
        let trimmed = value.trim();
        return Ok((!trimmed.is_empty()).then(|| trimmed.to_string()));
    }
    if let Some(value) = value.as_bool() {
        return Ok(Some(value.to_string()));
    }
    if let Some(value) = value.as_i64() {
        return Ok(Some(value.to_string()));
    }
    if let Some(value) = value.as_u64() {
        return Ok(Some(value.to_string()));
    }
    if let Some(value) = value.as_f64() {
        return Ok(Some(value.to_string()));
    }
    Err(acp_option_error(format!("{key} must be a string, number, bool, or null")))
}

/// 解析 ACP 会话选项。
///
/// `merged_options` 是模型、provider 和调用侧合并后的选项；`acp_agent_name` 与
/// `acp_cfg` 用于兼容不同 agent 的配置键。返回 `ParsedAcpOptions`，其中包含权限、
/// 认证、会话模型、工具 allowlist、最大轮次以及兼容化后的 session config。
pub(crate) fn parse_acp_options(
    merged_options: &Value,
    acp_agent_name: &str,
    acp_cfg: &config::schema::AcpAgentConfig,
) -> Result<ParsedAcpOptions, Error> {
    let permission_mode =
        parse_enum_option::<PermissionMode>(merged_options, "acp_permission_mode")?;
    let full_access = parse_bool_option(merged_options, "full_access")?;
    let non_interactive_permissions = parse_enum_option::<NonInteractivePermissionPolicy>(
        merged_options,
        "acp_non_interactive_permissions",
    )?;
    let auth_policy = parse_enum_option::<AuthPolicy>(merged_options, "acp_auth_policy")?;
    let session_mode = parse_string_option(merged_options, "acp_session_mode")?;

    let model = parse_string_option(merged_options, "acp_session_model")?;
    let allowed_tools = parse_string_array_option(merged_options, "acp_allowed_tools")?;
    let max_turns = parse_i64_option(merged_options, "acp_max_turns")?;

    let session_options = (model.is_some() || allowed_tools.is_some() || max_turns.is_some())
        .then_some(AcpSessionOptions { model, allowed_tools, max_turns });

    let mut session_config_options = Vec::new();
    if let Some(config) = merged_options.get("acp_session_config") {
        let Some(config) = config.as_object() else {
            return Err(acp_option_error("acp_session_config must be an object"));
        };
        for (config_id, value) in config {
            if let Some(value) = config_option_value(value, config_id)? {
                session_config_options.push((
                    resolve_compatible_config_id(acp_agent_name, &acp_cfg.command, config_id),
                    value,
                ));
            }
        }
    }

    if let Some(value) = merged_options.get("reasoning_effort") {
        let effort = serde_json::from_value::<vw_api_types::ReasoningEffortDto>(value.clone())
            .map_err(|_| acp_option_error("reasoning_effort must be a supported string"))?;
        let value = match effort {
            vw_api_types::ReasoningEffortDto::None => "none",
            vw_api_types::ReasoningEffortDto::Low => "low",
            vw_api_types::ReasoningEffortDto::Medium => "medium",
            vw_api_types::ReasoningEffortDto::High => "high",
            vw_api_types::ReasoningEffortDto::Xhigh => "xhigh",
            vw_api_types::ReasoningEffortDto::Max => "max",
            vw_api_types::ReasoningEffortDto::Minimal => "minimal",
        };
        let config_id =
            resolve_compatible_config_id(acp_agent_name, &acp_cfg.command, "reasoning_effort");
        if !session_config_options.iter().any(|(existing, _)| existing == &config_id) {
            session_config_options.push((config_id, value.to_string()));
        }
    }

    for implicit_key in ["thought_level"] {
        let Some(value) = merged_options.get(implicit_key) else {
            continue;
        };
        let config_id =
            resolve_compatible_config_id(acp_agent_name, &acp_cfg.command, implicit_key);
        if session_config_options.iter().any(|(existing, _)| existing == &config_id) {
            continue;
        }
        // 隐式键只在显式 session config 未提供同一兼容键时补入，避免覆盖用户更精确
        // 的 agent 级配置。
        if let Some(value) = config_option_value(value, implicit_key)? {
            session_config_options.push((config_id, value));
        }
    }

    Ok(ParsedAcpOptions {
        permission_mode,
        full_access,
        non_interactive_permissions,
        auth_policy,
        session_mode,
        session_options,
        session_config_options,
    })
}
#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;

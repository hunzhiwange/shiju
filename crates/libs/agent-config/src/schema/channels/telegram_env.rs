//! Telegram `allowed_users` 环境变量引用解析。
//!
//! 该模块允许配置中的 Telegram 允许用户列表使用 `${env:NAME}` 形式引用环境变量。
//! 解析时会校验环境变量名、拒绝空值，并支持 JSON 数组或逗号分隔列表，避免把未解析或格式错误的
//! 授权列表带入运行时。

use crate::schema::ChannelsConfig;
use crate::schema::channels::helpers::is_valid_env_var_name;
use anyhow::{Context, Result};

fn parse_telegram_allowed_users_env_value(
    raw_value: &str,
    env_name: &str,
    field_name: &str,
) -> Result<Vec<String>> {
    let trimmed = raw_value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("{field_name} env reference ${{env:{env_name}}} resolved to an empty value");
    }

    let mut resolved: Vec<String> = Vec::new();
    if trimmed.starts_with('[') {
        // 环境变量可能来自部署系统，显式要求数组格式合法，避免把错误配置静默当作单个用户 ID。
        let parsed: serde_json::Value = serde_json::from_str(trimmed).with_context(|| {
            format!(
                "{field_name} env reference ${{env:{env_name}}} must be valid JSON array or comma-separated list"
            )
        })?;
        let items = parsed.as_array().with_context(|| {
            format!("{field_name} env reference ${{env:{env_name}}} must be a JSON array")
        })?;
        for (idx, item) in items.iter().enumerate() {
            let candidate = match item {
                serde_json::Value::String(v) => v.trim().to_string(),
                serde_json::Value::Number(v) => v.to_string(),
                _ => {
                    anyhow::bail!(
                        "{field_name} env reference ${{env:{env_name}}}[{idx}] must be string or number"
                    );
                }
            };
            if !candidate.is_empty() {
                resolved.push(candidate);
            }
        }
    } else {
        // 逗号分隔保留给简单部署场景；空片段会被忽略，但最终结果仍必须至少有一个有效 ID。
        resolved.extend(
            trimmed
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
        );
    }

    if resolved.is_empty() {
        anyhow::bail!("{field_name} env reference ${{env:{env_name}}} produced no user IDs");
    }

    Ok(resolved)
}

/// 展开 Telegram `allowed_users` 中的环境变量引用。
///
/// 参数 `channels` 会被原地更新：普通用户 ID 保持为去除首尾空白后的值，`${env:NAME}` 引用会被替换为
/// 环境变量中解析出的用户 ID 列表。若未启用 Telegram 渠道则直接返回成功。
///
/// 当环境变量名非法、变量未设置、变量值为空或变量值无法解析成非空用户列表时返回错误。这里对格式保持
/// 严格校验，是为了避免授权列表失效后意外放宽访问边界。
pub fn resolve_telegram_allowed_users_env_refs(channels: &mut ChannelsConfig) -> Result<()> {
    let Some(telegram) = channels.telegram.as_mut() else {
        return Ok(());
    };

    let field_name = "config.channels_config.telegram.allowed_users";
    let mut expanded_allowed_users: Vec<String> = Vec::new();
    for (idx, raw_entry) in telegram.allowed_users.drain(..).enumerate() {
        let entry = raw_entry.trim();
        if entry.is_empty() {
            continue;
        }

        if let Some(env_expr) =
            entry.strip_prefix("${env:").and_then(|value| value.strip_suffix('}'))
        {
            let env_name = env_expr.trim();
            // 环境变量名限制为 shell 常见安全子集，避免把拼写错误或插值片段误当成真实授权源。
            if !is_valid_env_var_name(env_name) {
                anyhow::bail!(
                    "{field_name}[{idx}] has invalid env var name ({env_name}); expected [A-Za-z_][A-Za-z0-9_]*"
                );
            }
            let env_value = std::env::var(env_name).with_context(|| {
                format!("{field_name}[{idx}] references unset environment variable {env_name}")
            })?;
            let mut parsed =
                parse_telegram_allowed_users_env_value(&env_value, env_name, field_name)?;
            expanded_allowed_users.append(&mut parsed);
        } else {
            expanded_allowed_users.push(entry.to_string());
        }
    }

    telegram.allowed_users = expanded_allowed_users;
    Ok(())
}

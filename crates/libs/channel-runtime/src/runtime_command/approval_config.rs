//! 非 CLI 工具授权状态的配置读写与摘要输出。
//!
//! 本模块只处理授权状态如何映射到运行时配置和用户可读摘要，不直接决定某次
//! 工具调用是否放行。WASM 目标没有本地配置文件写入能力，因此对应路径显式返回不可用。

use super::*;
use super::approval_target_label;

/// 将自然语言授权模式转换为稳定标签。
///
/// 参数：`mode` 是运行时策略枚举。
///
/// 返回值：用于追踪事件和状态摘要的蛇形字符串。
///
/// 错误处理：该函数不产生错误。
pub use channel_support::config::non_cli_natural_language_mode_label;

#[cfg(not(target_arch = "wasm32"))]
fn parse_runtime_config(contents: &str, config_path: &Path) -> Result<Config> {
    let mut parsed: Config = if contents.trim_start().starts_with('{') {
        serde_json::from_str(contents)
            .with_context(|| format!("Failed to parse {}", config_path.display()))?
    } else {
        toml::from_str(contents)
            .with_context(|| format!("Failed to parse {}", config_path.display()))?
    };
    parsed.config_path = config_path.to_path_buf();
    Ok(parsed)
}

/// 只更新读取来源中的审批列表，不能把项目配置写入默认租户数据库。
#[cfg(not(target_arch = "wasm32"))]
async fn save_approval_lists(contents: &str, path: &Path, config: &Config) -> Result<()> {
    // 保留未识别字段和已有密文，避免序列化整个 Config 时加入默认值。
    let mut payload: serde_json::Value = if contents.trim_start().starts_with('{') {
        serde_json::from_str(contents)?
    } else {
        serde_json::to_value(toml::from_str::<toml::Value>(contents)?)?
    };
    payload["autonomy"]["auto_approve"] = serde_json::to_value(&config.autonomy.auto_approve)?;
    payload["autonomy"]["always_ask"] = serde_json::to_value(&config.autonomy.always_ask)?;
    let encoded = serde_json::to_string_pretty(&payload)?;
    tokio::fs::write(path, encoded)
        .await
        .with_context(|| format!("Failed to save {}", path.display()))
}

/// 在 WASM 目标上跳过非 CLI 授权持久化。
///
/// 参数被保留以保持跨目标签名一致。
///
/// 返回值：始终返回 `Ok(None)`，表示没有可写配置路径。
///
/// 错误处理：WASM 分支不会触发文件系统错误。
#[cfg(target_arch = "wasm32")]
pub async fn persist_non_cli_approval_to_config(
    _ctx: &ChannelRuntimeContext,
    _tool_name: &str,
) -> Result<Option<PathBuf>> {
    Ok(None)
}

/// 将非 CLI 工具授权持久化到运行时配置。
///
/// 参数：
/// - `ctx`：提供运行时配置路径和当前策略。
/// - `tool_name`：要加入 `autonomy.auto_approve` 的工具名。
///
/// 返回值：写入或确认的配置路径；无配置路径时返回 `Ok(None)`。
///
/// 错误处理：读取、解析或保存配置失败时返回错误；未发生变更时不写文件。
#[cfg(not(target_arch = "wasm32"))]
pub async fn persist_non_cli_approval_to_config(
    ctx: &ChannelRuntimeContext,
    tool_name: &str,
) -> Result<Option<PathBuf>> {
    let Some(config_path) = runtime_config_path(ctx) else {
        return Ok(None);
    };

    let contents = tokio::fs::read_to_string(&config_path)
        .await
        .with_context(|| format!("Failed to read {}", config_path.display()))?;
    let mut parsed = parse_runtime_config(&contents, &config_path)?;

    let mut changed = false;
    if !parsed.autonomy.auto_approve.iter().any(|entry| entry == tool_name) {
        parsed.autonomy.auto_approve.push(tool_name.to_string());
        changed = true;
    }

    // auto_approve 与 always_ask 互斥，持久化批准时必须移除显式询问项。
    let before_always_ask = parsed.autonomy.always_ask.len();
    parsed.autonomy.always_ask.retain(|entry| entry != tool_name);
    if parsed.autonomy.always_ask.len() != before_always_ask {
        changed = true;
    }

    if changed {
        save_approval_lists(&contents, &config_path, &parsed).await?;
    }

    Ok(Some(config_path))
}

/// 在 WASM 目标上跳过非 CLI 授权撤销的持久化。
///
/// 参数被保留以保持跨目标签名一致。
///
/// 返回值：始终返回 `Ok(None)`，表示没有可写配置路径。
///
/// 错误处理：WASM 分支不会触发文件系统错误。
#[cfg(target_arch = "wasm32")]
pub async fn remove_non_cli_approval_from_config(
    _ctx: &ChannelRuntimeContext,
    _tool_name: &str,
) -> Result<Option<(PathBuf, bool)>> {
    Ok(None)
}

/// 从运行时配置中移除非 CLI 工具授权。
///
/// 参数：
/// - `ctx`：提供运行时配置路径。
/// - `tool_name`：要从 `autonomy.auto_approve` 移除的工具名。
///
/// 返回值：存在配置路径时返回路径和是否实际移除。
///
/// 错误处理：读取、解析或保存配置失败时返回错误；未找到条目时不写文件。
#[cfg(not(target_arch = "wasm32"))]
pub async fn remove_non_cli_approval_from_config(
    ctx: &ChannelRuntimeContext,
    tool_name: &str,
) -> Result<Option<(PathBuf, bool)>> {
    let Some(config_path) = runtime_config_path(ctx) else {
        return Ok(None);
    };

    let contents = tokio::fs::read_to_string(&config_path)
        .await
        .with_context(|| format!("Failed to read {}", config_path.display()))?;
    let mut parsed = parse_runtime_config(&contents, &config_path)?;

    let before_auto_approve = parsed.autonomy.auto_approve.len();
    parsed.autonomy.auto_approve.retain(|entry| entry != tool_name);
    let removed = parsed.autonomy.auto_approve.len() != before_auto_approve;

    if removed {
        save_approval_lists(&contents, &config_path, &parsed).await?;
    }

    Ok(Some((config_path, removed)))
}

/// 汇总当前非 CLI 授权状态。
///
/// 参数：
/// - `ctx`：通道运行时上下文。
/// - `sender`：当前请求发送者，用于筛选待审批请求。
/// - `channel`：当前通道名，用于展示通道级自然语言策略。
/// - `reply_target`：当前会话/聊天目标，用于筛选待审批请求。
///
/// 返回值：面向用户的多行状态摘要。
///
/// 错误处理：读取或解析持久化配置失败时返回错误，由调用方转换为回复。
pub async fn describe_non_cli_approvals(
    ctx: &ChannelRuntimeContext,
    sender: &str,
    channel: &str,
    reply_target: &str,
) -> Result<String> {
    let mut response = String::new();
    response.push_str("Supervised non-CLI tool approvals:\n");

    // 同时展示运行时有效状态和持久化配置，便于排查热加载或内存授权差异。
    let mut runtime_auto =
        ctx.approval_manager.auto_approve_tools().into_iter().collect::<Vec<_>>();
    runtime_auto.sort();
    if runtime_auto.is_empty() {
        response.push_str("- Runtime auto_approve (effective): (none)\n");
    } else {
        let _ =
            writeln!(response, "- Runtime auto_approve (effective): {}", runtime_auto.join(", "));
    }

    let mut runtime_always =
        ctx.approval_manager.always_ask_tools().into_iter().collect::<Vec<_>>();
    runtime_always.sort();
    if runtime_always.is_empty() {
        response.push_str("- Runtime always_ask (effective): (none)\n");
    } else {
        let _ =
            writeln!(response, "- Runtime always_ask (effective): {}", runtime_always.join(", "));
    }

    let mut session_grants =
        ctx.approval_manager.non_cli_session_allowlist().into_iter().collect::<Vec<_>>();
    session_grants.sort();
    if session_grants.is_empty() {
        response.push_str("- Runtime session grants: (none)\n");
    } else {
        let _ = writeln!(response, "- Runtime session grants: {}", session_grants.join(", "));
    }

    let one_time_all_tools_tokens = ctx.approval_manager.non_cli_allow_all_once_remaining();
    let _ = writeln!(
        response,
        "- Runtime one-time all-tools bypass tokens: {}",
        one_time_all_tools_tokens
    );

    let mut approval_approvers =
        ctx.approval_manager.non_cli_approval_approvers().into_iter().collect::<Vec<_>>();
    approval_approvers.sort();
    if approval_approvers.is_empty() {
        response.push_str("- Runtime non_cli_approval_approvers: (any channel-allowed sender)\n");
    } else {
        let _ = writeln!(
            response,
            "- Runtime non_cli_approval_approvers: {}",
            approval_approvers.join(", ")
        );
    }

    let default_mode = non_cli_natural_language_mode_label(
        ctx.approval_manager.non_cli_natural_language_approval_mode(),
    );
    let effective_mode = non_cli_natural_language_mode_label(
        ctx.approval_manager.non_cli_natural_language_approval_mode_for_channel(channel),
    );
    let _ =
        writeln!(response, "- Runtime non_cli_natural_language_approval_mode: {}", default_mode);
    let _ = writeln!(
        response,
        "- Runtime non_cli_natural_language_approval_mode (current channel `{channel}`): {}",
        effective_mode
    );

    let mut mode_overrides = ctx
        .approval_manager
        .non_cli_natural_language_approval_mode_by_channel()
        .into_iter()
        .map(|(ch, mode)| format!("{ch}={}", non_cli_natural_language_mode_label(mode)))
        .collect::<Vec<_>>();
    mode_overrides.sort();
    if mode_overrides.is_empty() {
        response.push_str("- Runtime non_cli_natural_language_approval_mode_by_channel: (none)\n");
    } else {
        let _ = writeln!(
            response,
            "- Runtime non_cli_natural_language_approval_mode_by_channel: {}",
            mode_overrides.join(", ")
        );
    }

    let mut pending_requests = ctx.approval_manager.list_non_cli_pending_requests(
        Some(sender),
        Some(channel),
        Some(reply_target),
    );
    pending_requests.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    if pending_requests.is_empty() {
        response.push_str("- Pending approvals (sender+chat/channel scoped): (none)\n");
    } else {
        response.push_str("- Pending approvals (sender+chat/channel scoped):\n");
        for req in pending_requests {
            let reason =
                req.reason.as_deref().filter(|text| !text.trim().is_empty()).unwrap_or("n/a");
            let _ = writeln!(
                response,
                "  - {}: tool={}, expires_at={}, reason={}",
                req.request_id,
                approval_target_label(&req.tool_name),
                req.expires_at,
                reason
            );
        }
    }

    let mut excluded = snapshot_non_cli_excluded_tools(ctx);
    excluded.sort();
    if excluded.is_empty() {
        response.push_str("- Runtime non_cli_excluded_tools: (none)\n");
    } else {
        let _ = writeln!(response, "- Runtime non_cli_excluded_tools: {}", excluded.join(", "));
    }

    let Some(config_path) = runtime_config_path(ctx) else {
        response.push_str(
            "- Persisted config approvals: unavailable (runtime config path not resolved)\n",
        );
        return Ok(response);
    };

    // 只在原生目标读取磁盘配置，WASM 明确报告不可用而不是伪造状态。
    #[cfg(not(target_arch = "wasm32"))]
    {
        let contents = tokio::fs::read_to_string(&config_path)
            .await
            .with_context(|| format!("Failed to read {}", config_path.display()))?;
        let parsed = parse_runtime_config(&contents, &config_path)?;

        let mut auto_approve = parsed.autonomy.auto_approve;
        auto_approve.sort();
        if auto_approve.is_empty() {
            response.push_str("- Persisted autonomy.auto_approve: (none)\n");
        } else {
            let _ = writeln!(
                response,
                "- Persisted autonomy.auto_approve: {}",
                auto_approve.join(", ")
            );
        }

        let mut always_ask = parsed.autonomy.always_ask;
        always_ask.sort();
        if always_ask.is_empty() {
            response.push_str("- Persisted autonomy.always_ask: (none)\n");
        } else {
            let _ =
                writeln!(response, "- Persisted autonomy.always_ask: {}", always_ask.join(", "));
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        response.push_str("- Persisted config: unavailable on WASM\n");
    }

    let _ = writeln!(response, "- Config path: {}", config_path.display());
    Ok(response)
}

#[cfg(test)]
#[path = "approval_config_tests.rs"]
mod approval_config_tests;

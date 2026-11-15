//! ACP 代理兼容超时和诊断文案。

use super::*;

/// 解析 Gemini ACP 初始化超时时间。
///
/// 环境变量缺失、非数字或小于等于零时使用默认值。
pub(super) fn resolve_gemini_acp_startup_timeout() -> Duration {
    resolve_timeout_from_env("VWACP_GEMINI_ACP_STARTUP_TIMEOUT_MS", 15_000)
}

/// 解析 Claude ACP 会话创建超时时间。
///
/// 环境变量缺失、非数字或小于等于零时使用默认值。
pub(super) fn resolve_claude_acp_session_create_timeout() -> Duration {
    resolve_timeout_from_env("VWACP_CLAUDE_ACP_SESSION_CREATE_TIMEOUT_MS", 60_000)
}

fn resolve_timeout_from_env(key: &str, default_ms: u64) -> Duration {
    let timeout_ms = std::env::var(key)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_ms);
    Duration::from_millis(timeout_ms)
}

/// 构造 Gemini ACP 初始化超时的诊断信息。
///
/// 如果没有检测到 API key，会在返回文案中提示非交互认证缺失。
pub(super) fn build_gemini_acp_startup_timeout_message(command: &str) -> String {
    let mut parts = vec![
        "Gemini CLI ACP startup timed out before initialize completed.".to_string(),
        "This usually means the local Gemini CLI is waiting on interactive OAuth or has incompatible ACP subprocess behavior.".to_string(),
    ];

    if std::env::var("GEMINI_API_KEY").ok().filter(|value| !value.trim().is_empty()).is_none()
        && std::env::var("GOOGLE_API_KEY").ok().filter(|value| !value.trim().is_empty()).is_none()
    {
        parts.push(
            "No GEMINI_API_KEY or GOOGLE_API_KEY was set for non-interactive auth.".to_string(),
        );
    }

    parts.push(format!(
        "Try upgrading Gemini CLI and using API-key-based auth for non-interactive ACP runs. Command: {command}"
    ));
    parts.join(" ")
}

/// 构造 Claude ACP 会话创建超时的诊断信息。
pub(super) fn build_claude_acp_session_create_timeout_message() -> String {
    [
        "Claude ACP session creation timed out before session/new completed.",
        "This matches the known persistent-session stall seen with some Claude Code and @agentclientprotocol/claude-agent-acp combinations.",
        "In harnessed or non-interactive runs, prefer --approve-all with nonInteractivePermissions=deny, upgrade Claude Code and the Claude ACP adapter, or use vwacp claude exec as a one-shot fallback.",
    ]
    .join(" ")
}

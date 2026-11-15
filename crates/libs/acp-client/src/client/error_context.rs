//! ACP 错误上下文补充与会话控制错误整理。

use super::*;

/// 将代理进程退出上下文补充到 ACP 错误中。
///
/// 仅对初始化和新建会话错误追加上下文，避免改变其他错误类型的语义。
pub(super) fn enrich_acp_error_with_process_context(
    err: AcpError,
    finalized: &FinalizedChild,
) -> AcpError {
    let Some(context) = format_process_context(&finalized.summary, &finalized.stderr_output) else {
        return err;
    };

    match err {
        AcpError::Initialize(message) => AcpError::Initialize(format!("{message} {context}")),
        AcpError::NewSession(message) => AcpError::NewSession(format!("{message} {context}")),
        other => other,
    }
}

fn format_process_context(summary: &ChildExitSummary, stderr_output: &str) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(code) = summary.exit_code {
        parts.push(format!("exit code {code}"));
    }
    if let Some(signal) = summary.signal.as_deref() {
        parts.push(format!("signal {signal}"));
    }

    let stderr = stderr_output.trim();
    if !stderr.is_empty() {
        let preview = stderr.chars().take(400).collect::<String>().replace('\n', " | ");
        parts.push(format!("stderr: {preview}"));
    }

    if parts.is_empty() {
        return None;
    }

    Some(format!("ACP agent process exited early ({})", parts.join(", ")))
}

/// 包装会话控制类 ACP 错误。
///
/// 该函数会识别常见 JSON-RPC 错误码，给不支持的 `session/set_*` 能力提供更清晰
/// 的提示；无法解析时保留原始错误文本。
pub(super) fn wrap_session_control_error<E>(
    method: &'static str,
    context: Option<String>,
    error: E,
    constructor: fn(String) -> AcpError,
) -> AcpError
where
    E: StdError + Send + Sync + 'static,
{
    let raw = error.to_string();
    if let Some(acp) = parse_session_control_acp_summary(&raw) {
        let summary = format_session_control_acp_summary(&acp);
        if is_likely_session_control_unsupported_error(&acp) {
            let context_suffix =
                context.as_deref().map(|value| format!(" {value}")).unwrap_or_default();
            return constructor(format!(
                "Agent rejected {method}{context_suffix}: {summary}. The adapter may not implement {method}, or the requested value is not supported."
            ));
        }
        return constructor(format!(
            "Failed {method}{}: {summary}",
            context.as_deref().map(|value| format!(" {value}")).unwrap_or_default()
        ));
    }

    constructor(format!(
        "Failed {method}{}: {raw}",
        context.as_deref().map(|value| format!(" {value}")).unwrap_or_default()
    ))
}

#[derive(Debug)]
struct SessionControlAcpSummary {
    code: i64,
    message: String,
    details: Option<String>,
}

fn parse_session_control_acp_summary(raw: &str) -> Option<SessionControlAcpSummary> {
    let code = [-32601_i64, -32602, -32603]
        .into_iter()
        .find(|candidate| raw.contains(&candidate.to_string()))?;

    let lower = raw.to_ascii_lowercase();
    let message = if lower.contains("method not found") {
        "method not found".to_string()
    } else if lower.contains("invalid params") {
        "invalid params".to_string()
    } else if lower.contains("internal error") {
        "internal error".to_string()
    } else {
        raw.trim().to_string()
    };

    Some(SessionControlAcpSummary { code, message, details: extract_details_field(raw) })
}

fn is_likely_session_control_unsupported_error(acp: &SessionControlAcpSummary) -> bool {
    if matches!(acp.code, -32601 | -32602) {
        return true;
    }
    acp.code == -32603
        && acp
            .details
            .as_deref()
            .is_some_and(|details| details.to_ascii_lowercase().contains("invalid params"))
}

fn format_session_control_acp_summary(acp: &SessionControlAcpSummary) -> String {
    if let Some(details) = acp.details.as_deref().filter(|details| !details.trim().is_empty()) {
        return format!(
            "{} (ACP {}, adapter reported \"{}\")",
            details.trim(),
            acp.code,
            acp.message
        );
    }
    format!("{} (ACP {})", acp.message, acp.code)
}

fn extract_details_field(raw: &str) -> Option<String> {
    for needle in ["details: ", "\"details\":\"", "\"details\": \""] {
        if let Some(index) = raw.find(needle) {
            let tail = &raw[index + needle.len()..];
            if let Some(extracted) = extract_quoted_or_delimited_text(tail) {
                return Some(extracted);
            }
        }
    }
    None
}

fn extract_quoted_or_delimited_text(raw: &str) -> Option<String> {
    let trimmed = raw.trim_start();
    let mut chars = trimmed.chars();
    let first = chars.next()?;
    if matches!(first, '"' | '\'') {
        let rest = chars.as_str();
        let end = rest.find(first)?;
        return Some(rest[..end].to_string());
    }

    let end = trimmed.find([',', '}', '\n']).unwrap_or(trimmed.len());
    let candidate = trimmed[..end].trim().trim_matches(|ch| matches!(ch, '"' | '\'')).to_string();
    if candidate.is_empty() {
        return None;
    }
    Some(candidate)
}

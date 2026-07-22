//! 输出错误的标准化、消息整理与退出码映射。

use serde_json::Value;

use crate::acp_error_shapes::{
    extract_acp_error, format_unknown_error_message, is_acp_resource_not_found_error,
};
use crate::types::{
    EXIT_CODE_ERROR, EXIT_CODE_NO_SESSION, EXIT_CODE_PERMISSION_DENIED, EXIT_CODE_TIMEOUT,
    EXIT_CODE_USAGE, ExitCode, OutputErrorAcpPayload, OutputErrorCode, OutputErrorOrigin,
    OutputErrorParams,
};

const AUTH_REQUIRED_ACP_CODES: [i64; 1] = [-32000];
const QUERY_CLOSED_BEFORE_RESPONSE_DETAIL: &str = "query closed before response received";

#[derive(Debug, Clone, Default)]
pub struct NormalizeOutputErrorOptions {
    pub default_code: Option<OutputErrorCode>,
    pub detail_code: Option<String>,
    pub origin: Option<OutputErrorOrigin>,
    pub retryable: Option<bool>,
    pub acp: Option<OutputErrorAcpPayload>,
}

#[derive(Debug, Default)]
struct ErrorMeta {
    output_code: Option<OutputErrorCode>,
    detail_code: Option<String>,
    origin: Option<OutputErrorOrigin>,
    retryable: Option<bool>,
    acp: Option<OutputErrorAcpPayload>,
}

fn as_record(value: &Value) -> Option<&serde_json::Map<String, Value>> {
    match value {
        Value::Object(record) => Some(record),
        _ => None,
    }
}

fn parse_output_error_code(value: &Value) -> Option<OutputErrorCode> {
    match value.as_str()? {
        "NO_SESSION" => Some(OutputErrorCode::NoSession),
        "TIMEOUT" => Some(OutputErrorCode::Timeout),
        "PERMISSION_DENIED" => Some(OutputErrorCode::PermissionDenied),
        "PERMISSION_PROMPT_UNAVAILABLE" => Some(OutputErrorCode::PermissionPromptUnavailable),
        "RUNTIME" => Some(OutputErrorCode::Runtime),
        "USAGE" => Some(OutputErrorCode::Usage),
        _ => None,
    }
}

fn parse_output_error_origin(value: &Value) -> Option<OutputErrorOrigin> {
    match value.as_str()? {
        "cli" => Some(OutputErrorOrigin::Cli),
        "runtime" => Some(OutputErrorOrigin::Runtime),
        "queue" => Some(OutputErrorOrigin::Queue),
        "acp" => Some(OutputErrorOrigin::Acp),
        _ => None,
    }
}

fn is_named_error(value: &Value, expected: &str) -> bool {
    as_record(value)
        .and_then(|record| record.get("name"))
        .and_then(Value::as_str)
        .is_some_and(|name| name == expected)
}

fn is_auth_required_message(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    normalized.contains("auth required")
        || normalized.contains("authentication required")
        || normalized.contains("authorization required")
        || normalized.contains("credential required")
        || normalized.contains("credentials required")
        || normalized.contains("token required")
        || normalized.contains("login required")
}

fn is_acp_auth_required_payload(acp: Option<&OutputErrorAcpPayload>) -> bool {
    let Some(acp) = acp else {
        return false;
    };
    if !AUTH_REQUIRED_ACP_CODES.contains(&acp.code) {
        return false;
    }
    if is_auth_required_message(acp.message.as_str()) {
        return true;
    }

    let Some(data) = acp.data.as_ref().and_then(as_record) else {
        return false;
    };
    if data.get("authRequired").and_then(Value::as_bool) == Some(true) {
        return true;
    }
    if data
        .get("methodId")
        .and_then(Value::as_str)
        .is_some_and(|method_id| !method_id.trim().is_empty())
    {
        return true;
    }
    data.get("methods").and_then(Value::as_array).is_some_and(|methods| !methods.is_empty())
}

fn read_output_error_meta(error: &Value) -> ErrorMeta {
    let Some(record) = as_record(error) else {
        return ErrorMeta::default();
    };

    let detail_code = record
        .get("detailCode")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|detail_code| !detail_code.is_empty())
        .map(ToOwned::to_owned);

    ErrorMeta {
        output_code: record.get("outputCode").and_then(parse_output_error_code),
        detail_code,
        origin: record.get("origin").and_then(parse_output_error_origin),
        retryable: record.get("retryable").and_then(Value::as_bool),
        acp: record.get("acp").and_then(extract_acp_error),
    }
}

fn is_timeout_like(error: &Value) -> bool {
    is_named_error(error, "TimeoutError")
}

fn is_no_session_like(error: &Value) -> bool {
    is_named_error(error, "NoSessionError")
}

fn is_permission_denied_like(error: &Value) -> bool {
    is_named_error(error, "PermissionDeniedError")
}

fn is_permission_prompt_unavailable_like(error: &Value) -> bool {
    is_named_error(error, "PermissionPromptUnavailableError")
}

fn is_auth_policy_like(error: &Value) -> bool {
    is_named_error(error, "AuthPolicyError")
}

fn is_usage_like(error: &Value) -> bool {
    let Some(record) = as_record(error) else {
        return false;
    };
    record
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|name| matches!(name, "CommanderError" | "InvalidArgumentError"))
        || record.get("code").and_then(Value::as_str) == Some("commander.invalidArgument")
}

fn map_error_code(error: &Value) -> Option<OutputErrorCode> {
    if is_permission_prompt_unavailable_like(error) {
        return Some(OutputErrorCode::PermissionPromptUnavailable);
    }
    if is_permission_denied_like(error) {
        return Some(OutputErrorCode::PermissionDenied);
    }
    if is_timeout_like(error) {
        return Some(OutputErrorCode::Timeout);
    }
    if is_no_session_like(error) || is_acp_resource_not_found_error(error) {
        return Some(OutputErrorCode::NoSession);
    }
    if is_usage_like(error) {
        return Some(OutputErrorCode::Usage);
    }
    None
}

pub fn format_error_message(error: &Value) -> String {
    format_unknown_error_message(error)
}

pub fn is_acp_query_closed_before_response_error(error: &Value) -> bool {
    let Some(acp) = extract_acp_error(error) else {
        return false;
    };
    if acp.code != -32603 {
        return false;
    }

    acp.data
        .as_ref()
        .and_then(as_record)
        .and_then(|data| data.get("details"))
        .and_then(Value::as_str)
        .is_some_and(|details| {
            details.to_ascii_lowercase().contains(QUERY_CLOSED_BEFORE_RESPONSE_DETAIL)
        })
}

pub fn normalize_output_error(
    error: &Value,
    options: NormalizeOutputErrorOptions,
) -> OutputErrorParams {
    let meta = read_output_error_meta(error);
    let mapped = map_error_code(error);
    let mut code = mapped.or(options.default_code).unwrap_or(OutputErrorCode::Runtime);

    if let Some(output_code) = meta.output_code {
        code = output_code;
    }

    if code == OutputErrorCode::Runtime && is_acp_resource_not_found_error(error) {
        code = OutputErrorCode::NoSession;
    }

    let acp = options.acp.or(meta.acp).or_else(|| extract_acp_error(error));
    let detail_code = meta.detail_code.or(options.detail_code).or_else(|| {
        if is_auth_policy_like(error) || is_acp_auth_required_payload(acp.as_ref()) {
            Some("AUTH_REQUIRED".to_string())
        } else {
            None
        }
    });

    OutputErrorParams {
        code,
        message: format_error_message(error),
        detail_code,
        origin: meta.origin.or(options.origin),
        retryable: meta.retryable.or(options.retryable),
        acp,
        timestamp: None,
    }
}

pub fn is_retryable_prompt_error(error: &Value) -> bool {
    let meta = read_output_error_meta(error);
    if matches!(
        meta.output_code,
        Some(
            OutputErrorCode::PermissionDenied
                | OutputErrorCode::PermissionPromptUnavailable
                | OutputErrorCode::Timeout
                | OutputErrorCode::NoSession
                | OutputErrorCode::Usage
        )
    ) {
        return false;
    }
    if meta.detail_code.as_deref() == Some("AUTH_REQUIRED") {
        return false;
    }
    if is_permission_denied_like(error) || is_permission_prompt_unavailable_like(error) {
        return false;
    }
    if is_timeout_like(error) || is_no_session_like(error) || is_usage_like(error) {
        return false;
    }

    let Some(acp) = meta.acp.or_else(|| extract_acp_error(error)) else {
        return false;
    };
    if matches!(acp.code, -32001 | -32002) {
        return false;
    }
    if is_acp_auth_required_payload(Some(&acp)) {
        return false;
    }
    if matches!(acp.code, -32601 | -32602) {
        return false;
    }

    matches!(acp.code, -32603 | -32700)
}

pub fn exit_code_for_output_error_code(code: OutputErrorCode) -> ExitCode {
    match code {
        OutputErrorCode::Usage => EXIT_CODE_USAGE,
        OutputErrorCode::Timeout => EXIT_CODE_TIMEOUT,
        OutputErrorCode::NoSession => EXIT_CODE_NO_SESSION,
        OutputErrorCode::PermissionDenied | OutputErrorCode::PermissionPromptUnavailable => {
            EXIT_CODE_PERMISSION_DENIED
        }
        OutputErrorCode::Runtime => EXIT_CODE_ERROR,
    }
}

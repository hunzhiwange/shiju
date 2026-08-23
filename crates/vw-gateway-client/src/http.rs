//! 网关 HTTP 辅助函数。
//!
//! 本模块聚焦于可复用的底层能力：
//! - 认证参数注入
//! - 查询参数规范化
//! - 成功/失败响应日志
//! - 统一错误文本拼装
//!
//! 业务 API 应尽量通过这些辅助函数保持日志格式与错误表现一致。

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing::{debug, error, warn};
use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER};

use crate::endpoint::GatewayEndpoint;

const SANITIZED_PAYLOAD_MAX_CHARS: usize = 512;
const SANITIZED_FIELD_MAX_CHARS: usize = 128;
const REDACTED_VALUE: &str = "[REDACTED]";

/// 为异步请求构造器附加网关认证与租户上下文头。
///
/// 当认证字段为空或仅包含空白字符时，会自动跳过对应认证项。
pub fn apply_request_context(
    builder: reqwest::RequestBuilder,
    endpoint: &GatewayEndpoint,
) -> reqwest::RequestBuilder {
    let builder = builder
        .header(PLATFORM_ID_HEADER, endpoint.tenant_scope().platform_id().as_ref())
        .header(COMPANY_ID_HEADER, endpoint.tenant_scope().company_id().as_ref());
    let Some(auth) = endpoint.auth.as_ref() else {
        return builder;
    };

    if let Some(skey) = auth.skey.as_deref().filter(|value| !value.trim().is_empty()) {
        builder.bearer_auth(skey)
    } else {
        builder
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// 为阻塞请求构造器附加网关认证与租户上下文头。
///
/// 行为与异步版本保持一致，只是作用于阻塞客户端。
pub fn apply_blocking_request_context(
    builder: reqwest::blocking::RequestBuilder,
    endpoint: &GatewayEndpoint,
) -> reqwest::blocking::RequestBuilder {
    let builder = builder
        .header(PLATFORM_ID_HEADER, endpoint.tenant_scope().platform_id().as_ref())
        .header(COMPANY_ID_HEADER, endpoint.tenant_scope().company_id().as_ref());
    let Some(auth) = endpoint.auth.as_ref() else {
        return builder;
    };

    if let Some(skey) = auth.skey.as_deref().filter(|value| !value.trim().is_empty()) {
        builder.bearer_auth(skey)
    } else {
        builder
    }
}

#[allow(dead_code)]
/// 非流式请求默认超时时间，单位为秒。
pub const REQUEST_TIMEOUT_SECS: u64 = 30;

/// 将可选目录参数转换为网关接口约定的查询参数。
///
/// 只有在目录非空时才会附加 `directory` 查询参数，避免把空字符串传给后端。
pub fn directory_query(directory: Option<&str>) -> Vec<(String, String)> {
    directory
        .filter(|value| !value.trim().is_empty())
        .map(|value| vec![("directory".to_string(), value.to_string())])
        .unwrap_or_default()
}

/// 解析 JSON 响应，并统一记录成功或解码失败日志。
///
/// 该函数假设调用方已经完成请求发送，仅负责对响应状态与 JSON 反序列化阶段做统一处理。
pub async fn parse_json_response<T: DeserializeOwned>(
    method: &str,
    endpoint: &GatewayEndpoint,
    path: &str,
    response: reqwest::Response,
) -> Result<T, String> {
    if !response.status().is_success() {
        return Err(response_error(method, endpoint, path, response).await);
    }
    log_request_succeeded(method, endpoint, path);
    response.json().await.map_err(|err| {
        let msg = err.to_string();
        error!(
            target: "vw_gateway_client",
            method = method,
            endpoint = %endpoint.describe(),
            path = path,
            error = %msg,
            "gateway response JSON decode failed"
        );
        msg
    })
}

/// 记录成功响应；成功请求频率较高，保持在 DEBUG 避免淹没 INFO 日志。
pub fn log_request_succeeded(method: &str, endpoint: &GatewayEndpoint, path: &str) {
    debug!(
        target: "vw_gateway_client",
        method = method,
        endpoint = %endpoint.describe(),
        path = path,
        "gateway request succeeded"
    );
}

/// 将非成功响应转换为统一错误文本，便于上层直接展示。
///
/// 返回值不会携带结构化错误类型，目的是让上层无需额外转换即可直接显示给用户或记录到日志。
pub async fn response_error(
    method: &str,
    endpoint: &GatewayEndpoint,
    path: &str,
    response: reqwest::Response,
) -> String {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let sanitized_body = sanitize_payload(&body);
    warn!(
        target: "vw_gateway_client",
        method = method,
        endpoint = %endpoint.describe(),
        path = path,
        status = %status,
        response_body = %sanitized_body,
        "gateway request returned non-success status"
    );
    if sanitized_body.is_empty() {
        format!("gateway request failed: {status}")
    } else {
        format!("gateway request failed: {status} {sanitized_body}")
    }
}

/// 记录请求基础信息，便于排查网关交互问题。
///
/// 日志只包含方法、端点、路径及查询/请求体是否存在，不序列化业务载荷。
pub fn log_request<B: Serialize>(
    method: &str,
    endpoint: &GatewayEndpoint,
    path: &str,
    query: &[(String, String)],
    body: Option<&B>,
) {
    debug!(
        target: "vw_gateway_client",
        method = method,
        endpoint = %endpoint.describe(),
        path = path,
        query_parameter_count = query.len(),
        body_present = body.is_some(),
        "sending gateway request"
    );
}

/// 将非成功响应转换为统一错误文本（阻塞版本）。
#[cfg(not(target_arch = "wasm32"))]
pub fn response_error_blocking(
    method: &str,
    endpoint: &GatewayEndpoint,
    path: &str,
    response: reqwest::blocking::Response,
) -> String {
    let status = response.status();
    let body = response.text().unwrap_or_default();
    let sanitized_body = sanitize_payload(&body);
    warn!(
        target: "vw_gateway_client",
        method = method,
        endpoint = %endpoint.describe(),
        path = path,
        status = %status,
        response_body = %sanitized_body,
        "gateway request returned non-success status"
    );
    if sanitized_body.is_empty() {
        format!("gateway request failed: {status}")
    } else {
        format!("gateway request failed: {status} {sanitized_body}")
    }
}

/// 将传输层错误转换为字符串，并补充统一日志。
///
/// 这里的“传输层错误”包括连接失败、超时、TLS/协议错误等 `reqwest` 发送阶段异常。
pub fn transport_error(
    method: &str,
    endpoint: &GatewayEndpoint,
    path: &str,
    err: reqwest::Error,
) -> String {
    let msg = err.to_string();
    #[cfg(not(target_arch = "wasm32"))]
    let is_unavailable = err.is_connect() || err.is_timeout() || is_gateway_unavailable_error(&msg);
    #[cfg(target_arch = "wasm32")]
    let is_unavailable = err.is_timeout() || is_gateway_unavailable_error(&msg);
    if is_unavailable {
        debug!(
            target: "vw_gateway_client",
            method = method,
            endpoint = %endpoint.describe(),
            path = path,
            error = %msg,
            "gateway transport unavailable"
        );
    } else {
        warn!(
            target: "vw_gateway_client",
            method = method,
            endpoint = %endpoint.describe(),
            path = path,
            error = %msg,
            "gateway transport error"
        );
    }
    msg
}

/// 判断错误文本是否表示网关当前不可达，适用于有本地/默认回退的读取路径。
pub fn is_gateway_unavailable_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    [
        "timed out",
        "timeout",
        "connection refused",
        "connection reset",
        "connection closed",
        "connection aborted",
        "broken pipe",
        "error sending request",
        "error trying to connect",
        "tcp connect error",
        "dns error",
        "operation was canceled",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

/// 判断网关错误文本是否适合做一次短重试。
///
/// 这里只覆盖传输层抖动与明确的临时 HTTP 状态，调用方仍应只在幂等接口上使用。
pub(crate) fn is_retryable_gateway_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    let retryable_status = [
        "gateway request failed: 408",
        "gateway request failed: 429",
        "gateway request failed: 500",
        "gateway request failed: 502",
        "gateway request failed: 503",
        "gateway request failed: 504",
    ];
    if retryable_status.iter().any(|needle| lower.contains(needle)) {
        return true;
    }

    is_gateway_unavailable_error(message)
}

fn sanitize_payload(payload: &str) -> String {
    let payload = payload.trim();
    if payload.is_empty() {
        return String::new();
    }

    let sanitized = match serde_json::from_str::<Value>(payload) {
        Ok(mut value) => {
            sanitize_json_value(&mut value);
            value.to_string()
        }
        Err(_) if contains_sensitive_marker(payload) => REDACTED_VALUE.to_string(),
        Err(_) => payload.to_string(),
    };
    truncate_payload(sanitized)
}

fn sanitize_json_value(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                if is_sensitive_key(key) {
                    *value = Value::String(REDACTED_VALUE.to_string());
                } else {
                    sanitize_json_value(value);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                sanitize_json_value(value);
            }
        }
        Value::String(text) => {
            *text = if contains_sensitive_marker(text) {
                REDACTED_VALUE.to_string()
            } else {
                truncate_text(text, SANITIZED_FIELD_MAX_CHARS)
            };
        }
        _ => {}
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    [
        "skey",
        "token",
        "password",
        "credential",
        "apikey",
        "secret",
        "authorization",
    ]
    .iter()
    .any(|sensitive| normalized.contains(sensitive))
}

fn contains_sensitive_marker(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "skey",
        "token",
        "password",
        "credential",
        "api_key",
        "api-key",
        "apikey",
        "secret",
        "authorization",
        "bearer ",
        "sk-",
        "ghp_",
        "xoxb-",
    ]
    .iter()
    .any(|sensitive| lower.contains(sensitive))
}

fn truncate_payload(payload: String) -> String {
    truncate_text(&payload, SANITIZED_PAYLOAD_MAX_CHARS)
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let Some((end, _)) = text.char_indices().nth(max_chars) else {
        return text.to_string();
    };
    format!("{}…", &text[..end])
}

#[cfg(test)]
#[path = "http_tests.rs"]
mod http_tests;

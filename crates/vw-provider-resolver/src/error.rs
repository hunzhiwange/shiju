//! Provider 调用错误的标准化解析。
//!
//! 不同 provider 返回的错误结构与文案并不一致，本模块负责将这些原始错误
//! 转换成更稳定、可供上层展示和重试决策使用的统一结构。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 原始 API 调用错误信息。
///
/// 该结构保留上游响应中的主要字段，供后续做上下文溢出识别、重试判断
/// 与 UI 友好化提示转换。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCallError {
    pub message: String,
    #[serde(default)]
    pub status_code: Option<u16>,
    #[serde(default)]
    pub is_retryable: bool,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub response_headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub response_body: Option<String>,
}

/// 流式响应中的已解析错误。
///
/// 主要用于解析 SSE 或分块流中携带的结构化错误事件。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ParsedStreamError {
    ContextOverflow { message: String, response_body: String },
    ApiError { message: String, is_retryable: bool, response_body: String },
}

/// 统一后的 API 调用错误结构。
///
/// 上层通常只依赖这个枚举，而不是直接处理 provider 原始错误。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ParsedApiCallError {
    ContextOverflow {
        message: String,
        response_body: Option<String>,
    },
    ApiError {
        message: String,
        #[serde(default)]
        status_code: Option<u16>,
        is_retryable: bool,
        #[serde(default)]
        response_headers: Option<HashMap<String, String>>,
        #[serde(default)]
        response_body: Option<String>,
        #[serde(default)]
        metadata: Option<HashMap<String, String>>,
    },
}

/// 判断是否属于上下文窗口超限的常见错误文案。
fn overflow_patterns() -> &'static [&'static str] {
    &[
        "prompt is too long",
        "input is too long for requested model",
        "exceeds the context window",
        "input token count",
        "maximum prompt length is",
        "reduce the length of the messages",
        "maximum context length is",
        "exceeds the available context size",
        "greater than the context length",
        "context window exceeds limit",
        "exceeded model token limit",
        "context_length_exceeded",
    ]
}

/// 根据错误消息粗略识别上下文溢出场景。
///
/// 这里使用启发式匹配，而不是依赖单一 provider 的固定错误码。
fn is_overflow(message: &str) -> bool {
    let msg = message.to_ascii_lowercase();
    if overflow_patterns().iter().any(|p| msg.contains(p)) {
        return true;
    }
    if msg.starts_with("400") && msg.contains("(no body)") {
        return true;
    }
    msg.starts_with("413") && msg.contains("(no body)")
}

/// OpenAI 404 也可能是短暂错误，因此允许按可重试处理。
fn is_openai_error_retryable(e: &ApiCallError) -> bool {
    let Some(status) = e.status_code else {
        return e.is_retryable;
    };
    status == 404 || e.is_retryable
}

fn is_alibaba_market_activation_error(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("aliyun market app does not exist")
        || normalized.contains("may not have activated the service")
}

/// 针对特定 provider 将原始错误转换为更可操作的提示。
///
/// 例如对 GitHub Copilot 的 403 场景，会显式提示用户重新认证。
fn transform_message(provider_id: &str, message: &str, status_code: Option<u16>) -> String {
    if provider_id.contains("github-copilot") && status_code == Some(403) {
        return "请重新对 copilot provider 进行认证，以确保凭据可用。".to_string();
    }
    if provider_id == "alibaba-cn" && is_alibaba_market_activation_error(message) {
        return "当前阿里云账号未开通该云市场模型。请先在阿里云侧激活对应的 siliconflow 市场应用，或改用 siliconflow-cn provider 下的对应模型。".to_string();
    }
    message.to_string()
}

/// 尝试将输入解析为 JSON 对象。
fn json(input: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(input)
        .ok()
        .filter(|v| v.is_object())
}

/// 从流式错误响应体中提取标准化错误。
///
/// # 参数
///
/// * `input` - 原始流式错误片段
///
/// # 返回值
///
/// 若能识别为支持的错误结构，则返回标准化结果；否则返回 `None`
pub fn parse_stream_error(input: &str) -> Option<ParsedStreamError> {
    let body = json(input)?;
    if body.get("type")?.as_str()? != "error" {
        return None;
    }
    let response_body = serde_json::to_string(&body).ok()?;
    let code = body.get("error").and_then(|e| e.get("code")).and_then(|v| v.as_str());
    match code {
        Some("context_length_exceeded") => Some(ParsedStreamError::ContextOverflow {
            message: "输入超出该模型的上下文窗口".to_string(),
            response_body,
        }),
        Some("insufficient_quota") => Some(ParsedStreamError::ApiError {
            message: "配额已用尽，请检查套餐与计费信息。".to_string(),
            is_retryable: false,
            response_body,
        }),
        Some("usage_not_included") => Some(ParsedStreamError::ApiError {
            message:
                "要使用 Codex 与 ChatGPT 套餐，请升级到 Plus：https://chatgpt.com/explore/plus。"
                    .to_string(),
            is_retryable: false,
            response_body,
        }),
        Some("invalid_prompt") => Some(ParsedStreamError::ApiError {
            message: body
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("无效的 prompt。")
                .to_string(),
            is_retryable: false,
            response_body,
        }),
        _ => None,
    }
}

/// 将原始 API 调用错误转换为统一错误结构。
///
/// # 参数
///
/// * `provider_id` - 错误所属的 provider 标识
/// * `error` - 原始错误结构
pub fn parse_api_call_error(provider_id: &str, error: ApiCallError) -> ParsedApiCallError {
    let msg0 = if error.message.trim().is_empty() {
        error
            .response_body
            .clone()
            .or_else(|| error.status_code.map(|s| s.to_string()))
            .unwrap_or_else(|| "Unknown error".to_string())
    } else {
        error.message.clone()
    };
    let msg = transform_message(provider_id, &msg0, error.status_code);
    if is_overflow(&msg) {
        return ParsedApiCallError::ContextOverflow {
            message: msg,
            response_body: error.response_body,
        };
    }
    let metadata = error.url.clone().map(|u| HashMap::from([("url".to_string(), u)]));
    ParsedApiCallError::ApiError {
        message: msg,
        status_code: error.status_code,
        is_retryable: if provider_id.starts_with("openai") {
            is_openai_error_retryable(&error)
        } else {
            error.is_retryable
        },
        response_headers: error.response_headers,
        response_body: error.response_body,
        metadata,
    }
}

//! AISDK 错误转换模块
//!
//! 本模块提供将 `aisdk` 库的错误类型转换为 VibeWindow 内部使用的
//! `AssistantError` 类型的转换函数。该转换允许统一的错误处理和传递，
//! 无论底层使用哪个 AI SDK 实现。
//!
//! # 平台兼容性
//!
//! 由于 `aisdk` 库不支持 WebAssembly 目标平台，本模块的所有功能
//! 都通过 `#[cfg(not(target_arch = "wasm32"))]` 条件编译保护，
//! 仅在非 WASM 环境下可用。

#[cfg(not(target_arch = "wasm32"))]
use aisdk::Error as AiError;
#[cfg(not(target_arch = "wasm32"))]
use serde_json::Value;

#[cfg(not(target_arch = "wasm32"))]
fn aisdk_error_source_chain(e: &AiError) -> Option<String> {
    let mut parts = Vec::new();
    let mut current = std::error::Error::source(e);

    while let Some(source) = current {
        let text = source.to_string();
        if !text.trim().is_empty() {
            parts.push(text);
        }
        current = std::error::Error::source(source);
    }

    if parts.is_empty() { None } else { Some(parts.join(" | caused_by: ")) }
}

#[cfg(not(target_arch = "wasm32"))]
fn is_retryable_body_decode_error(
    details: &str,
    raw_error: &str,
    source_chain: Option<&str>,
) -> bool {
    let mut text = format!("{details}\n{raw_error}");
    if let Some(chain) = source_chain {
        text.push('\n');
        text.push_str(chain);
    }
    let normalized = text.to_ascii_lowercase();
    normalized.contains("error decoding response body")
        || normalized.contains("eof while parsing a value")
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn aisdk_assistant_error_log_fields(
    assistant_error: &crate::app::agent::session::message::AssistantError,
) -> serde_json::Map<String, Value> {
    let mut fields = serde_json::Map::new();

    match assistant_error {
        crate::app::agent::session::message::AssistantError::APIError {
            message,
            status_code,
            response_body,
            metadata,
            ..
        } => {
            fields.insert("error".to_string(), Value::String(message.clone()));
            if let Some(code) = status_code {
                fields.insert("statusCode".to_string(), Value::from(*code));
            }
            if let Some(body) = response_body.as_ref() {
                fields.insert("responseBody".to_string(), Value::String(body.clone()));
            }
            if let Some(metadata) = metadata.as_ref() {
                let meta_obj = metadata
                    .iter()
                    .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                    .collect::<serde_json::Map<String, Value>>();
                fields.insert("metadata".to_string(), Value::Object(meta_obj));
            }
        }
        other => {
            fields.insert(
                "error".to_string(),
                Value::String(serde_json::to_string(other).unwrap_or_else(|_| "?".to_string())),
            );
        }
    }

    fields
}

/// 将 AISDK 错误转换为内部 AssistantError 类型
///
/// 该函数负责将 `aisdk` 库产生的错误映射到 VibeWindow 内部的
/// `AssistantError` 枚举变体，保留关键错误信息并推断可重试性。
///
/// # 参数
///
/// * `provider_id` - 产生错误的 AI 提供商标识符（如 "openai"、"anthropic"）
/// * `e` - 来自 aisdk 库的原始错误实例
///
/// # 返回值
///
/// 返回转换后的 `AssistantError` 枚举变体：
///
/// | AISDK 错误类型 | 转换后的 AssistantError | 说明 |
/// |---------------|------------------------|------|
/// | `MissingField` | `ProviderAuthError` | 缺少必要字段，通常表示认证配置问题 |
/// | `ApiError` | `APIError` | API 调用错误，包含可重试性判断 |
/// | 其他 | `Unknown` | 未知错误，保留原始错误消息 |
///
/// # 可重试性判断逻辑
///
/// 对于 `ApiError` 类型，当 HTTP 状态码满足以下条件之一时，
/// 错误被标记为可重试（`is_retryable = true`）：
/// - 状态码为 429（请求过多/速率限制）
/// - 状态码为 5xx（服务器错误）
///
/// # 示例
///
/// ```ignore
/// use aisdk::Error as AiError;
/// let aisdk_err = AiError::ApiError {
///     details: "Rate limit exceeded".to_string(),
///     status_code: Some(StatusCode::TOO_MANY_REQUESTS),
/// };
/// let assistant_err = assistant_error_from_aisdk("openai", aisdk_err);
/// // assistant_err 现在是 AssistantError::APIError，且 is_retryable = true
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn assistant_error_from_aisdk(
    provider_id: &str,
    e: AiError,
) -> crate::app::agent::session::message::AssistantError {
    let raw_error = e.to_string();
    let source_chain = aisdk_error_source_chain(&e);

    match e {
        // 缺少必要字段错误 -> 转换为提供者认证错误
        // 这通常表示 API 密钥或配置缺失
        AiError::MissingField(field) => {
            crate::app::agent::session::message::AssistantError::ProviderAuthError {
                provider_id: provider_id.to_string(),
                message: field,
            }
        }
        // API 调用错误 -> 转换为通用 API 错误
        // 包含详细的错误信息和可重试性判断
        AiError::ApiError { details, status_code } => {
            // 判断错误是否可重试：
            // - 429: 速率限制，稍后重试可能成功
            // - 5xx: 服务器端临时故障，通常可重试
            let retryable = status_code.is_some_and(|s| s.as_u16() == 429 || s.is_server_error())
                || is_retryable_body_decode_error(&details, &raw_error, source_chain.as_deref());
            let mut metadata = std::collections::HashMap::from([
                ("source".to_string(), "aisdk".to_string()),
                ("raw_error".to_string(), raw_error.clone()),
            ]);
            if let Some(chain) = source_chain.as_ref() {
                metadata.insert("error_chain".to_string(), chain.clone());
            }

            crate::app::agent::session::message::AssistantError::APIError {
                message: details,
                // 将 HTTP 状态码转换为 i64 以保持跨语言兼容性
                status_code: status_code.map(|s| s.as_u16() as i64),
                is_retryable: retryable,
                // AISDK 不直接暴露响应头；保留为 None。
                response_headers: None,
                response_body: source_chain.or_else(|| Some(raw_error)),
                metadata: Some(metadata),
            }
        }
        // 其他未分类错误 -> 转换为未知错误
        // 保留原始错误的字符串表示以便调试
        other => crate::app::agent::session::message::AssistantError::Unknown {
            message: other.to_string(),
        },
    }
}
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "error_tests.rs"]
mod error_tests;

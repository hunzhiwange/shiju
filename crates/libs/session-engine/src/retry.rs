//! # 重试策略模块
//!
//! 本模块提供会话层级的重试策略实现，用于处理 API 调用失败时的自动重试逻辑。
//!
//! ## 主要功能
//!
//! - **指数退避延迟计算**：基于重试次数和错误类型计算合适的等待时间
//! - **可重试性判断**：分析错误类型确定是否应该重试请求
//! - **HTTP 头解析**：从响应头中提取服务端建议的重试延迟
//!
//! ## 重试策略
//!
//! 1. 优先使用服务端提供的重试延迟（`retry-after-ms` 或 `retry-after` 头）
//! 2. 无服务端建议时使用指数退避算法
//! 3. 对某些错误类型（如配额超限、过载等）提供用户友好的错误信息

use message_types as message;
use serde_json::Value;
#[cfg(test)]
use crate::retry_delay::{pow_u64, header_value_case_insensitive};
use std::time::Duration;

pub use crate::retry_delay::{
    RETRY_INITIAL_DELAY, RETRY_BACKOFF_FACTOR, RETRY_MAX_DELAY_NO_HEADERS, RETRY_MAX_DELAY, delay,
};

/// 异步睡眠指定毫秒数
///
/// 使用 tokio 异步运行时的睡眠功能，并确保延迟不超过绝对最大值。
///
/// # 参数
///
/// - `ms`: 睡眠时长（毫秒），将被限制在 `RETRY_MAX_DELAY` 以内
///
/// # 示例
///
/// ```ignore
/// sleep(5000).await; // 睡眠 5 秒
/// sleep(u64::MAX).await; // 将被限制为 RETRY_MAX_DELAY
/// ```
pub async fn sleep(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms.min(RETRY_MAX_DELAY))).await;
}

// Retry delay implementation is owned by session-engine.

/// 解析 JSON 格式的错误消息
///
/// 尝试将字符串解析为 JSON 值，用于从错误消息中提取结构化信息。
///
/// # 参数
///
/// - `message`: 待解析的消息字符串
///
/// # 返回值
///
/// 解析成功返回 `Some(Value)`，否则返回 `None`
fn parse_json_message(message: &str) -> Option<Value> {
    let msg = message.trim();
    if msg.is_empty() {
        return None;
    }
    serde_json::from_str::<Value>(msg).ok()
}

/// 从 JSON 对象中安全地获取字符串字段
///
/// # 参数
///
/// - `v`: JSON 值引用
/// - `key`: 字段名称
///
/// # 返回值
///
/// 如果字段存在且为字符串类型，返回字符串切片引用；否则返回 `None`
fn json_get_str<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

/// 判断错误是否可重试并返回重试原因
///
/// 分析错误类型和内容，确定是否应该进行重试。
/// 对于可重试的错误，返回人类可读的原因描述。
///
/// ## 错误类型处理
///
/// ### APIError
/// - 如果 `is_retryable` 标志为 false，不重试
/// - 检测免费配额超限错误，提供充值链接
/// - 检测服务过载错误
/// - 返回原始错误消息
///
/// ### Unknown / MessageAbortedError / ProviderAuthError
/// - 尝试解析 JSON 格式的错误消息
/// - 检测 `too_many_requests` 错误（429 状态码）
/// - 检测资源耗尽或服务不可用
/// - 检测速率限制错误
///
/// ### ContextOverflowError
/// - 上下文长度超限不可重试，返回 `None`
///
/// # 参数
///
/// - `error`: 错误引用
///
/// # 返回值
///
/// - `Some(String)`: 可重试，返回重试原因的描述
/// - `None`: 不可重试
///
/// # 示例
///
/// ```ignore
/// let error = AssistantError::APIError {
///     message: "Overloaded".to_string(),
///     is_retryable: true,
///     ...
/// };
///
/// if let Some(reason) = retryable(&error) {
///     println!("将重试，原因: {}", reason); // "Provider is overloaded"
/// }
/// ```
pub fn retryable(error: &message::AssistantError) -> Option<String> {
    match error {
        // 上下文超限错误不可重试，需要用户调整输入
        message::AssistantError::ContextOverflowError { .. } => None,

        // API 错误：检查是否标记为可重试
        message::AssistantError::APIError { message, is_retryable, response_body: _, .. } => {
            // 如果明确标记为不可重试，直接返回
            if !*is_retryable {
                return None;
            }

            // 检测服务过载错误
            if message.contains("Overloaded") {
                return Some("Provider is overloaded".to_string());
            }

            // 默认返回原始错误消息
            Some(message.clone())
        }

        // 未知错误、消息中止错误、Provider 认证错误：尝试解析 JSON 消息
        message::AssistantError::Unknown { message }
        | message::AssistantError::MessageAbortedError { message }
        | message::AssistantError::ProviderAuthError { message, .. } => {
            // 尝试解析 JSON 格式的错误消息
            let Some(json) = parse_json_message(message) else {
                return None;
            };

            // 确保解析结果是 JSON 对象
            if !json.is_object() {
                return None;
            }

            // 提取错误代码字段
            let code = json_get_str(&json, "code").unwrap_or_default();

            // 检测 too_many_requests 错误（通常是 429 状态码）
            // 格式：{ "type": "error", "error": { "type": "too_many_requests" } }
            if json_get_str(&json, "type") == Some("error")
                && json.get("error").and_then(|e| e.get("type")).and_then(Value::as_str)
                    == Some("too_many_requests")
            {
                return Some("Too Many Requests".to_string());
            }

            // 检测资源耗尽或服务不可用（基于错误代码）
            let code_lower = code.to_ascii_lowercase();
            if code_lower.contains("exhausted") || code_lower.contains("unavailable") {
                return Some("Provider is overloaded".to_string());
            }

            // 检测速率限制错误
            // 格式：{ "type": "error", "error": { "code": "rate_limit" } }
            if json_get_str(&json, "type") == Some("error")
                && json
                    .get("error")
                    .and_then(|e| e.get("code"))
                    .and_then(Value::as_str)
                    .is_some_and(|c| c.contains("rate_limit"))
            {
                return Some("Rate Limited".to_string());
            }

            // 返回完整的 JSON 错误信息
            serde_json::to_string(&json).ok()
        }

        // 其他错误类型默认不可重试
        _ => None,
    }
}
#[cfg(test)]
#[path = "retry_tests.rs"]
mod retry_tests;

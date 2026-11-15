//! Provider-directed and bounded local retry delay calculation.
use message_types as message;

/// 初始重试延迟（毫秒）
///
/// 当无法从响应头获取重试建议时，首次重试将等待此时长。
pub const RETRY_INITIAL_DELAY: u64 = 2000;

/// 指数退避因子
///
/// 每次重试的延迟将乘以此因子，实现指数退避效果。
/// 例如：2s -> 4s -> 8s -> 16s ...
pub const RETRY_BACKOFF_FACTOR: u64 = 2;

/// 无响应头时的最大重试延迟（毫秒）
///
/// 当无法从响应头获取重试建议时，延迟上限为 30 秒，
/// 避免在没有明确服务端指示的情况下等待过久。
pub const RETRY_MAX_DELAY_NO_HEADERS: u64 = 30_000;

/// 绝对最大重试延迟（毫秒）
///
/// 约等于 24.8 天，作为 sleep 函数的硬性上限，
/// 防止因计算错误导致无限等待。
pub const RETRY_MAX_DELAY: u64 = 2_147_483_647;

/// 计算 u64 类型的幂运算
///
/// 使用饱和乘法防止溢出，当结果超过 u64 最大值时返回 `u64::MAX`。
///
/// # 参数
///
/// - `base`: 底数
/// - `exp`: 指数
///
/// # 返回值
///
/// 返回 `base^exp` 的结果，溢出时返回 `u64::MAX`
#[doc(hidden)]
pub fn pow_u64(base: u64, exp: u64) -> u64 {
    let mut out = 1u64;
    for _ in 0..exp {
        out = out.saturating_mul(base);
    }
    out
}

/// 大小写不敏感地从 HTTP 头中获取值
///
/// HTTP 头名称规范上是不区分大小写的，此函数提供了大小写不敏感的查找能力。
///
/// # 参数
///
/// - `headers`: HTTP 头的键值对集合
/// - `key`: 要查找的头名称（查找时忽略大小写）
///
/// # 返回值
///
/// 找到时返回对应值的引用，否则返回 `None`
#[doc(hidden)]
pub fn header_value_case_insensitive<'a>(
    headers: &'a std::collections::HashMap<String, String>,
    key: &str,
) -> Option<&'a str> {
    let key_lower = key.to_ascii_lowercase();
    headers.iter().find(|(k, _)| k.to_ascii_lowercase() == key_lower).map(|(_, v)| v.as_str())
}

/// 计算重试延迟时间
///
/// 根据重试次数和错误信息计算应该等待的时间（毫秒）。
///
/// ## 延迟计算优先级
///
/// 1. 如果错误包含响应头，尝试读取 `retry-after-ms` 头（毫秒）
/// 2. 尝试读取 `retry-after` 头（秒，转换为毫秒）
/// 3. 使用指数退避算法：`INITIAL_DELAY * BACKOFF_FACTOR^(attempt-1)`
/// 4. 如果没有响应头，延迟上限为 `RETRY_MAX_DELAY_NO_HEADERS`
///
/// # 参数
///
/// - `attempt`: 当前重试次数（从 1 开始，小于 1 会被修正为 1）
/// - `error`: 可选的错误信息，用于提取服务端建议的延迟
///
/// # 返回值
///
/// 返回应该等待的毫秒数
///
/// # 示例
///
/// ```ignore
/// // 首次重试，无错误信息
/// let d = delay(1, None); // 返回 2000ms
///
/// // 第三次重试，无错误信息
/// let d = delay(3, None); // 返回 8000ms (2^3 * 2000)
///
/// // 从响应头获取延迟建议
/// let error = AssistantError::APIError { ... };
/// let d = delay(1, Some(&error)); // 可能返回服务端建议的延迟
/// ```
pub fn delay(attempt: u64, error: Option<&message::AssistantError>) -> u64 {
    // 确保重试次数至少为 1
    let attempt = attempt.max(1);

    // 尝试从 API 错误的响应头中获取重试延迟
    if let Some(message::AssistantError::APIError { response_headers, .. }) = error {
        if let Some(headers) = response_headers {
            // 优先检查 retry-after-ms 头（毫秒单位）
            if let Some(v) = header_value_case_insensitive(headers, "retry-after-ms") {
                if let Ok(parsed) = v.trim().parse::<f64>() {
                    if parsed.is_finite() && parsed >= 0.0 {
                        return parsed.ceil() as u64;
                    }
                }
            }

            // 其次检查 retry-after 头（秒单位，需转换为毫秒）
            if let Some(v) = header_value_case_insensitive(headers, "retry-after") {
                if let Ok(parsed) = v.trim().parse::<f64>() {
                    if parsed.is_finite() && parsed >= 0.0 {
                        return (parsed * 1000.0).ceil() as u64;
                    }
                }
            }

            // 有响应头但无重试建议时，使用指数退避（无上限）
            return RETRY_INITIAL_DELAY.saturating_mul(pow_u64(RETRY_BACKOFF_FACTOR, attempt - 1));
        }
    }

    // 无响应头或非 API 错误时，使用指数退避并限制最大延迟
    let computed = RETRY_INITIAL_DELAY.saturating_mul(pow_u64(RETRY_BACKOFF_FACTOR, attempt - 1));
    computed.min(RETRY_MAX_DELAY_NO_HEADERS)
}

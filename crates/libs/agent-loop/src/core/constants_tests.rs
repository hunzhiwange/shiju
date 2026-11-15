//! 测试代理循环常量维持安全的运行边界与重试提示契约。
//! 断言锁定的是防止无限循环和过短等待的保护值，而非任意实现细节。

use super::*;

#[test]
fn constants_keep_safe_runtime_bounds() {
    assert!(STREAM_CHUNK_MIN_CHARS > 0);
    assert!(DEFAULT_MAX_ITERATIONS > 0);
    assert_eq!(DEFAULT_MAX_ITERATIONS, 50_000);
    assert_eq!(DEFAULT_MAX_HISTORY_MESSAGES, 100);
    assert!(MIN_CHANNEL_MESSAGE_TIMEOUT_SECS >= 30);
    assert!(CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP >= 1);
    assert!(MISSING_TOOL_CALL_RETRY_PROMPT.contains("valid tool call"));
    assert!(EMPTY_FINAL_RESPONSE_RETRY_PROMPT.contains("non-empty final answer"));
}

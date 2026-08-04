use super::*;

#[test]
fn constants_keep_safe_runtime_bounds() {
    assert!(STREAM_CHUNK_MIN_CHARS > 0);
    assert!(DEFAULT_MAX_TOOL_ITERATIONS > 0);
    assert_eq!(DEFAULT_MAX_TOOL_ITERATIONS, 50_000);
    assert_eq!(DEFAULT_MAX_HISTORY_MESSAGES, 100);
    assert!(MIN_CHANNEL_MESSAGE_TIMEOUT_SECS >= 30);
    assert!(CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP >= 1);
    assert!(MISSING_TOOL_CALL_RETRY_PROMPT.contains("valid tool call"));
    assert!(EMPTY_FINAL_RESPONSE_RETRY_PROMPT.contains("non-empty final answer"));
}

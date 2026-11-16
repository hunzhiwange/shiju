//! Retry classification and provider backoff contracts migrated from the host suite.
use super::{is_acp_retryable_error, provider_retry_backoff_ms};
use super::retry_allowed as llm_step_retry_allowed;

#[test]
fn acp_retryable_error_matches_timeout_and_disconnect_messages() {
    assert!(is_acp_retryable_error("模型响应超时"));
    assert!(is_acp_retryable_error("request TIMED OUT"));
    assert!(is_acp_retryable_error("acp agent disconnected during request"));
    assert!(is_acp_retryable_error("queue owner disconnected before prompt completion"));
    assert!(is_acp_retryable_error("API error: None - error decoding response body"));
    assert!(!is_acp_retryable_error("invalid api key"));
}

#[test]
fn non_retryable_llm_error_matches_model_resolution_failures() {
    assert!(!llm_step_retry_allowed(
        r#"{"name":"Unknown","message":"未找到模型：llama3"}"#,
        false
    ));
    assert!(!llm_step_retry_allowed("模型格式错误：/llama3，请使用 provider/model", false));
    assert!(!llm_step_retry_allowed("模型ID存在歧义：llama3，请使用 provider/model", false));
    assert!(!llm_step_retry_allowed("model not found", false));
    assert!(llm_step_retry_allowed("模型响应超时", false));
}

#[test]
fn llm_step_retry_allowed_blocks_model_config_errors_only() {
    assert!(!llm_step_retry_allowed(
        r#"{"name":"Unknown","message":"未找到模型：llama3"}"#,
        false,
    ));
    assert!(llm_step_retry_allowed("模型响应超时", false));
    assert!(llm_step_retry_allowed("模型响应超时", true));
    assert!(!llm_step_retry_allowed("invalid api key", true));
}

#[test]
fn acp_retryable_error_matches_timeout_word_and_eof_failure() {
    assert!(is_acp_retryable_error("request timeout"));
    assert!(is_acp_retryable_error("EOF while parsing a value at line 1"));
}

#[test]
fn non_retryable_llm_error_matches_english_model_resolution_failures() {
    assert!(!llm_step_retry_allowed("unknown model: custom/model", false));
    assert!(!llm_step_retry_allowed("unsupported model family", false));
}

#[test]
fn configured_provider_backoff_controls_retry_delay() {
    assert_eq!(provider_retry_backoff_ms(750, 0), 750);
    assert_eq!(provider_retry_backoff_ms(750, 1), 1_500);
    assert_eq!(provider_retry_backoff_ms(750, 2), 3_000);
    assert_eq!(provider_retry_backoff_ms(750, 3), 5_000);
    assert_eq!(provider_retry_backoff_ms(u64::MAX, u32::MAX), 5_000);
}

#[test]
fn llm_step_retry_allowed_respects_structured_api_error_flag() {
    assert!(!llm_step_retry_allowed(
        r#"{"name":"APIError","message":"temporary timeout","is_retryable":false}"#,
        false,
    ));
    assert!(llm_step_retry_allowed(
        r#"{"name":"APIError","message":"temporary timeout","is_retryable":true}"#,
        false,
    ));
}

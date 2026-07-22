use super::{
    RETRY_INITIAL_DELAY, RETRY_MAX_DELAY_NO_HEADERS, delay, header_value_case_insensitive,
    parse_json_message, pow_u64, retryable,
};
use crate::app::agent::session::message::AssistantError;
use std::collections::HashMap;

#[test]
fn pow_u64_saturates_on_overflow() {
    assert_eq!(pow_u64(2, 3), 8);
    assert_eq!(pow_u64(u64::MAX, 2), u64::MAX);
}

#[test]
fn pow_u64_handles_identity_and_zero_base() {
    assert_eq!(pow_u64(u64::MAX, 0), 1);
    assert_eq!(pow_u64(0, 3), 0);
}

#[tokio::test]
async fn sleep_returns_for_zero_delay() {
    super::sleep(0).await;
}

#[test]
fn delay_prefers_retry_after_headers_case_insensitively() {
    let mut headers = HashMap::new();
    headers.insert("Retry-After-MS".to_string(), "12.4".to_string());
    let error = AssistantError::APIError {
        message: "rate limited".to_string(),
        status_code: Some(429),
        response_body: None,
        response_headers: Some(headers),
        metadata: None,
        is_retryable: true,
    };

    assert_eq!(delay(1, Some(&error)), 13);
}

#[test]
fn header_lookup_and_json_parse_reject_empty_values() {
    let mut headers = HashMap::new();
    headers.insert("Retry-After".to_string(), "2".to_string());

    assert_eq!(header_value_case_insensitive(&headers, "retry-after"), Some("2"));
    assert!(parse_json_message("").is_none());
    assert!(parse_json_message("{\"type\":\"error\"}").is_some());
    assert!(parse_json_message("not json").is_none());
}

#[test]
fn delay_uses_retry_after_seconds_and_exponential_fallbacks() {
    let mut headers = HashMap::new();
    headers.insert("retry-after".to_string(), "1.25".to_string());
    let with_seconds = AssistantError::APIError {
        message: "rate limited".to_string(),
        status_code: Some(429),
        response_body: None,
        response_headers: Some(headers),
        metadata: None,
        is_retryable: true,
    };
    assert_eq!(delay(0, Some(&with_seconds)), 1250);

    let invalid_headers = AssistantError::APIError {
        message: "retry".to_string(),
        status_code: Some(503),
        response_body: None,
        response_headers: Some(HashMap::from([
            ("retry-after-ms".to_string(), "-1".to_string()),
            ("retry-after".to_string(), "nan".to_string()),
        ])),
        metadata: None,
        is_retryable: true,
    };
    assert_eq!(delay(3, Some(&invalid_headers)), RETRY_INITIAL_DELAY * 4);

    assert_eq!(delay(3, None), RETRY_INITIAL_DELAY * 4);
    assert_eq!(delay(99, None), RETRY_MAX_DELAY_NO_HEADERS);
}

#[test]
fn delay_falls_back_from_invalid_milliseconds_to_trimmed_seconds() {
    let error = AssistantError::APIError {
        message: "rate limited".to_string(),
        status_code: Some(429),
        response_body: None,
        response_headers: Some(HashMap::from([
            ("retry-after-ms".to_string(), "infinite".to_string()),
            ("retry-after".to_string(), " 0.0011 ".to_string()),
        ])),
        metadata: None,
        is_retryable: true,
    };

    assert_eq!(delay(4, Some(&error)), 2);
}

#[test]
fn delay_only_caps_backoff_when_response_headers_are_absent() {
    let with_empty_headers = AssistantError::APIError {
        message: "temporary".to_string(),
        status_code: Some(503),
        response_body: None,
        response_headers: Some(HashMap::new()),
        metadata: None,
        is_retryable: true,
    };
    let without_headers = AssistantError::APIError {
        message: "temporary".to_string(),
        status_code: Some(503),
        response_body: None,
        response_headers: None,
        metadata: None,
        is_retryable: true,
    };
    let non_api_error = AssistantError::Unknown { message: "temporary".to_string() };

    assert_eq!(delay(6, Some(&with_empty_headers)), RETRY_INITIAL_DELAY * 32);
    assert_eq!(delay(6, Some(&without_headers)), RETRY_MAX_DELAY_NO_HEADERS);
    assert_eq!(delay(6, Some(&non_api_error)), RETRY_MAX_DELAY_NO_HEADERS);
}

#[test]
fn delay_normalizes_zero_attempt_without_server_headers() {
    assert_eq!(delay(0, None), RETRY_INITIAL_DELAY);
}

#[test]
fn retryable_handles_api_error_branches() {
    let non_retryable = AssistantError::APIError {
        message: "bad request".to_string(),
        status_code: Some(400),
        response_body: None,
        response_headers: None,
        metadata: None,
        is_retryable: false,
    };
    assert!(retryable(&non_retryable).is_none());

    let overloaded = AssistantError::APIError {
        message: "Provider Overloaded".to_string(),
        status_code: Some(529),
        response_body: None,
        response_headers: None,
        metadata: None,
        is_retryable: true,
    };
    assert_eq!(retryable(&overloaded).as_deref(), Some("Provider is overloaded"));

    let plain = AssistantError::APIError {
        message: "temporary".to_string(),
        status_code: Some(500),
        response_body: None,
        response_headers: None,
        metadata: None,
        is_retryable: true,
    };
    assert_eq!(retryable(&plain).as_deref(), Some("temporary"));
}

#[test]
fn retryable_parses_structured_json_errors() {
    let too_many = AssistantError::Unknown {
        message: r#"{"type":"error","error":{"type":"too_many_requests"}}"#.to_string(),
    };
    assert_eq!(retryable(&too_many).as_deref(), Some("Too Many Requests"));

    let exhausted = AssistantError::MessageAbortedError {
        message: r#"{"code":"RESOURCE_EXHAUSTED"}"#.to_string(),
    };
    assert_eq!(retryable(&exhausted).as_deref(), Some("Provider is overloaded"));

    let rate = AssistantError::ProviderAuthError {
        provider_id: "p".to_string(),
        message: r#"{"type":"error","error":{"code":"hard_rate_limit"}}"#.to_string(),
    };
    assert_eq!(retryable(&rate).as_deref(), Some("Rate Limited"));

    let other_json = AssistantError::Unknown { message: r#"{"message":"try later"}"#.to_string() };
    assert_eq!(retryable(&other_json).as_deref(), Some(r#"{"message":"try later"}"#));
}

#[test]
fn retryable_matches_unavailable_codes_case_insensitively() {
    let unavailable =
        AssistantError::Unknown { message: r#"{"code":"Service_UNAVAILABLE"}"#.to_string() };

    assert_eq!(retryable(&unavailable).as_deref(), Some("Provider is overloaded"));
}

#[test]
fn retryable_preserves_json_objects_with_non_matching_error_shapes() {
    let unmatched = AssistantError::MessageAbortedError {
        message: r#"{"type":"warning","error":{"type":"too_many_requests","code":"rate-limit"},"code":429}"#
            .to_string(),
    };
    let serialized = retryable(&unmatched).expect("JSON objects remain retryable by default");
    let actual: serde_json::Value =
        serde_json::from_str(&serialized).expect("retry reason should remain valid JSON");

    assert_eq!(
        actual,
        serde_json::json!({
            "type": "warning",
            "error": { "type": "too_many_requests", "code": "rate-limit" },
            "code": 429,
        })
    );
}

#[test]
fn retryable_rejects_non_retryable_error_shapes() {
    assert!(
        retryable(&AssistantError::ContextOverflowError {
            message: "too long".to_string(),
            response_body: None,
        })
        .is_none()
    );
    assert!(retryable(&AssistantError::MessageOutputLengthError).is_none());
    assert!(retryable(&AssistantError::Unknown { message: "plain".to_string() }).is_none());
    assert!(retryable(&AssistantError::Unknown { message: "[]".to_string() }).is_none());
}

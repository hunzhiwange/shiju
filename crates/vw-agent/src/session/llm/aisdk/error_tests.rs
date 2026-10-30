//! 验证 AI SDK 错误链到结构化助手错误的分类与脱敏。

use aisdk::Error as AiError;
use reqwest::StatusCode;
use serde_json::Value;

use crate::app::agent::session::message::AssistantError;

use super::{
    aisdk_assistant_error_log_fields, aisdk_error_source_chain, assistant_error_from_aisdk,
    is_retryable_transport_error,
};

#[test]
fn missing_field_maps_to_provider_auth_error() {
    let error = assistant_error_from_aisdk("openai", AiError::MissingField("api_key".to_string()));

    match error {
        AssistantError::ProviderAuthError { provider_id, message } => {
            assert_eq!(provider_id, "openai");
            assert_eq!(message, "api_key");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn api_error_preserves_status_retryability_body_and_metadata() {
    let error = assistant_error_from_aisdk(
        "openai",
        AiError::ApiError {
            details: "rate limited".to_string(),
            status_code: Some(StatusCode::TOO_MANY_REQUESTS),
        },
    );

    match error {
        AssistantError::APIError {
            message,
            status_code,
            is_retryable,
            response_headers,
            response_body,
            metadata,
        } => {
            assert_eq!(message, "rate limited");
            assert_eq!(status_code, Some(429));
            assert!(is_retryable);
            assert!(response_headers.is_none());
            assert!(response_body.as_deref().unwrap_or_default().contains("rate limited"));

            let metadata = metadata.expect("metadata should be present");
            assert_eq!(metadata.get("source").map(String::as_str), Some("aisdk"));
            assert!(
                metadata
                    .get("raw_error")
                    .map(|value| value.contains("rate limited"))
                    .unwrap_or(false)
            );
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
// 无显式提示时以 HTTP 类别推断重试性，服务端错误与请求错误分开处理。
fn api_error_marks_server_errors_retryable_and_client_errors_not_retryable() {
    let server = assistant_error_from_aisdk(
        "openai",
        AiError::ApiError {
            details: "temporarily unavailable".to_string(),
            status_code: Some(StatusCode::BAD_GATEWAY),
        },
    );
    let client = assistant_error_from_aisdk(
        "openai",
        AiError::ApiError {
            details: "bad request".to_string(),
            status_code: Some(StatusCode::BAD_REQUEST),
        },
    );
    let without_status = assistant_error_from_aisdk(
        "openai",
        AiError::ApiError { details: "unknown".to_string(), status_code: None },
    );

    assert!(matches!(server, AssistantError::APIError { is_retryable: true, .. }));
    assert!(matches!(client, AssistantError::APIError { is_retryable: false, .. }));
    assert!(matches!(without_status, AssistantError::APIError { is_retryable: false, .. }));
}

#[test]
fn api_error_marks_body_decode_failures_retryable_without_status() {
    let error = assistant_error_from_aisdk(
        "openai",
        AiError::ApiError {
            details: "error decoding response body".to_string(),
            status_code: None,
        },
    );

    assert!(matches!(error, AssistantError::APIError { is_retryable: true, .. }));
}

#[test]
fn api_error_marks_request_transport_failures_retryable_without_status() {
    let error = assistant_error_from_aisdk(
        "alibaba-token-plan-cn",
        AiError::ApiError {
            details: "error sending request for url (https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions)".into(),
            status_code: None,
        },
    );

    assert!(matches!(error, AssistantError::APIError { is_retryable: true, .. }));
}

#[test]
fn api_error_client_status_prevents_transport_text_from_enabling_retry() {
    for status in [StatusCode::BAD_REQUEST, StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN] {
        let error = assistant_error_from_aisdk(
            "alibaba-token-plan-cn",
            AiError::ApiError {
                details: "error decoding response body".into(),
                status_code: Some(status),
            },
        );

        assert!(matches!(error, AssistantError::APIError { is_retryable: false, .. }));
    }
}

#[test]
// 响应体解码失败需沿详细信息和错误源链查找暂时性网络原因。
fn transport_retryability_checks_details_raw_error_and_source_chain() {
    assert!(is_retryable_transport_error("ERROR DECODING RESPONSE BODY", "request failed", None,));
    assert!(is_retryable_transport_error("request failed", "EOF while parsing a value", None,));
    assert!(is_retryable_transport_error(
        "request failed",
        "transport closed",
        Some("error decoding response body"),
    ));
    assert!(!is_retryable_transport_error(
        "bad request",
        "invalid input",
        Some("provider rejected request"),
    ));
}

#[test]
fn api_error_without_status_preserves_raw_display_fallback() {
    let error = assistant_error_from_aisdk(
        "custom",
        AiError::ApiError { details: String::new(), status_code: None },
    );

    match error {
        AssistantError::APIError {
            message,
            status_code,
            is_retryable,
            response_body,
            metadata,
            ..
        } => {
            assert!(message.is_empty());
            assert_eq!(status_code, None);
            assert!(!is_retryable);
            assert_eq!(response_body.as_deref(), Some("API error: None - "));

            let metadata = metadata.expect("metadata should be present");
            assert_eq!(metadata.get("source").map(String::as_str), Some("aisdk"));
            assert_eq!(metadata.get("raw_error").map(String::as_str), Some("API error: None - "),);
            assert!(!metadata.contains_key("error_chain"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn unknown_aisdk_error_keeps_display_message() {
    let error =
        assistant_error_from_aisdk("openai", AiError::InvalidInput("bad option".to_string()));

    match error {
        AssistantError::Unknown { message } => {
            assert_eq!(message, "Invalid input: bad option");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn other_unclassified_aisdk_errors_keep_their_display_messages() {
    let cases = [
        (AiError::ToolCallError("tool failed".to_string()), "Tool error: tool failed"),
        (AiError::PromptError("template failed".to_string()), "Prompt error: template failed"),
        (AiError::Other("miscellaneous".to_string()), "AI SDK error: miscellaneous"),
    ];

    for (source, expected) in cases {
        let error = assistant_error_from_aisdk("custom", source);

        match error {
            AssistantError::Unknown { message } => assert_eq!(message, expected),
            other => panic!("unexpected error: {other:?}"),
        }
    }
}

#[test]
fn api_log_fields_include_optional_details() {
    let error = AssistantError::APIError {
        message: "boom".to_string(),
        status_code: Some(503),
        is_retryable: true,
        response_headers: None,
        response_body: Some("body text".to_string()),
        metadata: Some(std::collections::HashMap::from([
            ("provider".to_string(), "openai".to_string()),
            ("request".to_string(), "abc".to_string()),
        ])),
    };

    let fields = aisdk_assistant_error_log_fields(&error);

    assert_eq!(fields.get("error"), Some(&Value::String("boom".to_string())));
    assert_eq!(fields.get("statusCode"), Some(&Value::from(503)));
    assert_eq!(fields.get("responseBody"), Some(&Value::String("body text".to_string())));
    assert_eq!(fields["metadata"]["provider"], Value::String("openai".to_string()));
    assert_eq!(fields["metadata"]["request"], Value::String("abc".to_string()));
}

#[test]
fn api_log_fields_omit_absent_optional_details() {
    let error = AssistantError::APIError {
        message: "plain failure".to_string(),
        status_code: None,
        is_retryable: false,
        response_headers: None,
        response_body: None,
        metadata: None,
    };

    let fields = aisdk_assistant_error_log_fields(&error);

    assert_eq!(fields.len(), 1);
    assert_eq!(fields.get("error"), Some(&Value::String("plain failure".to_string())));
    assert!(!fields.contains_key("statusCode"));
    assert!(!fields.contains_key("responseBody"));
    assert!(!fields.contains_key("metadata"));
}

#[test]
fn non_api_log_fields_serialize_error_variant() {
    let fields = aisdk_assistant_error_log_fields(&AssistantError::Unknown {
        message: "mystery".to_string(),
    });

    let error =
        fields.get("error").and_then(Value::as_str).expect("serialized error should be present");
    assert!(error.contains("Unknown"));
    assert!(error.contains("mystery"));
}

#[test]
fn source_chain_returns_none_when_aisdk_error_has_no_source() {
    assert_eq!(aisdk_error_source_chain(&AiError::Other("plain".to_string())), None);
}

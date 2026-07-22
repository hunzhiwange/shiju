use std::collections::HashMap;

use serde_json::json;

use crate::error::{
    ApiCallError, ParsedApiCallError, ParsedStreamError, parse_api_call_error, parse_stream_error,
};

#[test]
fn alibaba_market_activation_error_is_rewritten_to_actionable_message() {
    let parsed = parse_api_call_error(
        "alibaba-cn",
        ApiCallError {
            message:
                "Aliyun market app does not exist, the user may not have activated the service."
                    .to_string(),
            status_code: Some(400),
            is_retryable: false,
            url: None,
            response_headers: None,
            response_body: None,
        },
    );

    let ParsedApiCallError::ApiError { message, .. } = parsed else {
        panic!("expected api error");
    };

    assert!(message.contains("激活"));
    assert!(message.contains("siliconflow-cn"));
}

#[test]
fn unrelated_alibaba_error_message_is_preserved() {
    let parsed = parse_api_call_error(
        "alibaba-cn",
        ApiCallError {
            message: "some other upstream failure".to_string(),
            status_code: Some(500),
            is_retryable: true,
            url: None,
            response_headers: None,
            response_body: None,
        },
    );

    let ParsedApiCallError::ApiError { message, .. } = parsed else {
        panic!("expected api error");
    };

    assert_eq!(message, "some other upstream failure");
}

#[test]
fn parse_stream_error_rejects_non_json_non_object_and_non_error_events() {
    assert!(parse_stream_error("not json").is_none());
    assert!(parse_stream_error("[]").is_none());
    assert!(parse_stream_error(r#"{"type":"message"}"#).is_none());
    assert!(parse_stream_error(r#"{"type":"error","error":{"code":"unknown"}}"#).is_none());
}

#[test]
fn parse_stream_error_maps_context_overflow() {
    let parsed = parse_stream_error(
        r#"{"type":"error","error":{"code":"context_length_exceeded","message":"too long"}}"#,
    )
    .expect("context overflow should parse");

    match parsed {
        ParsedStreamError::ContextOverflow { message, response_body } => {
            assert_eq!(message, "输入超出该模型的上下文窗口");
            assert!(response_body.contains("context_length_exceeded"));
        }
        other => panic!("expected context overflow, got {other:?}"),
    }
}

#[test]
fn parse_stream_error_maps_known_api_error_codes() {
    let insufficient_quota =
        parse_stream_error(r#"{"type":"error","error":{"code":"insufficient_quota"}}"#).unwrap();
    let usage_not_included =
        parse_stream_error(r#"{"type":"error","error":{"code":"usage_not_included"}}"#).unwrap();
    let invalid_prompt = parse_stream_error(
        r#"{"type":"error","error":{"code":"invalid_prompt","message":"bad prompt"}}"#,
    )
    .unwrap();
    let invalid_prompt_without_message =
        parse_stream_error(r#"{"type":"error","error":{"code":"invalid_prompt"}}"#).unwrap();

    match insufficient_quota {
        ParsedStreamError::ApiError { message, is_retryable, .. } => {
            assert!(message.contains("配额"));
            assert!(!is_retryable);
        }
        other => panic!("expected api error, got {other:?}"),
    }
    match usage_not_included {
        ParsedStreamError::ApiError { message, is_retryable, .. } => {
            assert!(message.contains("Plus"));
            assert!(!is_retryable);
        }
        other => panic!("expected api error, got {other:?}"),
    }
    match invalid_prompt {
        ParsedStreamError::ApiError { message, is_retryable, .. } => {
            assert_eq!(message, "bad prompt");
            assert!(!is_retryable);
        }
        other => panic!("expected api error, got {other:?}"),
    }
    match invalid_prompt_without_message {
        ParsedStreamError::ApiError { message, .. } => assert_eq!(message, "无效的 prompt。"),
        other => panic!("expected api error, got {other:?}"),
    }
}

#[test]
fn parse_api_call_error_uses_response_body_status_or_unknown_for_empty_message() {
    let from_body = parse_api_call_error(
        "provider",
        ApiCallError {
            message: "   ".to_string(),
            status_code: Some(500),
            is_retryable: false,
            url: None,
            response_headers: None,
            response_body: Some("body failure".to_string()),
        },
    );
    let from_status = parse_api_call_error(
        "provider",
        ApiCallError {
            message: String::new(),
            status_code: Some(429),
            is_retryable: false,
            url: None,
            response_headers: None,
            response_body: None,
        },
    );
    let unknown = parse_api_call_error(
        "provider",
        ApiCallError {
            message: String::new(),
            status_code: None,
            is_retryable: false,
            url: None,
            response_headers: None,
            response_body: None,
        },
    );

    assert_api_error_message(from_body, "body failure");
    assert_api_error_message(from_status, "429");
    assert_api_error_message(unknown, "Unknown error");
}

#[test]
fn parse_api_call_error_detects_overflow_patterns_and_no_body_statuses() {
    for message in [
        "prompt is too long",
        "INPUT IS TOO LONG FOR REQUESTED MODEL",
        "exceeds the context window",
        "input token count exceeded",
        "maximum prompt length is 1",
        "reduce the length of the messages",
        "maximum context length is 1",
        "exceeds the available context size",
        "greater than the context length",
        "context window exceeds limit",
        "exceeded model token limit",
        "context_length_exceeded",
        "400 (no body)",
        "413 (no body)",
    ] {
        let parsed = parse_api_call_error(
            "provider",
            ApiCallError {
                message: message.to_string(),
                status_code: Some(400),
                is_retryable: false,
                url: None,
                response_headers: None,
                response_body: Some("raw body".to_string()),
            },
        );

        match parsed {
            ParsedApiCallError::ContextOverflow { message: parsed_message, response_body } => {
                assert_eq!(parsed_message, message);
                assert_eq!(response_body.as_deref(), Some("raw body"));
            }
            other => panic!("expected overflow for {message}, got {other:?}"),
        }
    }
}

#[test]
fn parse_api_call_error_keeps_retry_headers_body_and_url_metadata() {
    let headers = HashMap::from([("retry-after".to_string(), "5".to_string())]);
    let parsed = parse_api_call_error(
        "anthropic",
        ApiCallError {
            message: "rate limited".to_string(),
            status_code: Some(429),
            is_retryable: true,
            url: Some("https://api.example.test/v1".to_string()),
            response_headers: Some(headers.clone()),
            response_body: Some("slow down".to_string()),
        },
    );

    match parsed {
        ParsedApiCallError::ApiError {
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
            assert_eq!(response_headers, Some(headers));
            assert_eq!(response_body.as_deref(), Some("slow down"));
            assert_eq!(
                metadata.and_then(|m| m.get("url").cloned()),
                Some("https://api.example.test/v1".to_string())
            );
        }
        other => panic!("expected api error, got {other:?}"),
    }
}

#[test]
fn parse_api_call_error_treats_openai_404_as_retryable() {
    let parsed = parse_api_call_error(
        "openai-chat",
        ApiCallError {
            message: "not found".to_string(),
            status_code: Some(404),
            is_retryable: false,
            url: None,
            response_headers: None,
            response_body: None,
        },
    );

    match parsed {
        ParsedApiCallError::ApiError { is_retryable, .. } => assert!(is_retryable),
        other => panic!("expected api error, got {other:?}"),
    }
}

#[test]
fn parse_api_call_error_preserves_openai_retryable_for_non_404_status() {
    let not_retryable = parse_api_call_error(
        "openai-chat",
        ApiCallError {
            message: "server error".to_string(),
            status_code: Some(500),
            is_retryable: false,
            url: None,
            response_headers: None,
            response_body: None,
        },
    );
    let retryable = parse_api_call_error(
        "openai-chat",
        ApiCallError {
            message: "rate limited".to_string(),
            status_code: Some(429),
            is_retryable: true,
            url: None,
            response_headers: None,
            response_body: None,
        },
    );

    match not_retryable {
        ParsedApiCallError::ApiError { is_retryable, .. } => assert!(!is_retryable),
        other => panic!("expected api error, got {other:?}"),
    }
    match retryable {
        ParsedApiCallError::ApiError { is_retryable, .. } => assert!(is_retryable),
        other => panic!("expected api error, got {other:?}"),
    }
}

#[test]
fn parse_api_call_error_preserves_openai_retryable_without_status() {
    let parsed = parse_api_call_error(
        "openai",
        ApiCallError {
            message: "network".to_string(),
            status_code: None,
            is_retryable: true,
            url: None,
            response_headers: None,
            response_body: None,
        },
    );

    match parsed {
        ParsedApiCallError::ApiError { is_retryable, .. } => assert!(is_retryable),
        other => panic!("expected api error, got {other:?}"),
    }
}

#[test]
fn github_copilot_forbidden_error_is_rewritten() {
    let parsed = parse_api_call_error(
        "github-copilot-chat",
        ApiCallError {
            message: "forbidden".to_string(),
            status_code: Some(403),
            is_retryable: false,
            url: None,
            response_headers: None,
            response_body: None,
        },
    );

    match parsed {
        ParsedApiCallError::ApiError { message, .. } => {
            assert!(message.contains("copilot provider"));
            assert!(message.contains("认证"));
        }
        other => panic!("expected api error, got {other:?}"),
    }
}

#[test]
fn parsed_error_types_serialize_with_snake_case_tags() {
    let stream = ParsedStreamError::ApiError {
        message: "msg".to_string(),
        is_retryable: false,
        response_body: "{}".to_string(),
    };
    let api = ParsedApiCallError::ContextOverflow {
        message: "too long".to_string(),
        response_body: None,
    };

    assert_eq!(serde_json::to_value(stream).unwrap()["type"], json!("api_error"));
    assert_eq!(serde_json::to_value(api).unwrap()["type"], json!("context_overflow"));
}

fn assert_api_error_message(parsed: ParsedApiCallError, expected: &str) {
    match parsed {
        ParsedApiCallError::ApiError { message, .. } => assert_eq!(message, expected),
        other => panic!("expected api error, got {other:?}"),
    }
}

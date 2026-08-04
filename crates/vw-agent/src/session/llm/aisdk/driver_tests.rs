use std::collections::HashMap;

use aisdk::Error as AiError;
use aisdk::core::{LanguageModel, Message};
use reqwest::StatusCode;
use serde_json::{Value, json};

use crate::app::agent::provider::provider;
use crate::app::agent::session::llm::types::Error;
use crate::app::agent::session::message::AssistantError;

use super::super::convert::AisdkRequestInfo;
use super::{dispatch_stream_request, log_build_failed, resolve_model};

fn test_model(provider_id: &str, adapter: &str) -> provider::Model {
    serde_json::from_value(json!({
        "id": "gpt-test",
        "providerID": provider_id,
        "api": {
            "id": "gpt-test",
            "url": "not a url",
            "adapter": adapter
        },
        "name": "GPT Test",
        "family": null,
        "capabilities": {
            "temperature": true,
            "reasoning": true,
            "attachment": false,
            "toolcall": true,
            "input": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "output": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "interleaved": false
        },
        "cost": {
            "input": 0.0,
            "output": 0.0,
            "cache": {
                "read": 0.0,
                "write": 0.0
            },
            "experimental_over_200k": null
        },
        "limit": {
            "context": 8192,
            "input": null,
            "output": 4096
        },
        "status": "active",
        "options": {},
        "headers": {},
        "release_date": "2026-01-01",
        "variants": {}
    }))
    .expect("test model should deserialize")
}

fn request_info(base_url: &str) -> AisdkRequestInfo {
    AisdkRequestInfo {
        base_url: base_url.to_string(),
        request_url: format!("{}/chat/completions", base_url.trim_end_matches('/')),
        path_override: Some("chat/completions".to_string()),
        enforce_strict_tool_schema: true,
        messages: vec![Message::User("hello".into())],
    }
}

fn assert_invalid_base_url(result: Result<(), Error>) {
    match result {
        Err(Error::Api(AssistantError::Unknown { message })) => {
            assert!(message.contains("Invalid base URL"), "message was {message:?}");
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn resolve_model_uses_complete_provider_model_reference_and_overrides() {
    let mut model = test_model("alibaba-cn", "openai-compatible");
    model.api.id = "qwen3.7-plus-2026-05-26".to_string();
    let request_info = request_info("https://custom.example.com/v1");

    let resolved = resolve_model("alibaba-cn", &model, "token", &request_info)
        .expect("complete Alibaba model reference should resolve");

    assert_eq!(resolved.settings.provider_name, "alibaba-cn");
    assert_eq!(resolved.name(), "qwen3.7-plus-2026-05-26");
    assert_eq!(resolved.settings.api_key, "token");
    assert_eq!(resolved.settings.base_url, "https://custom.example.com/v1");
    assert_eq!(resolved.settings.path.as_deref(), Some("chat/completions"));
}

#[test]
fn log_build_failed_wraps_aisdk_error_as_api_error() {
    let model = test_model("openai", "openai-compatible");

    let error = log_build_failed(
        "openai",
        &model,
        "https://api.example.com/v1/chat/completions",
        AiError::MissingField("api_key".to_string()),
    );

    match error {
        Error::Api(AssistantError::ProviderAuthError { provider_id, message }) => {
            assert_eq!(provider_id, "openai");
            assert_eq!(message, "api_key");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn log_build_failed_preserves_non_authentication_error_details() {
    let model = test_model("openai", "openai-compatible");

    let error = log_build_failed(
        "openai",
        &model,
        "https://api.example.com/v1/chat/completions",
        AiError::InvalidInput("unsupported option".to_string()),
    );

    match error {
        Error::Api(AssistantError::Unknown { message }) => {
            assert_eq!(message, "Invalid input: unsupported option");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn log_build_failed_preserves_api_status_and_retryability() {
    let model = test_model("openai", "openai-compatible");

    let error = log_build_failed(
        "openai",
        &model,
        "https://api.example.com/v1/chat/completions",
        AiError::ApiError {
            details: "rate limited".to_string(),
            status_code: Some(StatusCode::TOO_MANY_REQUESTS),
        },
    );

    match error {
        Error::Api(AssistantError::APIError { message, status_code, is_retryable, .. }) => {
            assert_eq!(message, "rate limited");
            assert_eq!(status_code, Some(429));
            assert!(is_retryable);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn openai_compatible_dispatch_reports_model_build_failure() {
    let model = test_model("openai", "openai-compatible");
    let mut events = Vec::new();

    let result = dispatch_stream_request(
        "openai",
        &model,
        "token",
        request_info("not a url"),
        &HashMap::new(),
        None,
        None,
        None,
        &Value::Null,
        0,
        None,
        &mut |event| events.push(event),
    )
    .await;

    assert!(events.is_empty());
    assert_invalid_base_url(result);
}

#[tokio::test]
async fn alibaba_dispatch_reports_model_build_failure() {
    let model = test_model("alibaba-cn", "openai-compatible");
    let mut events = Vec::new();

    let result = dispatch_stream_request(
        "alibaba-cn",
        &model,
        "token",
        request_info("not a url"),
        &HashMap::new(),
        None,
        None,
        None,
        &Value::Null,
        0,
        None,
        &mut |event| events.push(event),
    )
    .await;

    assert!(events.is_empty());
    assert_invalid_base_url(result);
}

#[tokio::test]
async fn openai_compatible_dispatch_without_path_reports_model_build_failure() {
    let model = test_model("openai", "openai-compatible");
    let mut request_info = request_info("not a url");
    request_info.path_override = None;
    let mut events = Vec::new();

    let result = dispatch_stream_request(
        "openai",
        &model,
        "token",
        request_info,
        &HashMap::new(),
        None,
        None,
        None,
        &Value::Null,
        0,
        None,
        &mut |event| events.push(event),
    )
    .await;

    assert!(events.is_empty());
    assert_invalid_base_url(result);
}

#[tokio::test]
async fn alibaba_dispatch_without_path_reports_model_build_failure() {
    let model = test_model("alibaba-cn", "openai-compatible");
    let mut request_info = request_info("not a url");
    request_info.path_override = None;
    let mut events = Vec::new();

    let result = dispatch_stream_request(
        "alibaba-cn",
        &model,
        "token",
        request_info,
        &HashMap::new(),
        None,
        None,
        None,
        &Value::Null,
        0,
        None,
        &mut |event| events.push(event),
    )
    .await;

    assert!(events.is_empty());
    assert_invalid_base_url(result);
}

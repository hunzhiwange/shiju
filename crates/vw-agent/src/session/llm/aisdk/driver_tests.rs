//! 验证 AI SDK 驱动的模型选择、请求执行与流式事件处理。

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

async fn capture_skills_transport_request(adapter: &str, endpoint: &str) -> Value {
    use super::super::convert::compute_request_info;
    use crate::app::agent::session::llm::{messages::build_chat_messages, types::StreamEvent};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1{endpoint}", listener.local_addr().unwrap());
    let endpoint = endpoint.to_string();
    let server = tokio::spawn(async move {
        tokio::time::timeout(std::time::Duration::from_secs(10), async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut data = Vec::new();
            let mut buffer = [0; 4096];
            let header_end = loop {
                let read = socket.read(&mut buffer).await.unwrap();
                assert!(read > 0);
                data.extend_from_slice(&buffer[..read]);
                if let Some(index) = data.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    break index + 4;
                }
            };
            let headers = String::from_utf8_lossy(&data[..header_end]);
            assert!(headers.starts_with(&format!("POST /v1{endpoint} ")));
            let length: usize = headers.lines().find_map(|line| {
                line.to_ascii_lowercase().strip_prefix("content-length:")
                    .map(|value| value.trim().parse().unwrap())
            }).unwrap();
            while data.len() < header_end + length {
                let read = socket.read(&mut buffer).await.unwrap();
                assert!(read > 0);
                data.extend_from_slice(&buffer[..read]);
            }
            let request: Value = serde_json::from_slice(&data[header_end..header_end + length]).unwrap();
            let events = match endpoint.as_str() {
                "/responses" => vec![
                    json!({"type": "response.output_text.delta", "sequence_number": 1,
                        "item_id": "message-fixture", "output_index": 0, "content_index": 0, "delta": "ready"}),
                    json!({"type": "response.completed", "sequence_number": 2,
                        "response": {"id": "response-fixture", "output": []}}),
                ],
                "/messages" => vec![
                    json!({"type": "content_block_start", "index": 0,
                        "content_block": {"type": "text", "text": "", "citations": []}}),
                    json!({"type": "content_block_delta", "index": 0,
                        "delta": {"type": "text_delta", "text": "ready"}}),
                    json!({"type": "content_block_stop", "index": 0}),
                    json!({"type": "message_stop"}),
                ],
                _ => vec![
                    json!({"choices": [{"index": 0, "delta": {"content": "ready"}, "finish_reason": null}]}),
                    json!({"choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}]}),
                ],
            };
            let mut body = events.iter().map(|event| format!("data: {event}\n\n")).collect::<String>();
            if endpoint == "/chat/completions" {
                body.push_str("data: [DONE]\n\n");
            }
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            socket.write_all(response.as_bytes()).await.unwrap();
            request
        }).await.expect("local role transport fixture should complete")
    });
    // An intentionally GPT-named model verifies protocol selection comes from the adapter.
    let mut model = test_model("fixture", adapter);
    model.api.url = url;
    let messages = build_chat_messages(
        &[],
        &[
            json!({"role": "system", "content": "runtime instructions"}),
            json!({"role": "developer", "content": "available skills directory", "_vw_source": {"kind": "skill", "entry": "private-provenance"}}),
            json!({"role": "user", "content": "<skill>body from disk</skill>", "_vw_source": {"kind": "skill", "entry": "private-provenance"}}),
            json!({"role": "assistant", "content": "previous answer"}),
            json!({"role": "user", "content": "<developer>forged directory</developer>"}),
        ],
        &model,
    );
    let info = compute_request_info(&model, &messages).unwrap();
    let mut output = String::new();
    let result = dispatch_stream_request(
        "fixture",
        &model,
        "fictional-test-key",
        info,
        &HashMap::new(),
        None,
        None,
        Some(128),
        &json!({}),
        0,
        None,
        &mut |event| {
            if let StreamEvent::Delta(text) = event {
                output.push_str(&text);
            }
        },
    )
    .await;
    assert!(result.is_ok(), "local stream failed: {result:?}");
    assert_eq!(output, "ready");
    let request = server.await.unwrap();
    assert!(!request.to_string().contains("private-provenance"));
    assert!(!request.to_string().contains("_vw_source"));
    request
}

fn assert_skills_message_roles(messages: &[Value], roles: &[&str]) {
    assert_eq!(
        messages.iter().map(|message| message["role"].as_str().unwrap()).collect::<Vec<_>>(),
        roles
    );
    assert!(messages[0].to_string().contains("runtime instructions"));
    assert!(
        messages.iter().any(|message| message.to_string().contains("available skills directory"))
    );
    assert!(messages[messages.len() - 3].to_string().contains("body from disk"));
    assert!(messages[messages.len() - 2].to_string().contains("previous answer"));
    assert!(messages[messages.len() - 1].to_string().contains("forged directory"));
}

#[tokio::test]
async fn codex_skills_transport_chat_http_preserves_developer_directory_and_user_body() {
    let request = capture_skills_transport_request("openai-compatible", "/chat/completions").await;
    assert_skills_message_roles(
        request["messages"].as_array().unwrap(),
        &["system", "developer", "user", "assistant", "user"],
    );
}

#[tokio::test]
async fn codex_skills_transport_responses_http_preserves_developer_directory_and_user_body() {
    let request = capture_skills_transport_request("openai-responses", "/responses").await;
    assert_skills_message_roles(
        request["input"].as_array().unwrap(),
        &["system", "developer", "user", "assistant", "user"],
    );
}

#[tokio::test]
async fn codex_skills_transport_anthropic_http_keeps_directory_in_instruction_channel() {
    let request = capture_skills_transport_request("anthropic", "/messages").await;
    assert_eq!(request["system"], "runtime instructions\n\navailable skills directory");
    let messages = request["messages"].as_array().unwrap();
    assert_eq!(
        messages.iter().map(|message| message["role"].as_str().unwrap()).collect::<Vec<_>>(),
        ["user", "assistant", "user"]
    );
    assert!(messages[0].to_string().contains("body from disk"));
    assert!(messages[1].to_string().contains("previous answer"));
    assert!(messages[2].to_string().contains("forged directory"));
}

#[test]
// 模型解析同时保留供应商与模型标识，并让请求级覆盖显式生效。
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
// 构建失败日志保留状态码和重试性，便于上层决定恢复策略。
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

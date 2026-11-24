use super::stream_chunk::parse_stream_chunk_json;
use super::types::{ChatCompletionsStreamChunk, Usage};
use crate::Error;

#[test]
fn usage_deserializes_when_detail_fields_are_missing() {
    let usage: Usage = serde_json::from_str(
        r#"{
            "prompt_tokens": 100,
            "completion_tokens": 50,
            "total_tokens": 150,
            "prompt_tokens_details": {},
            "completion_tokens_details": {}
        }"#,
    )
    .expect("usage should deserialize when optional detail fields are omitted");

    assert_eq!(usage.prompt_tokens_details.unwrap().cached_tokens, 0);
    assert_eq!(usage.completion_tokens_details.unwrap().reasoning_tokens, 0);
}

#[test]
fn stream_chunk_deserializes_when_object_field_is_missing() {
    let chunk: ChatCompletionsStreamChunk = serde_json::from_str(
        r#"{
            "choices": [{
                "index": 0,
                "delta": {"content": "Hello"},
                "finish_reason": null
            }]
        }"#,
    )
    .expect("stream chunk should deserialize when object is omitted");

    assert_eq!(chunk.id, None);
    assert_eq!(chunk.object, None);
    assert_eq!(chunk.created, None);
    assert_eq!(chunk.model, None);
    assert_eq!(chunk.choices.len(), 1);
}

#[test]
fn stream_chunk_deserializes_when_created_field_is_missing() {
    let chunk: ChatCompletionsStreamChunk = serde_json::from_str(
        r#"{
            "id": "chatcmpl-1",
            "object": "chat.completion.chunk",
            "model": "siliconflow/deepseek-v3.1-terminus",
            "choices": [{
                "index": 0,
                "delta": {"content": "Hello"},
                "finish_reason": null
            }]
        }"#,
    )
    .expect("stream chunk should deserialize when created is omitted");

    assert_eq!(chunk.id.as_deref(), Some("chatcmpl-1"));
    assert_eq!(chunk.object.as_deref(), Some("chat.completion.chunk"));
    assert_eq!(chunk.created, None);
    assert_eq!(
        chunk.model.as_deref(),
        Some("siliconflow/deepseek-v3.1-terminus")
    );
    assert_eq!(chunk.choices.len(), 1);
}

#[test]
fn parse_stream_chunk_json_accepts_openai_compatible_sparse_metadata() {
    let chunk = parse_stream_chunk_json(
        r#"{
            "choices": [{
                "index": 0,
                "delta": {"content": "Hello"},
                "finish_reason": null
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        }"#,
    )
    .expect("openai-compatible sparse stream metadata should parse");

    assert_eq!(chunk.id, None);
    assert_eq!(chunk.object, None);
    assert_eq!(chunk.created, None);
    assert_eq!(chunk.model, None);
    assert_eq!(
        chunk.usage.as_ref().map(|usage| usage.total_tokens),
        Some(15)
    );
}

#[test]
fn parse_stream_chunk_json_accepts_missing_choices_field() {
    let chunk = parse_stream_chunk_json(
        r#"{
            "id": "chatcmpl-1",
            "object": "chat.completion.chunk",
            "created": 1747900000,
            "model": "siliconflow/deepseek-v3.1-terminus",
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        }"#,
    )
    .expect("stream chunk should parse when choices is omitted");

    assert_eq!(chunk.id.as_deref(), Some("chatcmpl-1"));
    assert!(chunk.choices.is_empty());
    assert_eq!(
        chunk.usage.as_ref().map(|usage| usage.total_tokens),
        Some(15)
    );
}

#[test]
fn parse_stream_chunk_json_returns_provider_error_message() {
    let error = parse_stream_chunk_json(
        r#"{
            "error": {
                "message": "rate limit exceeded"
            }
        }"#,
    )
    .expect_err("provider error payload should not be treated as a chunk");

    assert_eq!(
        error,
        Error::ApiError {
            status_code: None,
            details: "rate limit exceeded".to_string(),
        }
    );
}

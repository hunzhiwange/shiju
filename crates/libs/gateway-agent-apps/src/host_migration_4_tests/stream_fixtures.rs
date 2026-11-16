//! Shared existing wire fixtures for diagnostic and transport tests.
use serde_json::{Value, json};
pub(crate) fn samples() -> Vec<(&'static str, &'static str, String)> {
    let data = |value: Value| format!("data: {value}\n\n");
    vec![
        ("openai_chat", "/v1/chat/completions", [
            data(json!({"object":"chat.completion.chunk","model":"fixture-v1","choices":[{"index":0,"delta":{"role":"assistant","content":"OK"},"finish_reason":null}]})),
            data(json!({"object":"chat.completion.chunk","model":"fixture-v1","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]})),
            "data: [DONE]\n\n".into(),
        ].concat()),
        ("anthropic", "/v1/messages", [
            data(json!({"type":"message_start","message":{"id":"m","role":"assistant","model":"fixture-v1","type":"message","content":[]}})),
            data(json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}})),
            data(json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"OK"}})),
            data(json!({"type":"content_block_stop","index":0})),
            data(json!({"type":"message_delta","delta":{"stop_reason":"end_turn"}})),
            data(json!({"type":"message_stop"})),
        ].concat()),
        ("openai_responses", "/v1/responses", [
            data(json!({"type":"response.created","response":{"id":"r","object":"response","model":"fixture-v1","status":"in_progress"}})),
            data(json!({"type":"response.output_text.delta","delta":"OK","item_id":"m","output_index":0,"content_index":0})),
            data(json!({"type":"response.completed","response":{"id":"r","object":"response","model":"fixture-v1","status":"completed"}})),
        ].concat()),
        ("gemini_native", "/v1/models/fixture:streamGenerateContent?alt=sse", [
            data(json!({"modelVersion":"fixture-v1","candidates":[{"index":0,"content":{"role":"model","parts":[{"text":"OK"}]}}]})),
            data(json!({"modelVersion":"fixture-v1","candidates":[{"index":0,"finishReason":"STOP"}]})),
        ].concat()),
    ]
}

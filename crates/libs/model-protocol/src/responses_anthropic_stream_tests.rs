use super::responses_anthropic_stream::response_body;
use axum::body::{Body, to_bytes};
use serde_json::json;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn cc_route_059_responses_anthropic_stream_expected_behavior() {
    let frames = [
        json!({"type":"message_start","message":{"role":"assistant","content":[],"usage":{"input_tokens":2,"output_tokens":0}}}),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"consider"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"content_block_start","index":1,"content_block":{"type":"text","text":"answer"}}),
        json!({"type":"content_block_stop","index":1}),
        json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":3}}),
        json!({"type":"message_stop"}),
    ];
    let wire: String = frames.iter().map(|v| format!("data: {v}\n\n")).collect();
    let body = response_body(Body::from(wire), "fixture".into(), Default::default(), CancellationToken::new());
    let bytes = to_bytes(body, 1024 * 1024).await.unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(text.contains("response.reasoning_summary_text.delta"));
    assert!(text.contains("response.output_text.delta"));
    assert!(text.contains("response.completed"));
    assert!(!text.contains("response.failed"));
}

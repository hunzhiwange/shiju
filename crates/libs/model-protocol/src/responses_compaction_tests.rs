use super::responses_compaction::Request;
use axum::{
    body::{Body, Bytes, to_bytes},
    http::StatusCode,
};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;
use vw_agent_apps::routing::compaction_replay::replay_text;

async fn stream(finish: &str) -> Vec<Value> {
    let request = Request::from_responses(json!({"model":"published","stream":true,"input":[
        {"type":"message","role":"user","content":"remember this"},
        {"type":"compaction_trigger"}
    ]}))
    .unwrap();
    let frames = [
        json!({"choices":[{"index":0,"delta":{"content":"摘要","reasoning_content":"private"}}]}),
        json!({"choices":[{"index":0,"delta":{},"finish_reason":finish}],
            "usage":{"prompt_tokens":3,"completion_tokens":2}}),
    ];
    let mut wire = frames.iter().map(|frame| format!("data: {frame}\r\n\r\n")).collect::<String>();
    wire.push_str("data: [DONE]\r\n\r\n");
    let chunks =
        wire.into_bytes().into_iter().map(|byte| Ok::<_, std::io::Error>(Bytes::from(vec![byte])));
    let body = request.response_body(
        Body::from_stream(futures_util::stream::iter(chunks)),
        CancellationToken::new(),
    );
    let bytes = to_bytes(body, 16384).await.unwrap();
    let events = std::str::from_utf8(&bytes).unwrap();
    assert!(!events.contains("private"));
    events
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(|data| serde_json::from_str(data).unwrap())
        .collect()
}

#[tokio::test]
async fn aggregation_t25_compaction_stream_emits_one_replayable_item() {
    let events = stream("stop").await;
    let items: Vec<_> =
        events.iter().filter(|event| event["type"] == "response.output_item.done").collect();
    assert_eq!(items.len(), 1);
    let item = &items[0]["item"];
    assert_eq!(item["type"], "compaction");
    let payload = item["encrypted_content"].as_str().unwrap();
    assert!(replay_text(payload).unwrap().ends_with("\n摘要"));
    let response = events.last().unwrap();
    assert_eq!(response["type"], "response.completed");
    assert_eq!(response["response"]["output"], json!([item]));
    assert_eq!(response["response"]["usage"]["total_tokens"], 5);
}

#[tokio::test]
async fn aggregation_t25_incomplete_compaction_emits_no_item() {
    for (finish, reason) in [("length", "max_output_tokens"), ("content_filter", "content_filter")]
    {
        let events = stream(finish).await;
        assert!(events.iter().all(|event| event.get("item").is_none()));
        let last = events.last().unwrap();
        assert_eq!(last["type"], "response.incomplete");
        assert_eq!(last["response"]["incomplete_details"]["reason"], reason);
    }
}

#[test]
fn aggregation_t25_compaction_request_protects_the_summary_turn() {
    let request = Request::from_responses(json!({"model":"selected","stream":true,
        "tools":[{"type":"function","name":"lookup","parameters":{"type":"object"}}],
        "tool_choice":"required",
        "input":[{"type":"message","role":"user","content":"remember this",
            "internal_chat_message_metadata_passthrough":{"fixture":true}},
            {"type":"compaction_trigger"}]
    }))
    .unwrap();
    let mut overrides = json!({"model":"different","stream":false,"tool_choice":"required",
        "messages":[],"response_format":{"type":"json_object"},"n":2});
    request.apply_wire_controls(overrides.as_object_mut().unwrap());
    assert_eq!(overrides["model"], "selected");
    assert_eq!(overrides["stream"], true);
    assert_eq!(overrides["tool_choice"], "none");
    assert_eq!(overrides["tools"][0]["function"]["name"], "lookup");
    assert_eq!(overrides["messages"][0], json!({"role":"user","content":"remember this"}));
    assert!(overrides["messages"][1]["content"].as_str().unwrap().contains("CONTEXT CHECKPOINT"));
    assert!(overrides.get("response_format").is_none());
    assert!(overrides.get("n").is_none());
    let ordinary =
        json!({"model":"selected","stream":true,"input":[{"role":"user","content":"hi"}]});
    assert!(matches!(Request::from_responses(ordinary), Err(StatusCode::UNPROCESSABLE_ENTITY)));
    let duplicate = json!({"model":"selected","stream":true,"input":[
        {"type":"compaction_trigger"}, {"type":"compaction_trigger"}
    ]});
    assert!(matches!(Request::from_responses(duplicate), Err(StatusCode::BAD_REQUEST)));
}

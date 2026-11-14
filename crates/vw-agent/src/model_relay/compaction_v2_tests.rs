use super::compaction_v2;
use axum::body::{Body, to_bytes};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

#[test]
fn cc_route_078_compaction_v2_expected_behavior() {
    let mut body = json!({"model":"selected","stream":true,"input":[{"type":"compaction_trigger"}],
        "tools":[{"type":"web_search"}],"tool_choice":"required","text":{"format":{"type":"json_object"},"verbosity":"low"}});
    assert!(compaction_v2::is_request(&body));
    compaction_v2::prepare_native(&mut body).unwrap();
    assert!(!compaction_v2::is_request(&body));
    assert_eq!(body["tool_choice"], "none");
    assert_eq!(body["tools"][0]["type"], "web_search");
    assert_eq!(body["text"], json!({"verbosity":"low"}));
    assert!(body["input"][0]["content"].as_str().unwrap().contains("CONTEXT CHECKPOINT"));
}

async fn terminal(status: &str, text: &str) -> Vec<Value> {
    let value = json!({"type":"response.completed","response":{"status":status,
        "incomplete_details":{"reason":"max_output_tokens"},"output":[{"type":"message","content":[{"type":"output_text","text":text}]}]}});
    let wire = format!("data: {value}\r\n\r\n");
    let body = compaction_v2::response_body(Body::from(wire), CancellationToken::new());
    let bytes = to_bytes(body, 16384).await.unwrap();
    std::str::from_utf8(&bytes).unwrap().lines().filter_map(|line| line.strip_prefix("data: "))
        .map(|data| serde_json::from_str(data).unwrap()).collect()
}

#[tokio::test]
async fn cc_route_078_native_summary_emits_replayable_compaction() {
    let events = terminal("completed", "摘要").await;
    assert_eq!(events[0]["type"], "response.output_item.done");
    let item = &events[0]["item"];
    assert_eq!(item["type"], "compaction");
    assert!(vw_agent_apps::routing::compaction_replay::replay_text(item["encrypted_content"].as_str().unwrap()).unwrap().ends_with("\n摘要"));
    assert_eq!(events[1]["response"]["output"][1], *item);
}

#[tokio::test]
async fn cc_route_078_incomplete_preserves_reason_and_empty_summary_fails() {
    let events = terminal("incomplete", "partial").await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["type"], "response.incomplete");
    assert_eq!(events[0]["response"]["incomplete_details"]["reason"], "max_output_tokens");
    assert_eq!(terminal("completed", "").await[0]["type"], "response.failed");
}

#[tokio::test]
async fn cc_route_078_cancel_stops_pending_upstream() {
    let cancel = CancellationToken::new();
    cancel.cancel();
    let stream = futures_util::stream::pending::<Result<axum::body::Bytes, std::io::Error>>();
    let body = compaction_v2::response_body(Body::from_stream(stream), cancel);
    assert!(to_bytes(body, 1024).await.unwrap().is_empty());
}

#[test]
fn cc_route_078_anthropic_summary_request_is_supported() {
    let request = super::responses_compaction::Request::from_responses_anthropic(json!({
        "model":"selected","stream":true,"input":[{"type":"compaction_trigger"}]
    })).unwrap();
    assert!(request.chat["messages"].to_string().contains("CONTEXT CHECKPOINT"));
}

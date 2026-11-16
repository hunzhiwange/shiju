use serde_json::json;

#[test]
fn cc_route_078_anthropic_summary_request_is_supported() {
    let request = super::responses_compaction::Request::from_responses_anthropic(json!({
        "model":"selected","stream":true,"input":[{"type":"compaction_trigger"}]
    })).unwrap();
    assert!(request.chat["messages"].to_string().contains("CONTEXT CHECKPOINT"));
}

use super::signature_rectify::repair;
use axum::http::StatusCode;
use serde_json::json;

#[test]
fn cc_route_065_signature_rectify_expected_behavior() {
    let url = reqwest::Url::parse("https://example.com/v1/messages").unwrap();
    let body = json!({"messages":[{"role":"assistant","content":[{"type":"thinking","thinking":"t","signature":"bad"},{"type":"text","text":"keep"}]}]});
    let fixed = repair(&body, StatusCode::BAD_REQUEST, b"Invalid signature in thinking block", &url).unwrap().unwrap();
    assert_eq!(fixed["messages"][0]["content"], json!([{"type":"text","text":"keep"}]));
    assert!(repair(&fixed, StatusCode::BAD_REQUEST, b"Invalid signature in thinking block", &url).unwrap().is_none());
    assert!(repair(&body, StatusCode::BAD_REQUEST, b"invalid request", &url).unwrap().is_none());
    assert!(repair(&body, StatusCode::UNAUTHORIZED, b"Invalid signature in thinking block", &url).unwrap().is_none());
}

#[test]
fn cc_route_065_encrypted_task_is_rejected() {
    let url = reqwest::Url::parse("https://example.com/v1/responses").unwrap();
    let body = json!({"input":[{"type":"agent_message","content":[{"type":"encrypted_content","encrypted_content":"opaque"}]},{"type":"additional_tools"}]});
    assert!(repair(&body, StatusCode::BAD_REQUEST, br#"{"error":{"code":"invalid_encrypted_content"}}"#, &url).is_err());
}

#[test]
fn cc_route_065_tool_payload_and_call_id_survive() {
    let url = reqwest::Url::parse("https://example.com/v1/responses").unwrap();
    let body = json!({"input":[{"type":"reasoning","encrypted_content":"opaque"},{"type":"function_call","call_id":"c1","name":"tool","arguments":"{}"}]});
    let fixed = repair(&body, StatusCode::BAD_REQUEST, br#"{"error":{"code":"invalid_encrypted_content"}}"#, &url).unwrap().unwrap();
    assert_eq!(fixed["input"], json!([body["input"][1].clone()]));
}

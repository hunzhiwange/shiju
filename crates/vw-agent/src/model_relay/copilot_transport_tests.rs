use super::copilot_transport::Context;
use serde_json::json;

#[test]
fn cc_route_071_copilot_transport_expected_behavior() {
    let input = json!({"model":"claude-sonnet-4-6[1m]","messages":[{"role":"user","content":[{"type":"tool_result","tool_use_id":"fixture"}]}]});
    let mut request = reqwest::Request::new(reqwest::Method::POST, "https://api.githubcopilot.com/chat/completions".parse().unwrap());
    *request.body_mut() = Some(input.to_string().into());
    Context::new(&input).apply(&mut request).unwrap();
    let body: serde_json::Value = serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
    assert_eq!(body["model"], "claude-sonnet-4.6-1m");
    assert_eq!(request.headers()["x-initiator"], "agent");
    assert!(!request.headers().contains_key("authorization"));
    assert_eq!(request.url().path(), "/chat/completions");
}

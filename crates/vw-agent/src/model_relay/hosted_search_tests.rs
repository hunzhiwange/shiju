use super::{
    hosted_search, messages_responses_request, messages_responses_result, messages_responses_stream,
};
use serde_json::json;

fn request() -> serde_json::Value {
    json!({"model":"m","max_tokens":100,"messages":[{"role":"user","content":"search"}],
        "tools":[{"type":"web_search_20250305","name":"search","max_uses":1}],
        "tool_choice":{"type":"tool","name":"search"}})
}

#[test]
fn cc_route_044_hosted_search_expected_behavior() {
    let output = messages_responses_request::to_responses(&request()).unwrap();
    assert_eq!(output["tools"], json!([{"type":"web_search"}]));
    assert_eq!(output["tool_choice"], json!({"type":"web_search"}));
    assert_eq!(output["max_tool_calls"], 1);
    assert!(super::messages::to_chat(&request()).is_err());
}

#[test]
fn cc_route_044_failure_and_limit_are_returned_to_client() {
    let search = hosted_search::request(&request()).unwrap().2;
    let body = json!({"id":"r","model":"upstream","status":"completed","output":[
        {"id":"ws1","type":"web_search_call","status":"failed","error":{"code":"rate_limit"}},
        {"id":"ws2","type":"web_search_call","status":"completed","action":{"type":"search","query":"q"}}
    ],"usage":{"input_tokens":2,"output_tokens":1}});
    let result = messages_responses_result::with_search(&body, "m", &search).unwrap();
    assert_eq!(result["content"][1]["content"]["error_code"], "too_many_requests");
    assert_eq!(result["content"][3]["content"]["error_code"], "max_uses_exceeded");
    assert_eq!(result["stop_reason"], "end_turn");
    assert_eq!(result["usage"]["server_tool_use"]["web_search_requests"], 2);
}

#[test]
fn cc_route_044_rejects_invalid_limits_and_execution_modes() {
    for limit in [json!(0), json!(-1), json!(1.5), json!("1")] {
        let mut body = request();
        body["tools"][0]["max_uses"] = limit;
        assert!(messages_responses_request::to_responses(&body).is_err());
    }
    let mut body = request();
    body["tools"][0]["type"] = json!("web_search_20260209");
    assert!(messages_responses_request::to_responses(&body).is_err());
    // Without a declared hosted tool, even terminal streamed search is rejected.
    let mut decoder = messages_responses_stream::Decoder::new("m".into());
    assert!(decoder.frame(&json!({"type":"response.completed","response":{
        "id":"r","model":"upstream","status":"completed","output":[{"id":"w","type":"web_search_call","status":"completed"}],
        "usage":{"input_tokens":1,"output_tokens":1}}}).to_string()).is_err());
}

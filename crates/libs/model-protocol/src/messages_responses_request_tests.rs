use super::messages_responses_request::to_responses;
use axum::http::StatusCode;
use serde_json::json;

#[test]
fn cc_route_041_messages_responses_request_expected_behavior() {
    let body = json!({"model":"selected","max_tokens":64,"stream":true,
        "system":[{"type":"text","text":"x-anthropic-billing-header: private\n\nFollow rules"}],
        "tools":[{"name":"lookup","description":"Find a value","input_schema":{"type":"object","properties":{}}}],
        "tool_choice":{"type":"tool","name":"lookup","disable_parallel_tool_use":true},
        "messages":[{"role":"user","content":"Find it"},
            {"role":"assistant","content":[{"type":"tool_use","id":"call_1","name":"lookup","input":{"key":"a"}}]},
            {"role":"user","content":[{"type":"tool_result","tool_use_id":"call_1","content":"value"}]}]});
    let result = to_responses(&body).unwrap();
    assert_eq!(result["instructions"], "Follow rules");
    assert_eq!(result["model"], "selected");
    assert_eq!(result["max_output_tokens"], 64);
    assert_eq!(result["stream"], true);
    assert_eq!(result["store"], false);
    assert_eq!(result["parallel_tool_calls"], false);
    assert_eq!(result["tool_choice"], json!({"type":"function","name":"lookup"}));
    assert_eq!(result["tools"][0]["name"], "lookup");
    assert_eq!(result["input"][0]["role"], "user");
    assert_eq!(result["input"][1]["type"], "function_call");
    assert_eq!(result["input"][1]["call_id"], "call_1");
    assert_eq!(serde_json::from_str::<serde_json::Value>(result["input"][1]["arguments"].as_str().unwrap()).unwrap(), json!({"key":"a"}));
    assert_eq!(result["input"][2], json!({"type":"function_call_output","call_id":"call_1","output":"value"}));
    assert!(result.get("messages").is_none());
    assert!(result.get("stream_options").is_none());
}

#[test]
fn cc_route_041_rejects_orphan_outputs_and_unsupported_controls() {
    let mut body = json!({"model":"m","max_tokens":1,"messages":[{"role":"user","content":"hello"}]});
    assert_eq!(to_responses(&body).unwrap()["max_output_tokens"], 1);
    body["stop_sequences"] = json!(["STOP"]);
    assert_eq!(to_responses(&body), Err(StatusCode::UNPROCESSABLE_ENTITY));
    body.as_object_mut().unwrap().remove("stop_sequences");
    body["messages"] = json!([{"role":"user","content":[{"type":"tool_result","tool_use_id":"unknown","content":"value"}]}]);
    assert_eq!(to_responses(&body), Err(StatusCode::BAD_REQUEST));
}

#[test]
fn cc_route_041_rejects_destination_fields_and_preserves_old_relay_policy() {
    let mut body = json!({"model":"m","max_tokens":16,"tool_choice":"auto","messages":[{"role":"user","content":"hello"}]});
    assert!(to_responses(&body).is_ok());
    assert!(super::messages::to_chat(&body).is_err());
    body["api_key"] = json!("fixture");
    assert_eq!(to_responses(&body), Err(StatusCode::UNPROCESSABLE_ENTITY));
}

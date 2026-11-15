use super::messages_responses_result::from_responses;
use axum::http::StatusCode;
use serde_json::{Value, json};

fn response() -> Value {
    json!({"id":"resp_42","model":"upstream","status":"completed","error":null,
        "output":[{"type":"message","role":"assistant","status":"completed",
            "content":[{"type":"output_text","text":"answer","annotations":[]}]}],
        "usage":{"input_tokens":20,"output_tokens":3}})
}

#[test]
fn cc_route_042_messages_responses_result_expected_behavior() {
    let result = from_responses(&response(), "client").unwrap();
    assert_eq!(
        result,
        json!({"id":"resp_42","type":"message","role":"assistant",
        "model":"client","content":[{"type":"text","text":"answer"}],
        "stop_reason":"end_turn","stop_sequence":null,
        "usage":{"input_tokens":20,"output_tokens":3}})
    );
}

#[test]
fn cc_route_042_tools_refusal_order_and_cache_usage() {
    let mut body = response();
    body["output"].as_array_mut().unwrap().extend([
        json!({"type":"function_call","id":"item","call_id":"call_42",
            "name":"lookup","arguments":"{\"x\":1}","status":"completed"}),
        json!({"type":"message","role":"assistant","content":[
            {"type":"refusal","refusal":"declined"}]}),
    ]);
    body["usage"]["input_tokens_details"] = json!({"cached_tokens":8,"cache_write_tokens":2});
    let result = from_responses(&body, "client").unwrap();
    assert_eq!(
        result["content"],
        json!([
        {"type":"text","text":"answer"},
        {"type":"tool_use","id":"call_42","name":"lookup","input":{"x":1}},
        {"type":"text","text":"declined"}])
    );
    assert_eq!(result["stop_reason"], "tool_use");
    assert_eq!(
        result["usage"],
        json!({"input_tokens":10,"output_tokens":3,
        "cache_read_input_tokens":8,"cache_creation_input_tokens":2})
    );
    body["usage"]["cache_read_input_tokens"] = json!(0);
    body["usage"]["cache_creation_input_tokens"] = json!(1);
    assert_eq!(
        from_responses(&body, "m").unwrap()["usage"],
        json!({"input_tokens":19,"output_tokens":3,"cache_creation_input_tokens":1})
    );
    // The older session bridge still rejects refusal blocks.
    assert!(
        super::protocol_bridge::response(vw_shared::provider::ModelProtocol::Responses, &body)
            .is_err()
    );
}

#[test]
fn cc_route_042_incomplete_is_never_successful_end_turn() {
    for reason in ["max_output_tokens", "max_tokens"] {
        let mut body = response();
        body["status"] = json!("incomplete");
        body["incomplete_details"] = json!({"reason":reason});
        body["output"][0]["status"] = json!("incomplete");
        assert_eq!(from_responses(&body, "m").unwrap()["stop_reason"], "max_tokens");
    }
    for reason in [Value::Null, json!("content_filter"), json!("unknown")] {
        let mut body = response();
        body["status"] = json!("incomplete");
        body["incomplete_details"] = json!({"reason":reason});
        assert_eq!(from_responses(&body, "m"), Err(StatusCode::BAD_GATEWAY));
    }
}

#[test]
fn cc_route_042_failures_and_malformed_results_fail_closed() {
    for status in ["failed", "cancelled", "queued", "in_progress", "unknown"] {
        let mut body = response();
        body["status"] = json!(status);
        assert_eq!(from_responses(&body, "m"), Err(StatusCode::BAD_GATEWAY));
    }
    for patch in [
        json!({"error":{"message":"private upstream error"}}),
        json!({"status":null}),
        json!({"output":null}),
        json!({"output":[{"type":"web_search_call"}]}),
        json!({"output":[{"type":"message","role":"user","content":[]}]}),
        json!({"output":[{"type":"message","role":"assistant","status":"in_progress","content":[]}]}),
        json!({"output":[{"type":"message","role":"assistant","content":[{"type":"refusal","refusal":42}]}]}),
        json!({"output":[{"type":"function_call","call_id":"c","name":"f","arguments":"{"}]}),
        json!({"output":[{"type":"function_call","call_id":"c","name":"f","arguments":"{}","status":"incomplete"}]}),
        json!({"usage":{"input_tokens":1,"output_tokens":2,"cache_read_input_tokens":3}}),
        json!({"usage":{"input_tokens":1,"output_tokens":2,"cache_read_input_tokens":-1}}),
    ] {
        let mut body = response();
        body.as_object_mut().unwrap().extend(patch.as_object().unwrap().clone());
        assert_eq!(from_responses(&body, "m"), Err(StatusCode::BAD_GATEWAY), "{patch}");
    }
    assert_eq!(from_responses(&json!(null), "m"), Err(StatusCode::BAD_GATEWAY));
}

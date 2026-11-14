use super::messages_chat_result::from_chat;
use serde_json::{Value, json};

fn completion() -> Value {
    json!({"id":"reply","choices":[{"message":{"role":"assistant","content":"answer"},"finish_reason":"stop"}],"usage":{"prompt_tokens":20,"completion_tokens":3}})
}

#[test]
fn cc_route_039_messages_chat_result_expected_behavior() {
    let mut body = completion();
    body["choices"][0]["message"]["refusal"] = json!("declined");
    body["choices"][0]["finish_reason"] = json!("content_filter");
    let result = from_chat(&json!({"data":body}), "client").unwrap();
    assert_eq!(
        result["content"],
        json!([{"type":"text","text":"answer"},{"type":"text","text":"declined"}])
    );
    assert_eq!(result["stop_reason"], "end_turn");
    assert_eq!(result["model"], "client");
}

#[test]
fn cc_route_039_parts_tools_usage_and_stops() {
    for (reason, expected) in [
        ("stop", "end_turn"),
        ("length", "max_tokens"),
        ("tool_calls", "tool_use"),
        ("function_call", "tool_use"),
    ] {
        let mut body = completion();
        body["choices"][0]["finish_reason"] = json!(reason);
        body["choices"][0]["message"]["content"] = json!([
            {"type":"output_text","text":"first"},{"type":"refusal","refusal":"second"}]);
        body["choices"][0]["message"]["tool_calls"] = json!([
            {"id":"a","type":"function","function":{"name":"lookup","arguments":"{\"x\":1}"}},
            {"id":"b","type":"function","function":{"name":"next","arguments":"{}"}}]);
        body["usage"] = json!({"prompt_tokens":20,"completion_tokens":3,"cache_read_input_tokens":0,
            "prompt_tokens_details":{"cached_tokens":8,"cache_write_tokens":2}});
        let result = from_chat(&body, "client").unwrap();
        assert_eq!(result["stop_reason"], expected);
        assert_eq!(
            result["usage"],
            json!({"input_tokens":10,"output_tokens":3,"cache_read_input_tokens":8,"cache_creation_input_tokens":2})
        );
        assert_eq!(result["content"].as_array().unwrap().len(), 4);
        assert_eq!(
            result["content"][2],
            json!({"type":"tool_use","id":"a","name":"lookup","input":{"x":1}})
        );
        assert_eq!(result["content"][3]["id"], "b");
    }
    let mut body = completion();
    body["choices"][0]["message"]["function_call"] =
        json!({"id":"legacy","name":"lookup","arguments":{"x":1}});
    body["choices"][0]["finish_reason"] = json!("function_call");
    assert_eq!(from_chat(&body, "m").unwrap()["content"][1]["input"], json!({"x":1}));
    let wrapped =
        json!({"data":body,"id":"outer","usage":{"prompt_tokens":4,"completion_tokens":2}});
    assert_eq!(from_chat(&wrapped, "m").unwrap()["id"], "outer");
}

#[test]
fn cc_route_039_malformed_fails_and_session_stays_strict() {
    assert_eq!(from_chat(&json!(42), "m"), Err(axum::http::StatusCode::BAD_GATEWAY));
    let mut refusal = completion();
    refusal["choices"][0]["message"]["refusal"] = json!("no");
    assert!(super::messages::from_chat(&refusal, "m").is_err());
    for patch in [
        json!({"choices":[]}),
        json!({"choices":[{"message":{"role":"assistant","content":42},"finish_reason":"stop"}]}),
        json!({"choices":[{"message":{"role":"assistant","content":[{"type":"image"}]},"finish_reason":"stop"}]}),
        json!({"choices":[{"message":{"role":"assistant","tool_calls":[{"id":"x","type":"function","function":{"name":"f","arguments":"{"}}]},"finish_reason":"tool_calls"}]}),
        json!({"choices":[{"message":{"role":"assistant","refusal":false},"finish_reason":"stop"}]}),
        json!({"choices":[{"message":{"role":"assistant"},"finish_reason":"unknown"}]}),
        json!({"usage":{"prompt_tokens":2,"completion_tokens":1,"cache_read_input_tokens":3}}),
        json!({"error":{"message":"bad"}}),
    ] {
        let mut body = completion();
        body.as_object_mut().unwrap().extend(patch.as_object().unwrap().clone());
        assert_eq!(from_chat(&body, "m"), Err(axum::http::StatusCode::BAD_GATEWAY));
    }
}

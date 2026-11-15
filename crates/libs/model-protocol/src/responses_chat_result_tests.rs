use super::{responses::ChatTools, responses_chat_result::from_chat};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"id":"chat_fixture","created":12,"choices":[{"message":{
        "role":"assistant","content":"hello"},"finish_reason":"stop"}],
        "usage":{"prompt_tokens":4,"completion_tokens":2,
            "prompt_tokens_details":{"cached_tokens":1}}})
}

#[test]
fn cc_route_053_responses_chat_result_expected_behavior() {
    let mut body = fixture();
    body["choices"][0]["message"]["refusal"] = json!("declined");
    let result = from_chat(&body, "published", "resp_fixture", &ChatTools::default()).unwrap();
    assert_eq!(result["output"][0]["content"][0]["text"], "hello");
    assert_eq!(result["output"][0]["content"][1]["refusal"], "declined");
    assert_eq!(result["status"], "completed");
    assert_eq!(result["model"], "published");
    assert_eq!(result["created_at"], 12);
    assert_eq!(result["usage"]["total_tokens"], 6);
    assert_eq!(result["usage"]["input_tokens_details"]["cached_tokens"], 1);
}

#[test]
fn cc_route_053_legacy_call_and_length_preserve_arguments() {
    let mut body = fixture();
    body["choices"][0]["message"]["function_call"] =
        json!({"name":"lookup","arguments":"{\"n\": 9007199254740993}"});
    body["choices"][0]["finish_reason"] = json!("length");
    let result = from_chat(&body, "published", "resp_fixture", &ChatTools::default()).unwrap();
    assert_eq!(result["status"], "incomplete");
    assert_eq!(result["incomplete_details"]["reason"], "max_output_tokens");
    assert_eq!(result["output"][1]["call_id"], "call_0");
    assert_eq!(result["output"][1]["arguments"], "{\"n\": 9007199254740993}");
}

#[test]
fn cc_route_053_custom_tool_restores_request_context() {
    let (chat, tools) = super::responses_chat_request::convert(json!({
        "model":"published","input":"hi","tools":[{"type":"custom","name":"patch"}]
    }))
    .unwrap();
    let mut body = fixture();
    body["choices"][0]["message"]["content"] = Value::Null;
    body["choices"][0]["message"]["tool_calls"] = json!([{"id":"call_fixture",
        "type":"function","function":{"name":chat["tools"][0]["function"]["name"],
            "arguments":"{\"input\":\"original patch\"}"}}]);
    body["choices"][0]["finish_reason"] = json!("tool_calls");
    let result = from_chat(&body, "published", "resp_fixture", &tools).unwrap();
    assert_eq!(result["output"][0]["type"], "custom_tool_call");
    assert_eq!(result["output"][0]["input"], "original patch");
}

#[test]
fn cc_route_053_malformed_terminal_is_rejected() {
    let mut body = fixture();
    body["choices"][0]["finish_reason"] = Value::Null;
    assert!(from_chat(&body, "published", "resp_fixture", &ChatTools::default()).is_err());
}

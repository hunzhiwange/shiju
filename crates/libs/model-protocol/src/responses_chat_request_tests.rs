use super::responses_chat_request::convert;
use serde_json::json;

#[test]
fn cc_route_051_responses_chat_request_expected_behavior() {
    let (chat, _) = convert(json!({"model":"test","instructions":"Be concise",
        "input":"hello","reasoning":{"effort":"high"}})).unwrap();
    assert_eq!(chat["messages"], json!([
        {"role":"system","content":"Be concise"}, {"role":"user","content":"hello"}
    ]));
    assert_eq!(chat["reasoning_effort"], "high");
}

#[test]
fn cc_route_051_orphan_does_not_interrupt_parallel_outputs() {
    let (chat, _) = convert(json!({"model":"test","input":[
        {"type":"function_call","call_id":"a","name":"lookup","arguments":"{}"},
        {"type":"function_call","call_id":"b","name":"lookup","arguments":"{}"},
        {"type":"function_call_output","call_id":"a","output":"first"},
        {"type":"function_call_output","output":"orphan"},
        {"type":"function_call_output","call_id":"b","output":"second"}
    ]})).unwrap();
    let messages = chat["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[0]["tool_calls"].as_array().unwrap().len(), 2);
    assert_eq!(messages[1]["tool_call_id"], "a");
    assert_eq!(messages[2]["tool_call_id"], "b");
    assert_eq!(messages[3]["role"], "user");
    assert!(messages[3]["content"].as_str().unwrap().contains("orphan"));
}

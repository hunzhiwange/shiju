use super::*;
use serde_json::json;
use vw_shared::provider::ModelProtocol as P;

fn turn() -> Value {
    json!({"model":"bound-api","max_tokens":128,"stream":true,"stream_options":{"include_usage":true},"messages":[
        {"role":"system","content":"系统"},{"role":"user","content":"开始"},
        {"role":"assistant","content":"调用","tool_calls":[{"id":"c1","type":"function","function":{"name":"echo","arguments":"{\"text\":\"中文\"}"}}]},
        {"role":"tool","tool_call_id":"c1","content":"结果"},{"role":"user","content":"继续"}],
        "tools":[{"type":"function","function":{"name":"echo","description":"示例","parameters":{"type":"object","properties":{"text":{"type":"string"}}}}}],"tool_choice":"auto"})
}

#[test]
fn model_protocol_023_messages_request_preserves_tool_history_and_system() {
    let v = request(P::Messages, &turn()).unwrap();
    assert_eq!(v["system"], "系统");
    assert_eq!(v["messages"][1]["content"][1]["id"], "c1");
    assert_eq!(v["messages"][2]["content"][0]["tool_use_id"], "c1");
    assert_eq!(v["tools"][0]["input_schema"]["type"], "object");
    assert_eq!(v["stream"], true);
}
#[test]
fn model_protocol_023_responses_request_preserves_tool_history_and_no_storage() {
    let v = request(P::Responses, &turn()).unwrap();
    assert_eq!(v["store"], false);
    assert_eq!(v["input"][3]["call_id"], "c1");
    assert_eq!(v["input"][4]["output"], "结果");
    assert_eq!(v["tools"][0]["name"], "echo");
    assert_eq!(v["max_output_tokens"], 128);
}
#[test]
fn model_protocol_023_gemini_request_preserves_tool_ids_and_results() {
    let v = request(P::Gemini, &turn()).unwrap();
    assert_eq!(v["systemInstruction"]["parts"][0]["text"], "系统");
    assert_eq!(v["contents"][1]["parts"][1]["functionCall"]["id"], "c1");
    assert_eq!(v["contents"][2]["parts"][0]["functionResponse"]["name"], "echo");
    assert_eq!(v["generationConfig"]["maxOutputTokens"], 128);
}
#[test]
fn model_protocol_023_messages_response_preserves_text_tools_usage() {
    let v=response(P::Messages,&json!({"id":"m1","role":"assistant","type":"message","model":"upstream","content":[{"type":"text","text":"完成"},{"type":"tool_use","id":"c1","name":"echo","input":{"a":1}}],"stop_reason":"tool_use","usage":{"input_tokens":2,"output_tokens":3}})).unwrap();
    assert_eq!(v["choices"][0]["message"]["content"], "完成");
    assert_eq!(v["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(v["usage"]["total_tokens"], 5);
}
#[test]
fn model_protocol_023_responses_response_preserves_text_tools_and_length() {
    let v=response(P::Responses,&json!({"id":"r1","model":"upstream","status":"incomplete","incomplete_details":{"reason":"max_output_tokens"},"output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"文本"}]},{"type":"function_call","call_id":"c1","name":"echo","arguments":"{}"}],"usage":{"input_tokens":2,"output_tokens":3,"total_tokens":5}})).unwrap();
    assert_eq!(v["choices"][0]["message"]["tool_calls"][0]["id"], "c1");
    assert_eq!(v["choices"][0]["finish_reason"], "length");
}
#[test]
fn model_protocol_023_gemini_response_preserves_call_id_and_usage() {
    let v=response(P::Gemini,&json!({"responseId":"g1","modelVersion":"upstream","candidates":[{"index":0,"content":{"role":"model","parts":[{"text":"文字"},{"functionCall":{"id":"c1","name":"echo","args":{"n":1}}}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":2,"candidatesTokenCount":3,"totalTokenCount":5}})).unwrap();
    assert_eq!(v["choices"][0]["message"]["tool_calls"][0]["id"], "c1");
    assert_eq!(v["usage"]["total_tokens"], 5);
}
#[test]
fn model_protocol_023_requests_reject_unrepresentable_fields_and_orphan_results() {
    for target in [P::Messages, P::Responses, P::Gemini] {
        let mut v = turn();
        v["unknown_vendor_extension"] = true.into();
        assert!(request(target, &v).is_err());
        let mut v = turn();
        v["messages"][3]["tool_call_id"] = "foreign".into();
        assert!(request(target, &v).is_err());
        let mut v = turn();
        v["messages"][2]["reasoning_content"] = "private".into();
        assert!(request(target, &v).is_err());
    }
}
#[test]
fn model_protocol_023_responses_reject_failed_status_and_opaque_thoughts() {
    assert!(
        response(P::Responses, &json!({"status":"failed","output":[],"error":{"code":"x"}}))
            .is_err()
    );
    assert!(response(P::Gemini,&json!({"candidates":[{"content":{"parts":[{"text":"secret","thought":true}]},"finishReason":"STOP"}]})).is_err());
}

#[test]
fn model_protocol_023_explicit_disabled_thinking_is_forwarded_to_each_target() {
    let mut v = turn();
    v["reasoning_effort"] = "none".into();
    assert_eq!(request(P::Messages, &v).unwrap()["thinking"]["type"], "disabled");
    assert_eq!(request(P::Responses, &v).unwrap()["reasoning"]["effort"], "none");
    assert_eq!(
        request(P::Gemini, &v).unwrap()["generationConfig"]["thinkingConfig"]["thinkingBudget"],
        0
    );
    v["reasoning_effort"] = "high".into();
    for p in [P::Messages, P::Responses, P::Gemini] {
        assert!(request(p, &v).is_err());
    }
}

#[test]
fn model_protocol_023_json_schema_is_forwarded_without_rewriting_properties() {
    let mut v = turn();
    let schema = json!({"type":"object","properties":{"值":{"type":"string"}},"required":["值"],"additionalProperties":false});
    v["response_format"] =
        json!({"type":"json_schema","json_schema":{"name":"result","strict":true,"schema":schema}});
    assert_eq!(request(P::Messages, &v).unwrap()["output_config"]["format"]["schema"], schema);
    assert_eq!(request(P::Responses, &v).unwrap()["text"]["format"]["schema"], schema);
    assert_eq!(request(P::Gemini, &v).unwrap()["generationConfig"]["responseJsonSchema"], schema);
}
#[test]
fn model_protocol_023_default_thinking_is_not_silently_disabled() {
    assert!(request(P::Messages, &turn()).unwrap().get("thinking").is_none());
    assert!(request(P::Responses, &turn()).unwrap().get("reasoning").is_none());
    assert!(
        request(P::Gemini, &turn()).unwrap()["generationConfig"].get("thinkingConfig").is_none()
    );
}

#[test]
fn model_protocol_023_empty_optional_chat_state_is_not_opaque_thinking() {
    for p in [P::Messages, P::Responses, P::Gemini] {
        let mut v = turn();
        v["messages"][1]["tool_calls"] = Value::Null;
        v["messages"][2]["reasoning_content"] = "".into();
        v["messages"][2]["refusal"] = Value::Null;
        assert!(request(p, &v).is_ok());
    }
}

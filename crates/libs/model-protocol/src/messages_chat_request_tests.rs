use super::messages_chat_request::to_chat;
use serde_json::json;

#[test]
fn cc_route_038_messages_chat_request_expected_behavior() {
    let body = json!({"model":"m","max_tokens":32,"system":[
        {"type":"text","text":"x-anthropic-billing-header: dynamic\n\nFirst"},
        {"type":"text","text":"Second"}],
        "messages":[{"role":"user","content":"hello"}]});
    let output = to_chat(&body).unwrap();
    assert_eq!(output["messages"][0]["content"], "First\nSecond");
    assert_eq!(
        super::messages::to_chat(&body).unwrap()["messages"][0]["content"],
        "x-anthropic-billing-header: dynamic\n\nFirstSecond"
    );
}

#[test]
fn cc_route_038_choices_schema_and_multimodal() {
    for (choice, expected) in [("auto", "auto"), ("any", "required"), ("none", "none")] {
        let body = json!({"model":"m","max_tokens":32,"tool_choice":choice,
            "tools":[{"name":"lookup","input_schema":{"properties":{
                "url":{"type":"string","format":"uri"},
                "list":{"type":"array","items":{"format":"uri"}}}}}],
            "messages":[{"role":"user","content":[{"type":"text","text":"see"},
                {"type":"image","source":{"type":"base64","media_type":"image/png","data":"aGk="}}]}]});
        let output = to_chat(&body).unwrap();
        assert_eq!(output["tool_choice"], expected);
        assert_eq!(
            output["tools"][0]["function"]["parameters"],
            json!({"type":"object","properties":{
            "url":{"type":"string"},"list":{"type":"array","items":{}}}})
        );
        assert_eq!(
            output["messages"][0]["content"][1]["image_url"]["url"],
            "data:image/png;base64,aGk="
        );
        assert!(super::messages::to_chat(&body).is_err());
    }
}

#[test]
fn cc_route_038_tools_and_rejections() {
    let body = json!({"model":"m","max_tokens":32,"tool_choice":{"type":"tool","name":"lookup","disable_parallel_tool_use":true},
        "messages":[{"role":"assistant","content":[{"type":"tool_use","id":"c","name":"lookup","input":{"x":1}}]},
            {"role":"user","content":[{"type":"tool_result","tool_use_id":"c","content":"ok"}]}]});
    let output = to_chat(&body).unwrap();
    assert_eq!(output["parallel_tool_calls"], false);
    assert_eq!(output["tool_choice"], json!({"type":"function","function":{"name":"lookup"}}));
    assert_eq!(output["messages"][0]["tool_calls"][0]["function"]["arguments"], "{\"x\":1}");
    assert_eq!(output["messages"][1]["tool_call_id"], "c");
    for patch in [
        json!({"tool_choice":"unknown"}),
        json!({"authorization":"fixture"}),
        json!({"messages":[{"role":"user","content":[{"type":"document"}]}]}),
        json!({"system":[{"type":"text","text":"ok","unknown":true}]}),
    ] {
        let mut invalid = body.clone();
        invalid.as_object_mut().unwrap().extend(patch.as_object().unwrap().clone());
        assert!(to_chat(&invalid).is_err());
    }
}

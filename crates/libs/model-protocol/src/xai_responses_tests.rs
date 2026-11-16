use super::xai_responses::prepare;
use serde_json::json;

#[test]
fn cc_route_063_xai_responses_expected_behavior() {
    let mut body = json!({
        "model":"xai/Grok-4.5", "prompt_cache_retention":"24h", "safety_identifier":"client",
        "presence_penalty":0.5, "frequencyPenalty":1, "stop":["end"],
        "include":["output.encrypted_content", "message.output_text.logprobs"],
        "input":[{"type":"reasoning", "content":null, "summary":[]},
            {"type":"additional_tools", "tools":[{"type":"web_search", "external_web_access":true}]}],
        "tools":[{"type":"tool_search"}, {"type":"function", "name":"run", "parameters":{"type":"object"}}],
        "tool_choice":{"type":"tool_search"}
    });
    let url = reqwest::Url::parse("https://api.x.ai/v1/responses").unwrap();
    prepare(&mut body, &url);
    for field in [
        "prompt_cache_retention",
        "safety_identifier",
        "presence_penalty",
        "frequencyPenalty",
        "stop",
        "tool_choice",
    ] {
        assert!(body.get(field).is_none(), "{field}");
    }
    assert_eq!(body["include"], json!(["message.output_text.logprobs"]));
    assert_eq!(body["input"].as_array().unwrap().len(), 1);
    assert!(body["input"][0].get("content").is_none());
    assert_eq!(
        body["tools"],
        json!([
            {"type":"function", "name":"run", "parameters":{"type":"object"}},
            {"type":"web_search"}
        ])
    );
    let once = body.clone();
    prepare(&mut body, &url);
    assert_eq!(body, once);
}

#[test]
fn cc_route_063_other_destinations_preserve_every_field() {
    let original = json!({"model":"grok-4.5", "safety_identifier":"keep",
        "tools":[{"type":"tool_search"}], "external_web_access":true});
    for url in [
        "https://api.openai.com/v1/responses",
        "https://api.x.ai.attacker.test/v1/responses",
        "https://api.x.ai/v1/chat/completions",
        "https://relay.example/api.x.ai/v1/responses",
    ] {
        let mut body = original.clone();
        prepare(&mut body, &reqwest::Url::parse(url).unwrap());
        assert_eq!(body, original, "{url}");
    }
}

#[test]
fn cc_route_063_sampling_and_valid_tool_choice_survive() {
    let mut body = json!({"model":"grok-4.6", "presence_penalty":0.5,
        "tools":[{"type":"function", "name":"run", "parameters":{"type":"object"}}],
        "tool_choice":{"type":"function", "name":"run"}});
    let original = body.clone();
    prepare(&mut body, &reqwest::Url::parse("https://api.x.ai/v1/responses").unwrap());
    assert_eq!(body, original);
}

#[test]
fn cc_route_063_parameter_union_and_empty_include_are_normalized() {
    let mut body = json!({"include":["output.encrypted_content"],
    "tools":[{"type":"function", "name":"run", "parameters":{
        "anyOf":[{"type":"object", "properties":{"x":{"type":"string"}}}, {"type":"null"}]
    }}]});
    prepare(&mut body, &reqwest::Url::parse("https://api.x.ai/v1/responses").unwrap());
    assert!(body.get("include").is_none());
    assert_eq!(
        body["tools"][0]["parameters"],
        json!({"type":"object", "properties":{"x":{"type":"string"}}})
    );
}

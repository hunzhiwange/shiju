use serde_json::json;
#[test]
fn acp_model_source_relay_gemini_rejects_lossy_state_and_ambiguous_tools() {
    use super::gemini::{from_chat, to_chat};
    for part in [
        json!({"text":"x","thought":true}),
        json!({"text":"x","thoughtSignature":"signature"}),
        json!({"fileData":{"fileUri":"private"}}),
        json!({"executableCode":{"code":"x"}}),
    ] {
        assert!(
            to_chat(&json!({"model":"selected","contents":[{"role":"user","parts":[part]}]}))
                .is_err()
        );
    }
    assert!(to_chat(&json!({"model":"selected","contents":[{"role":"model","parts":[{"functionCall":{"name":"f","args":{}}},{"functionCall":{"name":"f","args":{}}}]},{"role":"user","parts":[{"functionResponse":{"name":"f","response":{}}}]}]})).is_err());
    let base = json!({"choices":[{"message":{"role":"assistant","content":"x"},"finish_reason":"length"}],"usage":{"prompt_tokens":3,"completion_tokens":2,"total_tokens":5}});
    assert_eq!(
        from_chat(&base, "selected").unwrap()["candidates"][0]["finishReason"],
        "MAX_TOKENS"
    );
    for extra in [
        json!({"reasoning_content":"private"}),
        json!({"content":[{"type":"image_url"}]}),
        json!({"annotations":[{"url":"private"}]}),
        json!({"refusal":"no"}),
    ] {
        let mut bad = base.clone();
        bad["choices"][0]["message"]
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        assert!(from_chat(&bad, "selected").is_err());
    }
}

#[test]
fn acp_model_source_relay_gemini_matches_idless_tools_and_preserves_reasoning_usage() {
    let request = json!({"model":"selected","contents":[{"role":"model","parts":[{"functionCall":{"name":"f"}}]},{"role":"user","parts":[{"functionResponse":{"name":"f","response":{"ok":true}}}]}]});
    let chat = super::gemini::to_chat(&request).unwrap();
    assert_eq!(chat["messages"][0]["tool_calls"][0]["id"], chat["messages"][1]["tool_call_id"]);
    assert_eq!(chat["messages"][0]["tool_calls"][0]["function"]["arguments"], "{}");
    let response=super::gemini::from_chat(&json!({"choices":[{"message":{"role":"assistant","content":"x"},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":5,"total_tokens":8,"prompt_tokens_details":{"cached_tokens":2},"completion_tokens_details":{"reasoning_tokens":2}}}),"selected").unwrap();
    assert_eq!(
        response["usageMetadata"],
        json!({"promptTokenCount":3,"candidatesTokenCount":3,"totalTokenCount":8,"cachedContentTokenCount":2,"thoughtsTokenCount":2})
    );
}

#[test]
fn model_protocol_018_gemini_native_requirements_are_selected_explicitly() {
    use provider_protocol_routes::RouteCapability as C;
    for (extra, capability) in [
        (json!({"generationConfig":{"thinkingConfig":{"thinkingBudget":1024}}}), C::Thinking),
        (json!({"generationConfig":{"responseMimeType":"application/json"}}), C::StructuredOutput),
        (
            json!({"contents":[{"role":"model","parts":[{"text":"thought","thoughtSignature":"signed"}]}]}),
            C::ThinkingReplay,
        ),
        (
            json!({"contents":[{"parts":[{"inlineData":{"mimeType":"image/png","data":"AAAA"}}]}]}),
            C::Image,
        ),
        (
            json!({"contents":[{"parts":[{"inlineData":{"mimeType":"application/pdf","data":"AAAA"}}]}]}),
            C::Pdf,
        ),
        (
            json!({"contents":[{"role":"model","parts":[{"functionCall":{"name":"lookup","args":{}}}]}]}),
            C::Tools,
        ),
    ] {
        assert!(super::routing::required(&extra, false).contains(&capability), "{extra}");
    }
}

#[test]
fn model_protocol_018_gemini_tool_turn_preserves_text_after_call() {
    let body = json!({"model":"selected","contents":[
        {"role":"model","parts":[{"functionCall":{"id":"one","name":"lookup","args":{}}},{"text":"checking"}]},
        {"role":"user","parts":[{"functionResponse":{"id":"one","name":"lookup","response":{"ok":true}}}]}
    ]});
    let chat = super::gemini::to_chat(&body).unwrap();
    assert_eq!(chat["messages"][0]["content"], "checking");
    assert_eq!(chat["messages"][1]["tool_call_id"], "one");
    let mut unfinished = body;
    unfinished["contents"][1] =
        json!({"role":"model","parts":[{"functionCall":{"id":"two","name":"lookup","args":{}}}]});
    assert!(super::gemini::to_chat(&unfinished).is_err());
}

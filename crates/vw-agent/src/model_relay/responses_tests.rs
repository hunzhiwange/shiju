use super::*;
use serde_json::json;

#[test]
fn acp_model_source_relay_responses_parallel_tools_roles_and_structured_output() {
    let request = json!({"model":"selected","instructions":"system","input":[
        {"role":"developer","content":"policy"},
        {"role":"assistant","content":[{"type":"output_text","text":"checking","annotations":[]}]},
        {"type":"function_call","call_id":"a","name":"one","arguments":"{ \"n\": 10000000000000000000 }"},
        {"type":"function_call","call_id":"b","name":"two","arguments":"{}"},
        {"type":"function_call_output","call_id":"a","output":"first"},
        {"type":"function_call_output","call_id":"b","output":"second"}
    ],"text":{"format":{"type":"json_schema","name":"answer","strict":true,"schema":{"type":"object"}}},"tool_choice":{"type":"function","name":"one"},"parallel_tool_calls":true});
    let chat = super::responses::to_chat(&request).unwrap();
    assert_eq!(chat["messages"][0], json!({"role":"system","content":"system"}));
    assert_eq!(chat["messages"][1], json!({"role":"developer","content":"policy"}));
    assert_eq!(chat["messages"][2]["tool_calls"].as_array().unwrap().len(), 2);
    assert_eq!(chat["messages"][2]["content"], "checking");
    assert_eq!(chat["messages"][3]["tool_call_id"], "a");
    assert_eq!(chat["messages"][4]["tool_call_id"], "b");
    assert_eq!(chat["response_format"]["json_schema"]["name"], "answer");
    assert_eq!(chat["tool_choice"]["function"]["name"], "one");
    let response = json!({"id":"upstream","choices":[{"message":chat["messages"][2],"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":11,"completion_tokens":4,"total_tokens":15,"prompt_tokens_details":{"cached_tokens":8},"completion_tokens_details":{"reasoning_tokens":2}}});
    let converted = super::responses::from_chat(&response, "selected", "resp-local").unwrap();
    assert_eq!(converted["output"][1]["arguments"], "{ \"n\": 10000000000000000000 }");
    assert_eq!(converted["usage"]["input_tokens_details"]["cached_tokens"], 8);
    assert_eq!(converted["usage"]["output_tokens_details"]["reasoning_tokens"], 2);
}

#[test]
fn acp_model_source_relay_responses_rejects_lossy_requests_and_outputs() {
    use axum::http::StatusCode;
    for extra in [
        json!({"store":true}),
        json!({"previous_response_id":"unknown"}),
        json!({"conversation":"elsewhere"}),
        json!({"reasoning":{"effort":"high"}}),
        json!({"input":[{"type":"item_reference","id":"elsewhere"}]}),
        json!({"input":[{"role":"assistant","content":[{"type":"output_text","text":"quote","annotations":[{"type":"url_citation"}]}]}]}),
    ] {
        let mut body = json!({"model":"selected","input":"hello"});
        body.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        assert_eq!(super::responses::to_chat(&body), Err(StatusCode::UNPROCESSABLE_ENTITY));
    }
    for key in ["reasoning_content", "annotations", "audio"] {
        let mut response = json!({"id":"upstream","choices":[{"message":{"role":"assistant","content":"answer"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1}});
        response["choices"][0]["message"][key] = json!({"unrepresentable":true});
        assert_eq!(
            super::responses::from_chat(&response, "selected", "resp-local"),
            Err(StatusCode::UNPROCESSABLE_ENTITY)
        );
    }
}

#[test]
fn acp_model_source_relay_responses_references_bind_every_identity_dimension() {
    let references = super::responses::References::default();
    let id = RelayIdentity {
        tenant: vw_api_types::id::TenantScope::new("responses", "one").unwrap(),
        project: "p".into(),
        session: "s".into(),
        provider: "provider".into(),
        model: "model".into(),
        generation: 1,
    };
    references.remember(&id, "local", "upstream").unwrap();
    assert_eq!(references.resolve(&id, "local").unwrap(), "upstream");
    assert!(references.resolve(&id, "upstream").is_err());
    for dimension in 0..6 {
        let mut other = id.clone();
        match dimension {
            0 => other.tenant = vw_api_types::id::TenantScope::new("responses", "two").unwrap(),
            1 => other.project = "other".into(),
            2 => other.session = "other".into(),
            3 => other.provider = "other".into(),
            4 => other.model = "other".into(),
            _ => other.generation = 2,
        }
        assert_eq!(references.resolve(&other, "local"), Err(axum::http::StatusCode::FORBIDDEN));
    }
}

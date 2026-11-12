use super::*;
use crate::app_routing::reference_scope::{ReferenceScope, References};
use axum::http::StatusCode;
use serde_json::json;

#[test]
fn aggregation_t25_codex_chat_replays_summary_and_isolates_foreign_ciphertext() {
    let request = json!({"model":"selected","input":[
        {"type":"compaction","id":"cmp_local","encrypted_content":"ccswitch-compaction-v1:5bey5L-d5a2Y"},
        {"type":"compaction_summary","id":"cmp_foreign","encrypted_content":"opaque-foreign"},
        {"type":"context_compaction"},
        {"role":"user","content":"continue"}
    ]});

    let (chat, _) = super::responses::to_chat(&request, true).unwrap();
    let messages = chat["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0]["role"], "user");
    assert!(messages[0]["content"].as_str().unwrap().ends_with("\n已保存"));
    assert_eq!(
        messages[1],
        json!({
            "role":"user",
            "content":"[earlier conversation was compacted; the summary is stored in a format this model cannot read]"
        })
    );
    assert_eq!(messages[2], json!({"role":"user","content":"continue"}));
    assert!(!chat.to_string().contains("opaque-foreign"));
    assert_eq!(super::responses::to_chat(&request, false), Err(StatusCode::UNPROCESSABLE_ENTITY));
}

#[test]
fn acp_model_source_relay_responses_codex_replays_namespaced_and_custom_tools() {
    let body = json!({
        "model": "selected",
        "input": [
            {"role": "user", "content": "edit the file"},
            {"type": "function_call", "namespace": "workspace", "name": "read", "call_id": "read-1", "arguments": "{}"},
            {"type": "custom_tool_call", "name": "apply_patch", "call_id": "patch-1", "input": "*** Begin Patch\n*** End Patch"},
            {"type": "function_call_output", "call_id": "read-1", "output": "old text"},
            {"type": "custom_tool_call_output", "call_id": "patch-1", "output": "done"}
        ],
        "tools": [
            {"type": "namespace", "name": "workspace", "tools": [{"type": "function", "name": "read", "parameters": {"type": "object"}}]},
            {"type": "custom", "name": "apply_patch", "format": {"type": "text"}}
        ]
    });
    let (chat, tools) = super::responses::to_chat(&body, false).unwrap();
    let calls = &chat["messages"][1]["tool_calls"];
    assert_eq!(calls[0]["function"]["name"], chat["tools"][0]["function"]["name"]);
    assert_eq!(calls[1]["function"]["arguments"], r#"{"input":"*** Begin Patch\n*** End Patch"}"#);
    assert_eq!(
        chat["messages"][3],
        json!({"role":"tool","tool_call_id":"patch-1","content":"done"})
    );
    let response = json!({"id":"upstream","choices":[{"message":chat["messages"][1],"finish_reason":"tool_calls"}],
        "usage":{"prompt_tokens":5,"completion_tokens":3}});
    let converted =
        super::responses::from_chat(&response, "selected", "resp-local", &tools, false).unwrap();
    assert_eq!(converted["output"][0]["namespace"], "workspace");
    assert_eq!(converted["output"][0]["name"], "read");
    assert_eq!(converted["output"][1]["type"], "custom_tool_call");
    assert_eq!(converted["output"][1]["input"], "*** Begin Patch\n*** End Patch");
    assert_eq!(converted["output"][1]["call_id"], "patch-1");
}

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
    let (chat, tools) = super::responses::to_chat(&request, false).unwrap();
    assert_eq!(chat["messages"][0], json!({"role":"system","content":"system"}));
    assert_eq!(chat["messages"][1], json!({"role":"developer","content":"policy"}));
    assert_eq!(chat["messages"][2]["tool_calls"].as_array().unwrap().len(), 2);
    assert_eq!(chat["messages"][2]["content"], "checking");
    assert_eq!(chat["messages"][3]["tool_call_id"], "a");
    assert_eq!(chat["messages"][4]["tool_call_id"], "b");
    assert_eq!(chat["response_format"]["json_schema"]["name"], "answer");
    assert_eq!(chat["tool_choice"]["function"]["name"], "one");
    let response = json!({"id":"upstream","choices":[{"message":chat["messages"][2],"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":11,"completion_tokens":4,"total_tokens":15,"prompt_tokens_details":{"cached_tokens":8},"completion_tokens_details":{"reasoning_tokens":2}}});
    let converted =
        super::responses::from_chat(&response, "selected", "resp-local", &tools, false).unwrap();
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
        assert_eq!(super::responses::to_chat(&body, false), Err(StatusCode::UNPROCESSABLE_ENTITY));
    }
    for key in ["reasoning_content", "annotations", "audio"] {
        let mut response = json!({"id":"upstream","choices":[{"message":{"role":"assistant","content":"answer"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1}});
        response["choices"][0]["message"][key] = json!({"unrepresentable":true});
        assert_eq!(
            super::responses::from_chat(
                &response,
                "selected",
                "resp-local",
                &Default::default(),
                false
            ),
            Err(StatusCode::UNPROCESSABLE_ENTITY)
        );
    }
}

#[test]
fn acp_model_source_relay_responses_references_bind_every_identity_dimension() {
    let references = References::default();
    let id = RelayIdentity {
        tenant: vw_api_types::id::TenantScope::new("responses", "one").unwrap(),
        project: "p".into(),
        session: "s".into(),
        provider: "provider".into(),
        model: "model".into(),
        generation: 1,
    };
    let scope = ReferenceScope::Session(id.clone());
    references.remember(&scope, "local", "upstream").unwrap();
    assert_eq!(references.resolve(&scope, "local").unwrap(), "upstream");
    assert!(references.resolve(&scope, "upstream").is_err());
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
        assert_eq!(
            references.resolve(&ReferenceScope::Session(other), "local"),
            Err(axum::http::StatusCode::FORBIDDEN)
        );
    }
}

fn native_fixture() -> (crate::provider::provider::Model, RelayIdentity) {
    let mut capabilities = vw_shared::provider::state::default_model_capabilities();
    capabilities.reasoning = true;
    capabilities.input.image = true;
    let model = serde_json::from_value(json!({
        "id":"model", "providerID":"provider", "name":"Model", "status":"active", "release_date":"",
        "api":{"id":"wire-model","url":"http://127.0.0.1/v1/responses","adapter":"openai-responses"},
        "capabilities":capabilities, "limit":{"context":4096,"output":1024},
        "cost":{"input":0,"output":0,"cache":{"read":0,"write":0}}
    })).unwrap();
    let identity = RelayIdentity {
        tenant: vw_api_types::id::TenantScope::new("responses", "inline").unwrap(),
        project: "p".into(),
        session: "s".into(),
        provider: "provider".into(),
        model: "model".into(),
        generation: 1,
    };
    (model, identity)
}

#[test]
fn acp_model_source_relay_responses_native_complete_inline_ids_are_not_lookups() {
    let (model, identity) = native_fixture();
    let input = json!([
        {"type":"message","id":"client-developer","role":"developer","content":[{"type":"input_text","text":"policy"}]},
        {"id":"client-user","role":"user","content":"question"},
        {"type":"message","id":"prior-lease-assistant","role":"assistant","phase":"commentary","status":"completed","content":[{"type":"output_text","text":"checking","annotations":[]}]},
        {"type":"function_call","id":"prior-lease-call","call_id":"call-a","name":"lookup","arguments":"{ \"n\": 1 }"},
        {"type":"function_call_output","id":"client-output","call_id":"call-a","output":"found"},
        {"type":"custom_tool_call","id":"prior-lease-custom","call_id":"call-b","name":"apply_patch","input":"patch text"},
        {"type":"custom_tool_call_output","id":"client-custom-output","call_id":"call-b","output":"done"},
        {"type":"message","id":"client-image","role":"user","content":[{"type":"input_image","image_url":"data:image/png;base64,AA==","detail":"auto"}]}
    ]);
    let bytes = serde_json::to_vec(&json!({"model":"model","input":input})).unwrap();
    let result = super::responses::native(&model, &identity, &bytes, &Default::default()).unwrap();
    let mut expected = input;
    for item in expected.as_array_mut().unwrap() {
        item.as_object_mut().unwrap().remove("id");
    }
    assert_eq!(result["input"], expected);
    assert_eq!(result["model"], "wire-model");
}

#[test]
fn acp_model_source_relay_responses_native_real_references_remain_identity_bound() {
    use axum::http::StatusCode;
    let (model, identity) = native_fixture();
    let references = std::sync::Arc::new(References::default());
    super::responses::native_output(json!({"id":"upstream","object":"response","status":"completed",
        "output":[{"type":"reasoning","id":"owned-item","encrypted_content":"opaque","summary":[]}]}),
        &identity, "local", &references, true).unwrap();
    let owned = json!({"model":"model","input":[{"type":"item_reference","id":"owned-item"}],"previous_response_id":"local"});
    let result = super::responses::native(
        &model,
        &identity,
        &serde_json::to_vec(&owned).unwrap(),
        &references,
    )
    .unwrap();
    assert_eq!(result["input"], owned["input"]);
    assert_eq!(result["previous_response_id"], "upstream");
    for dimension in 0..6 {
        let mut other = identity.clone();
        match dimension {
            0 => other.tenant = vw_api_types::id::TenantScope::new("responses", "other").unwrap(),
            1 => other.project = "other".into(),
            2 => other.session = "other".into(),
            3 => other.provider = "other".into(),
            4 => other.model = "other".into(),
            _ => other.generation = 2,
        }
        let mut request = owned.clone();
        request["model"] = other.model.clone().into();
        assert_eq!(
            super::responses::native(
                &model,
                &other,
                &serde_json::to_vec(&request).unwrap(),
                &references
            ),
            Err(StatusCode::FORBIDDEN)
        );
    }
    for item in [
        json!({"type":"item_reference","id":"unknown","role":"user","content":"cannot disguise a reference"}),
        json!({"type":"message","id":"unknown","role":"user"}),
        json!({"type":"reasoning","id":"unknown","encrypted_content":"opaque","summary":[]}),
        json!({"type":"message","id":"unknown","role":"assistant","content":"text","encrypted_content":"opaque"}),
        json!({"type":"message","id":"unknown","role":"assistant","content":[{"type":"output_text","text":"citation","annotations":[{"type":"file_citation","file_id":"foreign"}]}]}),
        json!({"type":"message","id":"unknown","role":"user","content":[{"type":"input_image","file_id":"foreign"}]}),
        json!({"type":"function_call","id":"unknown","name":"lookup","call_id":"a"}),
        json!({"type":"custom_tool_call","id":"unknown","name":"apply_patch","call_id":"b"}),
    ] {
        let bytes = serde_json::to_vec(&json!({"model":"model","input":[item]})).unwrap();
        assert_eq!(
            super::responses::native(&model, &identity, &bytes, &references),
            Err(StatusCode::FORBIDDEN)
        );
    }
    let forged = json!({"model":"model","input":[{"id":"client","role":"user","content":"hello"}],"previous_response_id":"upstream"});
    assert_eq!(
        super::responses::native(
            &model,
            &identity,
            &serde_json::to_vec(&forged).unwrap(),
            &references
        ),
        Err(StatusCode::FORBIDDEN)
    );
}

#[test]
fn model_protocol_017_plain_reasoning_roundtrip_keeps_one_assistant_turn() {
    let upstream = json!({"id":"chat-upstream","choices":[{"message":{"role":"assistant","content":"checking","reasoning_content":"inspect","tool_calls":[{"id":"call-a","type":"function","function":{"name":"lookup","arguments":"{ \"n\": 1 }"}}]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":1,"completion_tokens":2}});
    let converted =
        responses::from_chat(&upstream, "selected", "resp-local", &Default::default(), false)
            .unwrap();
    assert_eq!(converted["output"][0]["type"], "reasoning");
    let (replayed, _) =
        responses::to_chat(&json!({"model":"selected","input":converted["output"]}), false)
            .unwrap();
    assert_eq!(replayed["messages"].as_array().unwrap().len(), 1);
    assert_eq!(replayed["messages"][0], upstream["choices"][0]["message"]);
}

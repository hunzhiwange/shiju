//! Shared request behavior assertions against the exact production source modules.
use super::{chat, messages};
use axum::http::StatusCode;
use serde_json::{Value, json};

fn request() -> Value {
    json!({"model":"selected","max_tokens":64,"messages":[{"role":"user","content":"hello"}]})
}

#[test]
fn model_protocol_014_stop_sequences_preserve_unicode_and_reject_overflow() {
    let mut body = request();
    body["stop_sequences"] = json!(["结束", "\nEND"]);
    assert_eq!(messages::to_chat(&body).unwrap()["stop"], body["stop_sequences"]);
    body["stop_sequences"] = json!([]);
    assert!(messages::to_chat(&body).unwrap().get("stop").is_none());
    body["stop_sequences"] = json!(["1", "2", "3", "4", "5"]);
    assert_eq!(messages::to_chat(&body), Err(StatusCode::UNPROCESSABLE_ENTITY));
    for invalid in [json!("END"), json!([1]), json!([""])] {
        body["stop_sequences"] = invalid;
        assert_eq!(messages::to_chat(&body), Err(StatusCode::BAD_REQUEST));
    }
}

#[test]
fn model_protocol_014_structured_schema_is_preserved_without_effort_downgrade() {
    let mut body = request();
    body["output_config"] = json!({"format":{"type":"json_schema","schema":{
        "type":"object","properties":{"answer":{"type":"string"}},
        "required":["answer"],"additionalProperties":false}}});
    let converted = messages::to_chat(&body).unwrap();
    assert_eq!(
        converted["response_format"],
        json!({"type":"json_schema","json_schema":{
        "name":"messages_output","strict":true,"schema":body["output_config"]["format"]["schema"]}})
    );
    body["output_config"]["effort"] = "high".into();
    assert_eq!(messages::to_chat(&body), Err(StatusCode::UNPROCESSABLE_ENTITY));
}

#[test]
fn model_protocol_014_cache_hints_validate_all_scopes_and_do_not_leak_identity() {
    let mut body = request();
    body["cache_control"] = json!({"type":"ephemeral","ttl":"1h"});
    body["system"] = json!([{"type":"text","text":"system","cache_control":{"type":"ephemeral"}}]);
    body["metadata"] = json!({"user_id":"private-fixture"});
    let converted = messages::to_chat(&body).unwrap();
    assert_eq!(converted["messages"][0]["content"], "system");
    assert!(!converted.to_string().contains("cache_control"));
    assert!(!converted.to_string().contains("private-fixture"));
    body["cache_control"]["ttl"] = "forever".into();
    assert_eq!(messages::to_chat(&body), Err(StatusCode::UNPROCESSABLE_ENTITY));
}

#[test]
fn model_protocol_014_tool_choice_and_strict_schema_keep_execution_semantics() {
    let mut body = request();
    body["tools"] = json!([{"name":"lookup","input_schema":{"type":"object","properties":{},"required":[],"additionalProperties":false},"strict":true}]);
    for (source, target) in [("auto", "auto"), ("any", "required"), ("none", "none")] {
        body["tool_choice"] = json!({"type":source,"disable_parallel_tool_use":true});
        let converted = messages::to_chat(&body).unwrap();
        assert_eq!(converted["tool_choice"], target);
        assert_eq!(converted["parallel_tool_calls"], false);
        assert_eq!(converted["tools"][0]["function"]["strict"], true);
    }
    body["tool_choice"] = json!({"type":"tool","name":"lookup","disable_parallel_tool_use":false});
    let converted = messages::to_chat(&body).unwrap();
    assert_eq!(converted["tool_choice"], json!({"type":"function","function":{"name":"lookup"}}));
    assert_eq!(converted["parallel_tool_calls"], true);
    body["tools"][0]["strict"] = "yes".into();
    assert_eq!(messages::to_chat(&body), Err(StatusCode::BAD_REQUEST));
}

#[test]
fn model_protocol_014_images_preserve_order_and_enforce_selected_model() {
    let mut body = request();
    body["messages"][0]["content"] = json!([
        {"type":"text","text":"before"},
        {"type":"image","source":{"type":"base64","media_type":"image/png","data":"aGVsbG8="}},
        {"type":"text","text":"after"},
        {"type":"image","source":{"type":"url","url":"https://example.invalid/image.png"}}
    ]);
    let converted = messages::to_chat(&body).unwrap();
    assert_eq!(
        converted["messages"][0]["content"],
        json!([
            {"type":"text","text":"before"},
            {"type":"image_url","image_url":{"url":"data:image/png;base64,aGVsbG8="}},
            {"type":"text","text":"after"},
            {"type":"image_url","image_url":{"url":"https://example.invalid/image.png"}}
        ])
    );
    let mut capabilities = vw_shared::provider::state::default_model_capabilities();
    capabilities.input.image = true;
    let mut model: crate::provider::provider::Model = serde_json::from_value(json!({
        "id":"selected","providerID":"fixture","name":"Fixture","status":"active","release_date":"",
        "api":{"id":"wire-model","url":"https://example.invalid/v1","adapter":"openai"},
        "capabilities":capabilities,"limit":{"context":4096,"output":1024},
        "cost":{"input":0,"output":0,"cache":{"read":0,"write":0}}
    }))
    .unwrap();
    let bytes = serde_json::to_vec(&converted).unwrap();
    assert_eq!(chat::prepare(&model, "selected", &bytes).unwrap()["model"], "wire-model");
    model.capabilities.input.image = false;
    assert_eq!(chat::prepare(&model, "selected", &bytes), Err(StatusCode::UNPROCESSABLE_ENTITY));
}

#[test]
fn model_protocol_014_unrepresentable_extensions_and_thinking_are_rejected() {
    for (field, value) in [
        ("thinking", json!({"type":"enabled","budget_tokens":1024})),
        ("top_k", json!(40)),
        ("unknown_extension", json!(true)),
        ("output_config", json!({"effort":"high"})),
    ] {
        let mut body = request();
        body[field] = value;
        assert_eq!(messages::to_chat(&body), Err(StatusCode::UNPROCESSABLE_ENTITY));
    }
    for block in [
        json!({"type":"thinking","thinking":"preserve me","signature":"sig"}),
        json!({"type":"redacted_thinking","data":"opaque"}),
        json!({"type":"image","source":{"type":"file","file_id":"native-only"}}),
    ] {
        let mut body = request();
        body["messages"][0]["content"] = json!([block]);
        assert_eq!(messages::to_chat(&body), Err(StatusCode::UNPROCESSABLE_ENTITY));
    }
}

#[test]
fn model_protocol_014_invalid_image_sources_are_rejected_without_dropping_parts() {
    for source in [
        json!({"type":"url","url":"file:///private/image.png"}),
        json!({"type":"url","url":"https://user:secret@example.invalid/image"}),
        json!({"type":"url","url":"https://example.invalid/image","unknown":true}),
        json!({"type":"base64","media_type":"application/pdf","data":"aGVsbG8="}),
        json!({"type":"base64","media_type":"image/png","data":"not base64"}),
    ] {
        let mut body = request();
        body["messages"][0]["content"] = json!([{"type":"image","source":source}]);
        assert!(messages::to_chat(&body).is_err());
    }
}

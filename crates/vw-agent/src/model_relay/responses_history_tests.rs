use super::responses_history::{Context, Store, preserve_invalid};
use crate::{app_routing::reference_scope::ReferenceScope, model_relay::RelayIdentity};
use axum::http::StatusCode;
use serde_json::json;
use std::sync::Arc;

fn scope(generation: u64) -> ReferenceScope {
    ReferenceScope::Session(RelayIdentity {
        tenant: vw_api_types::id::TenantScope::default(),
        project: "project".into(),
        provider: "provider".into(),
        session: "session".into(),
        model: "model".into(),
        generation,
    })
}

#[test]
fn cc_route_055_responses_history_expected_behavior() {
    let store = Arc::new(Store::default());
    let mut request = json!({"input":"inspect"});
    let context = Context::prepare(store.clone(), scope(1), &mut request).unwrap();
    context
        .remember(&json!({"id":"resp-one","status":"completed","output":[
            {"type":"function_call","call_id":"a","name":"first","arguments":"{}"},
            {"type":"function_call","call_id":"b","name":"second","arguments":"{}"}
        ]}))
        .unwrap();
    let mut next = json!({"previous_response_id":"resp-one","input":[
        {"type":"function_call_output","call_id":"a","output":"one"},
        {"type":"function_call_output","call_id":"b","output":"two"}
    ]});
    Context::prepare(store.clone(), scope(1), &mut next).unwrap();
    assert!(next.get("previous_response_id").is_none());
    assert_eq!(next["input"].as_array().unwrap().len(), 5);
    let (chat, _) = super::responses_chat_request::convert(next).unwrap();
    assert_eq!(chat["messages"][1]["tool_calls"].as_array().unwrap().len(), 2);
    let mut foreign = json!({"previous_response_id":"resp-one","input":"hello"});
    assert!(matches!(
        Context::prepare(store.clone(), scope(2), &mut foreign),
        Err(StatusCode::FORBIDDEN)
    ));
    store.clear();
    assert!(matches!(Context::prepare(store, scope(1), &mut foreign), Err(StatusCode::FORBIDDEN)));
}

#[test]
fn cc_route_055_invalid_calls_preserve_arguments_and_results() {
    let mut items = vec![
        json!({"type":"function_call","call_id":"broken","name":"read","arguments":"{broken"}),
        json!({"type":"function_call_output","call_id":"broken","output":"failure evidence"}),
    ];
    preserve_invalid(&mut items);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["role"], "assistant");
    assert!(items[0]["content"].as_str().unwrap().contains("{broken"));
    assert!(items[0]["content"].as_str().unwrap().contains("failure evidence"));
}

use super::messages;
use serde_json::json;
#[test]
fn model_protocol_015_enabled_thinking_is_not_silently_disabled() {
    let body = json!({"model":"selected","max_tokens":4096,
        "thinking":{"type":"enabled","budget_tokens":1024},
        "messages":[{"role":"user","content":"hello"}]});
    assert!(context().request(&body).is_ok(), "expressible thinking must reach the bridge");
    assert_eq!(messages::to_chat(&body), Err(StatusCode::UNPROCESSABLE_ENTITY));
}

use super::{RelayIdentity, messages_thinking::Context};
use crate::app_routing::reference_scope::{ReferenceScope, References};
use axum::http::StatusCode;
use serde_json::Value;
use std::sync::Arc;

fn identity() -> RelayIdentity {
    RelayIdentity {
        tenant: vw_api_types::id::TenantScope::new("thinking", "fixture").unwrap(),
        project: "project".into(),
        session: "session".into(),
        provider: "provider".into(),
        model: "selected".into(),
        generation: 1,
    }
}
fn context() -> Context {
    Context::new(&identity(), "deployment".into(), Arc::new(References::default()))
}
fn response() -> Value {
    json!({"id":"reply","choices":[{"message":{"role":"assistant","content":"answer","reasoning_content":"真实推理",
        "tool_calls":[{"id":"a","type":"function","function":{"name":"first","arguments":"{\"x\":1}"}},
        {"id":"b","type":"function","function":{"name":"second","arguments":"{}"}}]},"finish_reason":"tool_calls"}],
        "usage":{"prompt_tokens":7,"completion_tokens":12}})
}
fn replay(result: &Value) -> Value {
    json!({"model":"selected","max_tokens":4096,"thinking":{"type":"adaptive"},"messages":[
        {"role":"user","content":"hello"},{"role":"assistant","content":result["content"]},
        {"role":"user","content":[{"type":"tool_result","tool_use_id":"a","content":"one"},
        {"type":"tool_result","tool_use_id":"b","content":"two"}]}]})
}

#[test]
fn model_protocol_015_parallel_tools_replay_actual_reasoning_and_results() {
    let context = context();
    let result = context.result(&response(), "selected").unwrap();
    assert_eq!(result["content"][0]["thinking"], "真实推理");
    assert!(result["content"][0]["signature"].as_str().unwrap().starts_with("shiju-bridge-v1:"));
    let converted = context.request(&replay(&result)).unwrap();
    assert_eq!(converted["messages"][1]["reasoning_content"], "真实推理");
    assert_eq!(converted["messages"][1]["tool_calls"][1]["id"], "b");
    assert_eq!(converted["messages"][2]["tool_call_id"], "a");
    assert_eq!(converted["messages"][3]["content"], "two");
    let next = context.result(&response(), "selected").unwrap();
    assert!(context.request(&replay(&next)).is_ok());
}

#[test]
fn model_protocol_015_tampered_reasoning_signature_text_or_tools_are_rejected() {
    let context = context();
    let result = context.result(&response(), "selected").unwrap();
    for (index, field, value) in [
        (0, "thinking", json!("changed")),
        (0, "signature", json!("native")),
        (1, "text", json!("changed")),
        (2, "id", json!("foreign")),
        (3, "input", json!({"changed":true})),
    ] {
        let mut input = replay(&result);
        input["messages"][1]["content"][index][field] = value;
        assert_eq!(context.request(&input), Err(StatusCode::FORBIDDEN));
    }
}

#[test]
fn model_protocol_015_receipts_cannot_cross_identity_deployment_or_backend() {
    let references = Arc::new(References::default());
    let original = identity();
    let context = Context::new(&original, "deployment".into(), references.clone());
    let input = replay(&context.result(&response(), "selected").unwrap());
    for field in ["tenant", "project", "session", "provider", "model", "generation"] {
        let mut other = original.clone();
        match field {
            "tenant" => {
                other.tenant = vw_api_types::id::TenantScope::new("other", "fixture").unwrap()
            }
            "project" => other.project = "other".into(),
            "session" => other.session = "other".into(),
            "provider" => other.provider = "other".into(),
            "model" => other.model = "other".into(),
            _ => other.generation += 1,
        }
        assert_eq!(
            Context::new(&other, "deployment".into(), references.clone()).request(&input),
            Err(StatusCode::FORBIDDEN)
        );
    }
    assert_eq!(
        Context::new(&original, "other-deployment".into(), references).request(&input),
        Err(StatusCode::FORBIDDEN)
    );
    assert_eq!(self::context().request(&input), Err(StatusCode::FORBIDDEN));
}

#[test]
fn model_protocol_015_expired_and_missing_receipts_fail_closed() {
    let refs = Arc::new(References::default());
    refs.remember(&ReferenceScope::Session(identity()), "shiju-bridge-v1:expired", "0:expired")
        .unwrap();
    let context = Context::new(&identity(), "deployment".into(), refs);
    let mut input = replay(&context.result(&response(), "selected").unwrap());
    input["messages"][1]["content"][0]["signature"] = "shiju-bridge-v1:expired".into();
    assert_eq!(context.request(&input), Err(StatusCode::FORBIDDEN));
    input["messages"][1]["content"][0].as_object_mut().unwrap().remove("signature");
    assert_eq!(context.request(&input), Err(StatusCode::FORBIDDEN));
}

#[test]
fn model_protocol_015_missing_thinking_and_foreign_tool_results_are_diagnostic_errors() {
    let context = context();
    let result = context.result(&response(), "selected").unwrap();
    let mut input = replay(&result);
    input["messages"][1]["content"].as_array_mut().unwrap().remove(0);
    assert_eq!(context.request(&input), Err(StatusCode::UNPROCESSABLE_ENTITY));
    let mut input = replay(&result);
    input["messages"][2]["content"][0]["tool_use_id"] = "foreign".into();
    assert_eq!(context.request(&input), Err(StatusCode::UNPROCESSABLE_ENTITY));
}

#[test]
fn model_protocol_015_model_modes_preserve_budgets_or_explicitly_reject_them() {
    use vw_shared::provider::types::{ApiInfo, Model, ModelCost, ModelCostCache, ModelLimit};
    let mut model = Model {
        id: "selected".into(),
        provider_id: "custom".into(),
        api: ApiInfo {
            id: "gpt-5.4".into(),
            url: "https://example.invalid/v1".into(),
            adapter: "openai-compatible".into(),
            protocols: None,
        },
        name: "selected".into(),
        family: None,
        capabilities: vw_shared::provider::state::default_model_capabilities(),
        cost: ModelCost {
            input: 0.0,
            output: 0.0,
            cache: ModelCostCache { read: 0.0, write: 0.0 },
            experimental_over_200k: None,
        },
        limit: ModelLimit { context: 4096, input: None, output: 4096 },
        status: "active".into(),
        options: Default::default(),
        headers: Default::default(),
        release_date: String::new(),
        variants: Default::default(),
    };
    model.provider_id = "custom".into();
    model.api.adapter = "openai-compatible".into();
    model.api.id = "gpt-5.4".into();
    model.api.url = "https://example.invalid/v1".into();
    model.capabilities.reasoning = true;
    let mut body = json!({});
    super::messages_thinking::configure(
        &mut body,
        &json!({"thinking":{"type":"adaptive"}}),
        &model,
    )
    .unwrap();
    assert_eq!(body["reasoning_effort"], "high");
    assert!(
        super::messages_thinking::configure(
            &mut body,
            &json!({"thinking":{"type":"enabled","budget_tokens":1024}}),
            &model
        )
        .is_err()
    );
    model.api.id = "o3".into();
    assert!(
        super::messages_thinking::configure(
            &mut body,
            &json!({"thinking":{"type":"disabled"}}),
            &model
        )
        .is_err()
    );
    model.provider_id = "alibaba".into();
    model.api.id = "qwen3.8-max".into();
    let mut body = json!({});
    super::messages_thinking::configure(
        &mut body,
        &json!({"thinking":{"type":"enabled","budget_tokens":1024}}),
        &model,
    )
    .unwrap();
    assert_eq!(body["enable_thinking"], true);
    assert_eq!(body["thinking_budget"], 1024);
    model.provider_id = "deepseek".into();
    model.api.id = "deepseek-v4-pro".into();
    super::messages_thinking::configure(
        &mut body,
        &json!({"thinking":{"type":"adaptive"}}),
        &model,
    )
    .unwrap();
    assert_eq!(body["thinking"]["type"], "enabled");
}

#[test]
fn model_protocol_015_native_signatures_are_preserved_and_missing_history_is_rejected() {
    let mut body = json!({"thinking":{"type":"adaptive"},"messages":[
        {"role":"user","content":"hello"},
        {"role":"assistant","content":[{"type":"thinking","thinking":"native reasoning","signature":"vendor signature"},
         {"type":"tool_use","id":"a","name":"lookup","input":{}}]},
        {"role":"user","content":[{"type":"tool_result","tool_use_id":"a","content":"done"}]}]});
    let original = body.clone();
    messages::prepare_deepseek_native(&mut body).unwrap();
    assert_eq!(body, original);
    body["messages"][1]["content"].as_array_mut().unwrap().remove(0);
    assert_eq!(messages::prepare_deepseek_native(&mut body), Err(StatusCode::UNPROCESSABLE_ENTITY));
    assert_eq!(body["thinking"]["type"], "adaptive");
}

#[test]
fn model_protocol_015_disabled_tool_round_does_not_require_invented_thinking() {
    let context = context();
    let mut wire = response();
    wire["choices"][0]["message"]["reasoning_content"] = Value::Null;
    let result = context.result(&wire, "selected").unwrap();
    assert_ne!(result["content"][0]["type"], "thinking");
    let mut input = replay(&result);
    input["thinking"] = json!({"type":"disabled"});
    assert!(context.request(&input).is_ok());
    let signed = context.result(&response(), "selected").unwrap();
    let mut input = replay(&signed);
    input.as_object_mut().unwrap().remove("thinking");
    input["messages"][1]["content"].as_array_mut().unwrap().remove(0);
    assert_eq!(context.request(&input), Err(StatusCode::UNPROCESSABLE_ENTITY));
}

#[test]
fn model_protocol_015_effort_and_schema_remain_separate_from_replay() {
    let context = context();
    let mut body = json!({"model":"selected","max_tokens":4096,"thinking":{"type":"adaptive"},
        "output_config":{"effort":"medium","format":{"type":"json_schema","schema":{"type":"object"}}},
        "messages":[{"role":"user","content":"hello"}]});
    let converted = context.request(&body).unwrap();
    assert_eq!(
        converted["response_format"]["json_schema"]["schema"],
        body["output_config"]["format"]["schema"]
    );
    body["output_config"]["effort"] = 3.into();
    assert_eq!(context.request(&body), Err(StatusCode::BAD_REQUEST));
}

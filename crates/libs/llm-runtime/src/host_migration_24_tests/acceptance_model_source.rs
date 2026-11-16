use crate::acp::acceptance_support::model_source::accepted;
use serde_json::json;

#[test]
fn acp_model_source_gemini_harness_requires_exact_model_and_generation_path() {
    let request = json!({"model":"acp-fixture-primary","authenticated":true,"route":"/v1beta/models/acp-fixture-primary:streamGenerateContent","tool_results":2});
    let mut evidence =
        json!({"errors":0,"requests":[request.clone(),request.clone(),request.clone(),request]});
    assert!(accepted(&evidence, "cli-local"));
    evidence["requests"][0]["route"] =
        json!("/v1beta/models/forbidden-native-model:streamGenerateContent");
    assert!(!accepted(&evidence, "cli-local"));
    evidence["requests"][0]["route"] = json!("/v1beta/models/acp-fixture-primary:countTokens");
    assert!(!accepted(&evidence, "cli-local"));
}

#[test]
fn acp_model_source_harness_contract_no_model_request_is_failure() {
    assert!(!accepted(&json!({"handshake": true, "requests": [], "errors": 0}), "session-model"));
}

#[test]
fn acp_model_source_harness_contract_handshake_is_not_model_success() {
    assert!(!accepted(&json!({"handshake": true, "model": "selected"}), "session-model"));
}

#[test]
fn acp_model_source_harness_contract_forged_model_is_failure() {
    assert!(!accepted(
        &json!({"handshake": true, "requests": [{"model": "selected"}], "errors": 0}),
        "session-model"
    ));
}

#[test]
fn acp_model_source_harness_contract_rejects_unauthenticated_or_wrong_model() {
    let request = json!({"model":"acp-fixture-primary", "authenticated":true,
                         "route":"/v1/chat/completions", "tool_results":2});
    let mut evidence =
        json!({"errors":0, "requests":[request.clone(),request.clone(),request.clone(),request]});
    assert!(accepted(&evidence, "cli-local"));
    assert!(!accepted(&evidence, "session-model"));
    evidence["requests"][3]["model"] = json!("acp-fixture-switched");
    assert!(accepted(&evidence, "session-model"));
    evidence["requests"][0]["authenticated"] = json!(false);
    assert!(!accepted(&evidence, "session-model"));
}

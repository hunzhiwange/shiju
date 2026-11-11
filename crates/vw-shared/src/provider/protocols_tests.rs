use super::ApiInfo;
use serde_json::{Value, json};

fn api(protocols: Value) -> Value {
    json!({"id":"model-a", "url":"https://example.test/v1", "protocols": protocols})
}

fn endpoint() -> Value {
    json!({
        "protocol":"messages", "url":"https://example.test/v1/messages",
        "deployment":"example-us-payg",
        "authentication":{"kind":"api_key_header", "name":"x-api-key"},
        "models":{"kind":"models", "ids":["model-a"]},
        "evidence":[{"kind":"official", "source":"research/group-01.json",
                     "reference":"example", "checked_at":"2026-11-11"}]
    })
}

#[test]
fn model_protocol_009_missing_stays_unknown() {
    let parsed: ApiInfo =
        serde_json::from_value(json!({"id":"a", "url":"https://example.test"})).unwrap();
    let serialized = serde_json::to_value(parsed).unwrap();
    assert!(serialized.get("protocols").is_none());
    assert_eq!(serialized["adapter"], "openai-compatible");
}

#[test]
fn model_protocol_009_empty_overrides_inference() {
    let parsed: ApiInfo = serde_json::from_value(api(json!([]))).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap()["protocols"], json!([]));
}

#[test]
fn model_protocol_009_confirmed_endpoint_roundtrips() {
    let value = api(json!([endpoint()]));
    let parsed: ApiInfo = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap()["protocols"], value["protocols"]);
}

#[test]
fn model_protocol_009_unknown_protocol_fails_closed() {
    let mut value = endpoint();
    value["protocol"] = "unknown".into();
    assert!(serde_json::from_value::<ApiInfo>(api(json!([value]))).is_err());
}

#[test]
fn model_protocol_009_unknown_authentication_fails_closed() {
    let mut value = endpoint();
    value["authentication"] = json!({"kind":"unknown"});
    assert!(serde_json::from_value::<ApiInfo>(api(json!([value]))).is_err());
}

#[test]
fn model_protocol_009_empty_model_scope_fails_closed() {
    let mut value = endpoint();
    value["models"]["ids"] = json!([]);
    assert!(serde_json::from_value::<ApiInfo>(api(json!([value]))).is_err());
}

#[test]
fn model_protocol_009_conflicting_deployment_fails_closed() {
    let value = endpoint();
    let mut conflicting = value.clone();
    conflicting["url"] = "https://other.test/messages".into();
    assert!(serde_json::from_value::<ApiInfo>(api(json!([value, conflicting]))).is_err());
}

#[test]
fn model_protocol_009_all_protocols_and_auth_shapes_roundtrip() {
    for protocol in ["chat", "messages", "responses", "gemini"] {
        for authentication in [
            json!({"kind":"none"}),
            json!({"kind":"bearer"}),
            json!({"kind":"api_key_header", "name":"api-key"}),
            json!({"kind":"oauth_bearer"}),
            json!({"kind":"aws_sigv4"}),
            json!({"kind":"unsupported", "reason":"dedicated SDK required"}),
        ] {
            let mut value = endpoint();
            value["protocol"] = protocol.into();
            value["authentication"] = authentication;
            let parsed: ApiInfo = serde_json::from_value(api(json!([value.clone()]))).unwrap();
            assert_eq!(serde_json::to_value(parsed).unwrap()["protocols"][0], value);
        }
    }
}

#[test]
fn model_protocol_009_model_scope_and_deployments_are_exact() {
    let value = endpoint();
    let mut other = value.clone();
    other["deployment"] = "example-eu-plan".into();
    other["url"] = "https://eu.example.test/messages".into();
    let parsed: ApiInfo = serde_json::from_value(api(json!([value, other]))).unwrap();
    let endpoints = parsed.protocols.unwrap();
    assert!(endpoints[0].models.contains("model-a"));
    assert!(!endpoints[0].models.contains("provider/model-a"));
    assert!(!endpoints[0].models.contains("model-b"));
}

#[test]
fn model_protocol_009_missing_evidence_or_scope_is_rejected() {
    for field in ["models", "authentication", "deployment", "evidence"] {
        let mut value = endpoint();
        value.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<ApiInfo>(api(json!([value]))).is_err(), "{field}");
    }
    let mut value = endpoint();
    value["evidence"] = json!([]);
    assert!(serde_json::from_value::<ApiInfo>(api(json!([value]))).is_err());
}

#[test]
fn model_protocol_009_all_scope_conflicts_with_overlapping_models() {
    let value = endpoint();
    let mut other = value.clone();
    other["models"] = json!({"kind":"all"});
    other["authentication"] = json!({"kind":"bearer"});
    assert!(serde_json::from_value::<ApiInfo>(api(json!([value, other]))).is_err());
}

#[test]
fn model_protocol_009_nonoverlapping_models_allow_distinct_endpoints() {
    let value = endpoint();
    let mut other = value.clone();
    other["models"]["ids"] = json!(["model-b"]);
    other["url"] = "https://other.test/messages".into();
    assert!(serde_json::from_value::<ApiInfo>(api(json!([value, other]))).is_ok());
}

#[test]
fn model_protocol_009_authentication_never_accepts_embedded_credentials() {
    for authentication in [
        json!({"kind":"bearer", "key":"fixture-value"}),
        json!({"kind":"api_key_header", "name":""}),
        json!({"kind":"unsupported", "reason":""}),
    ] {
        let mut value = endpoint();
        value["authentication"] = authentication;
        assert!(serde_json::from_value::<ApiInfo>(api(json!([value]))).is_err());
    }
}

#[test]
fn model_protocol_009_all_scope_rejects_conflicting_ids() {
    let mut value = endpoint();
    value["models"] = json!({"kind":"all", "ids":["model-a"]});
    assert!(serde_json::from_value::<ApiInfo>(api(json!([value]))).is_err());
}

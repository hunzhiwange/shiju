use super::agent_apps_diagnostics::{
    DiagnosticEndpointCandidatesQuery, DiagnosticEndpointCandidatesResponse, DiagnosticJobSnapshot,
    DiagnosticResult, DiagnosticScopeQuery, DiagnosticStartRequest,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::fmt::Debug;

fn assert_roundtrip<T: DeserializeOwned + Serialize + PartialEq + Debug>(wire: &Value) {
    let decoded = serde_json::from_value::<T>(wire.clone()).unwrap();
    let encoded = serde_json::to_value(&decoded).unwrap();
    assert_eq!(encoded, *wire);
    assert_eq!(serde_json::from_value::<T>(encoded).unwrap(), decoded);
}

fn result(kind: &str, target: Value, source: &str, phase: &str, evidence: Value) -> Value {
    let mut wire = target;
    wire["kind"] = json!(kind);
    wire["report"] = json!({
        "source": source,
        "timings": [{"phase": phase, "elapsedMs": 12}],
        "outcome": {"status": "succeeded", "evidence": evidence}
    });
    wire
}

#[test]
fn diagnostic_dto_distinguishes_probe_kinds() {
    let provider = json!({"providerId": "provider-1"});
    let model = json!({"providerId": "provider-1", "modelId": "model-1", "mayIncurCost": true});
    let reachability = result(
        "reachability",
        provider,
        "real_service",
        "response_headers",
        json!({"httpStatus": 401, "slow": false}),
    );
    let speed = result(
        "speed",
        json!({"endpointId": "candidate-1"}),
        "controlled_fixture",
        "warmup",
        json!({"httpStatus": 500, "latencyMs": 12}),
    );
    let model_response = result(
        "model",
        model.clone(),
        "real_service",
        "model_response",
        json!({"httpStatus": 200, "actualModelId": "model-actual"}),
    );
    let streaming = result(
        "streaming",
        model,
        "controlled_fixture",
        "first_event",
        json!({"httpStatus": 200, "actualModelId": "model-actual", "firstEventMs": 12, "maxEventGapMs": 2}),
    );
    let mcp = result(
        "mcp",
        json!({"serverId": "server-1"}),
        "controlled_fixture",
        "handshake",
        json!({"transport": "stdio"}),
    );

    for wire in [&reachability, &speed, &model_response, &streaming, &mcp] {
        assert_roundtrip::<DiagnosticResult>(wire);
    }
    let mut mixed_evidence = reachability.clone();
    mixed_evidence["report"]["outcome"]["evidence"]["firstEventMs"] = json!(12);
    assert!(
        serde_json::from_value::<DiagnosticResult>(mixed_evidence).is_err(),
        "reachability must reject evidence reserved for streaming probes"
    );
    let mut mislabeled_model = model_response;
    mislabeled_model["kind"] = json!("streaming");
    assert!(
        serde_json::from_value::<DiagnosticResult>(mislabeled_model).is_err(),
        "an HTTP status and a model ID do not establish streaming evidence"
    );

    let mut failed = streaming;
    failed["report"]["outcome"] = json!({
        "status": "failed",
        "error": {"phase": "stream", "code": "invalid_response"},
        "httpStatus": 200
    });
    assert_roundtrip::<DiagnosticResult>(&failed);
    failed["report"]["outcome"]["error"]["rawError"] = json!("upstream fixture error");
    assert!(serde_json::from_value::<DiagnosticResult>(failed).is_err());

    let mut cancelled = mcp;
    cancelled["report"]["outcome"] = json!({"status": "cancelled"});
    assert_roundtrip::<DiagnosticResult>(&cancelled);
    cancelled["report"]["outcome"]["evidence"] = json!({"transport": "stdio"});
    assert!(serde_json::from_value::<DiagnosticResult>(cancelled).is_err());

    let requests = [
        json!({"kind": "reachability", "providerId": "provider-1"}),
        json!({"kind": "speed", "providerId": "provider-1", "endpointIds": ["candidate-1"]}),
        json!({"kind": "model", "providerId": "provider-1", "modelId": "model-1"}),
        json!({"kind": "streaming", "providerId": "provider-1", "modelId": "model-1"}),
        json!({"kind": "mcp", "serverIds": ["server-1"]}),
    ];
    for request in requests {
        assert_roundtrip::<DiagnosticStartRequest>(&request);
        for forbidden in ["url", "accountId", "targetHostId", "command", "headers"] {
            let mut override_request = request.clone();
            override_request[forbidden] = json!("caller override");
            assert!(serde_json::from_value::<DiagnosticStartRequest>(override_request).is_err());
        }
    }

    let mut scope = json!({"projectId": "project-1", "targetHostId": "host-1", "appId": "claude"});
    assert_roundtrip::<DiagnosticScopeQuery>(&scope);
    scope["companyId"] = json!("tenant-override");
    assert!(serde_json::from_value::<DiagnosticScopeQuery>(scope).is_err());

    let mut snapshot = json!({
        "job": {
            "taskId": "diagnostic-1", "correlationId": "correlation-1", "kind": "reachability",
            "scope": {
                "project": {
                    "platform_id": "platform-1", "company_id": "company-1", "project_id": "project-1"
                },
                "targetHostId": "host-1", "appId": "claude"
            }
        },
        "progress": {"completed": 1, "total": 2},
        "results": [reachability]
    });
    for status in ["queued", "running", "cancelling", "succeeded", "cancelled", "interrupted"] {
        snapshot["state"] = json!({"status": status});
        assert_roundtrip::<DiagnosticJobSnapshot>(&snapshot);
    }
    snapshot["state"] = json!({
        "status": "failed",
        "error": {"phase": "connect", "code": "timeout"}
    });
    assert_roundtrip::<DiagnosticJobSnapshot>(&snapshot);
    snapshot["state"] =
        json!({"status": "cancelled", "error": {"phase": "connect", "code": "timeout"}});
    assert!(serde_json::from_value::<DiagnosticJobSnapshot>(snapshot.clone()).is_err());

    snapshot["state"] = json!({"status": "interrupted"});
    snapshot["progress"] = json!({"completed": 1});
    snapshot.as_object_mut().unwrap().remove("results");
    assert_roundtrip::<DiagnosticJobSnapshot>(&snapshot);

    let query = json!({"providerId": "provider-1", "draftUrls": ["https://example.invalid/v1"]});
    assert_roundtrip::<DiagnosticEndpointCandidatesQuery>(&query);
    let query = serde_json::from_value::<DiagnosticEndpointCandidatesQuery>(query).unwrap();
    assert!(!format!("{query:?}").contains("example.invalid"));
    assert_roundtrip::<DiagnosticEndpointCandidatesResponse>(&json!({
        "candidates": [{
            "id": "candidate-1", "displayUrl": "https://example.invalid/v1",
            "sources": ["draft", "saved_config", "preset", "custom"]
        }]
    }));
}

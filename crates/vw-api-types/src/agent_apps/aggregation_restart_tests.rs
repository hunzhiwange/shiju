use super::*;
use serde_json::json;
#[test]
fn aggregation_t45_restart_wire_is_bounded_and_explicit() {
    let request = json!({"targetHostId":"host","projectId":"project","expectedRevision":"opaque","requestId":"restart-one","confirmed":true});
    let decoded: CodexRestartRequest = serde_json::from_value(request.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), request);
    let mut missing = request.clone();
    missing.as_object_mut().unwrap().remove("confirmed");
    assert!(serde_json::from_value::<CodexRestartRequest>(missing).is_err());
    for key in ["pid", "command", "path", "environment", "args"] {
        let mut extra = request.clone();
        extra[key] = json!("untrusted");
        assert!(serde_json::from_value::<CodexRestartRequest>(extra).is_err());
    }
    let outcome = AggregationOutcome::Restarted { result: CodexRestartResult::NotRunning };
    assert_eq!(
        serde_json::to_value(&outcome).unwrap(),
        json!({"kind":"restarted","result":"not_running"})
    );
    assert_eq!(
        serde_json::from_value::<AggregationOutcome>(serde_json::to_value(&outcome).unwrap())
            .unwrap(),
        outcome
    );
}

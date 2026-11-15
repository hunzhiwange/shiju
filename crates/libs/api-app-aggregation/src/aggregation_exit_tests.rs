use super::*;
use serde_json::json;
#[test]
fn aggregation_t42_exit_request_rejects_paths_and_requires_confirmation() {
    let value = json!({"targetHostId":"host","projectId":"project","expectedRevision":"r1","requestId":"exit","confirmed":true});
    assert!(serde_json::from_value::<AggregationExitRequest>(value.clone()).is_ok());
    let mut injected = value.clone();
    injected["path"] = json!("/tmp/unsafe");
    assert!(serde_json::from_value::<AggregationExitRequest>(injected).is_err());
    let mut missing = value;
    missing.as_object_mut().unwrap().remove("confirmed");
    assert!(serde_json::from_value::<AggregationExitRequest>(missing).is_err());
    let exited = json!({"kind":"exited","state":{"revision":"r2","mode":"direct","attached":false,"defaultProviderId":null,
        "directProviderId":null,"members":[],"notices":[],"recovery":"restored"}});
    let result: AggregationOutcome = serde_json::from_value(exited.clone()).unwrap();
    assert_eq!(serde_json::to_value(result).unwrap(), exited);
}

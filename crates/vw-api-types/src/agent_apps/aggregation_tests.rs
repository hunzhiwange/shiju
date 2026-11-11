use super::aggregation::*;
use serde_json::json;
#[test]
fn aggregation_t39_requests_reject_unknown_inputs_and_use_contract_names() {
    let value = json!({"targetHostId":"host","projectId":"project"});
    let request: AggregationPreviewRequest = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(request.catalog_policy, CatalogPolicy::Preserve);
    for key in ["path", "accountId", "confirmed", "expectedRevision"] {
        let mut invalid = value.clone();
        invalid[key] = json!("untrusted");
        assert!(serde_json::from_value::<AggregationPreviewRequest>(invalid).is_err());
    }
    let model = AggregationModel { id: "model".into(), label: "model".into(), supports_1m: true };
    let encoded = serde_json::to_value(model).unwrap();
    assert_eq!(encoded["supports1m"], true);
    assert!(encoded.get("supports1M").is_none());
    assert_eq!(
        serde_json::to_value(AggregationNotice::CodexRouteOwnsCatalog).unwrap(),
        "codex_route_owns_catalog"
    );
}

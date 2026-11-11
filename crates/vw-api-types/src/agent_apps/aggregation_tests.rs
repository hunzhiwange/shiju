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

#[test]
fn aggregation_t49_directory_wire_contract_rejects_profile_selectors_and_mcp() {
    let body = json!({"targetHostId":"host","projectId":"project","expectedRevision":"opaque","change":{"kind":"remove"}});
    let request: OmpDirectoryWrite = serde_json::from_value(body.clone()).unwrap();
    assert!(matches!(request.change, OmpDirectoryChange::Remove {}));
    for key in ["profile", "path", "mcpServers", "accountId"] {
        let mut invalid = body.clone();
        invalid[key] = json!("external");
        assert!(serde_json::from_value::<OmpDirectoryWrite>(invalid).is_err());
    }
    let mut invalid = body;
    invalid["change"]["profile"] = json!({"name":"unexpected","declaration":{}});
    assert!(serde_json::from_value::<OmpDirectoryWrite>(invalid).is_err());
    let state = OmpDirectoryState {
        revision: "opaque".into(),
        profile: None,
        members: vec!["managed".into()],
        default_provider_id: Some("builtin".into()),
        default_model_id: Some("model".into()),
    };
    let encoded = serde_json::to_value(state).unwrap();
    assert!(encoded["profile"].is_null());
    assert_eq!(encoded["defaultModelId"], "model");
    assert!(encoded.get("default_model_id").is_none());
}

#[test]
fn aggregation_t48_pi_requests_reject_native_settings_and_extra_payloads() {
    let base = json!({"targetHostId":"host","projectId":"project","expectedRevision":"revision","change":{"kind":"remove"}});
    assert!(serde_json::from_value::<PiDirectoryWrite>(base.clone()).is_ok());
    for key in ["path", "auth", "defaultProviderId", "defaultModelId"] {
        let mut request = base.clone();
        request[key] = json!("untrusted");
        assert!(serde_json::from_value::<PiDirectoryWrite>(request).is_err());
    }
    let mut request = base;
    request["change"]["profile"] = json!({"name":"bad","declaration":{}});
    assert!(serde_json::from_value::<PiDirectoryWrite>(request).is_err());
}

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
fn aggregation_t40_membership_requests_require_revision_and_reject_untrusted_inputs() {
    let member = json!({"targetHostId":"host","projectId":"project","expectedRevision":"revision","enabled":true});
    let parsed: AggregationMemberRequest = serde_json::from_value(member.clone()).unwrap();
    assert!(parsed.enabled);
    assert_eq!(parsed.expected_revision, "revision");
    let default = json!({"targetHostId":"host","projectId":"project","expectedRevision":"revision","providerId":"provider"});
    assert_eq!(
        serde_json::from_value::<AggregationDefaultRequest>(default.clone()).unwrap().provider_id,
        "provider"
    );
    for key in ["path", "accountId", "command", "catalogPolicy"] {
        let mut value = member.clone();
        value[key] = json!("untrusted");
        assert!(serde_json::from_value::<AggregationMemberRequest>(value).is_err());
        let mut value = default.clone();
        value[key] = json!("untrusted");
        assert!(serde_json::from_value::<AggregationDefaultRequest>(value).is_err());
    }
    let mut value = member.clone();
    value.as_object_mut().unwrap().remove("expectedRevision");
    assert!(serde_json::from_value::<AggregationMemberRequest>(value).is_err());
    let mut value = member;
    value["enabled"] = json!("true");
    assert!(serde_json::from_value::<AggregationMemberRequest>(value).is_err());
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
fn aggregation_t46_desktop_wire_contract() {
    let body = json!({"targetHostId":"host","projectId":"project","expectedRevision":"opaque","mapping":{"mode":"proxy","routes":{"claude-sonnet-5":{"model":"upstream","supports1m":true}}}});
    let request: DesktopMappingChange = serde_json::from_value(body.clone()).unwrap();
    assert_eq!(request.project_id.as_ref(), "project");
    assert!(request.mapping.routes["claude-sonnet-5"].supports_1m);
    for key in ["path", "accountId", "command", "app"] {
        let mut invalid = body.clone();
        invalid[key] = json!("untrusted");
        assert!(serde_json::from_value::<DesktopMappingChange>(invalid).is_err());
    }
    for key in ["projectId", "expectedRevision"] {
        let mut invalid = body.clone();
        invalid.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<DesktopMappingChange>(invalid).is_err());
    }
    let value = serde_json::to_value(DesktopProfileStatus {
        revision: "opaque".into(),
        installation: DesktopInstallation::Installed,
        configured: true,
        mode: DesktopProfileMode::DirectGateway,
        applied_provider_id: Some("provider".into()),
        notices: vec![],
        recovery: RecoveryState::Pending,
    })
    .unwrap();
    assert_eq!(value["installation"], "installed");
    assert_eq!(value["mode"], "direct_gateway");
    assert_eq!(value["appliedProviderId"], "provider");
    assert_eq!(value["recovery"], "pending");
    assert!(value.get("profilePath").is_none());
    assert!(value.get("apiKey").is_none());
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

#[test]
fn aggregation_t44_ack_rejects_process_and_path_inputs() {
    let valid = json!({"targetHostId":"host","projectId":"project","expectedRevision":"revision"});
    let parsed: CodexClientsAckRequest = serde_json::from_value(valid.clone()).unwrap();
    assert_eq!(parsed.expected_revision, "revision");
    for field in ["daemonPid", "path", "command", "credentials"] {
        let mut value = valid.clone();
        value[field] = json!("untrusted");
        assert!(serde_json::from_value::<CodexClientsAckRequest>(value).is_err());
    }
    let state = CodexClientsState {
        revision: "revision".into(),
        daemon: true,
        others: true,
        auth: false,
        unverified: false,
    };
    let value = serde_json::to_value(state).unwrap();
    assert_eq!(
        value,
        json!({"revision":"revision","daemon":true,"others":true,"auth":false,"unverified":false})
    );
}

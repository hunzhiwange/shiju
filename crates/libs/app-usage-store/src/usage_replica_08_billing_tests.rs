use super::*;
use vw_api_transport::ProjectId;

#[test]
fn usage_replica_08_billing_defaults_are_atomic_and_scoped() {
    let mut db = Connection::open_in_memory().unwrap();
    initialize_pricing_store(&db).unwrap();
    let scope = ProjectScope::for_default_tenant(ProjectId("fixture".into())).unwrap();
    let mut defaults = load_billing_defaults(&db, &scope, "host-one").unwrap();
    defaults.insert("codex".into(), BillingModelSource::Request);
    save_billing_defaults(&mut db, &scope, "host-one", defaults.clone()).unwrap();
    assert_eq!(load_billing_defaults(&db, &scope, "host-one").unwrap(), defaults);
    assert_eq!(load_billing_defaults(&db, &scope, "host-two").unwrap()["codex"], BillingModelSource::Response);
    let mut invalid = defaults.clone();
    invalid.remove("claude");
    assert!(save_billing_defaults(&mut db, &scope, "host-one", invalid).is_err());
    assert_eq!(load_billing_defaults(&db, &scope, "host-one").unwrap(), defaults);
}

#[test]
fn usage_replica_08_relay_selection_preserves_missing_multiplier_and_native_cost() {
    use vw_api_usage::*;
    let mut db = Connection::open_in_memory().unwrap();
    initialize_pricing_store(&db).unwrap();
    let scope = ProjectScope::for_default_tenant(ProjectId("fixture".into())).unwrap();
    let mut record: UsageRecord = serde_json::from_value(serde_json::json!({
        "id":"request", "started_at_unix_seconds":1, "provider":{"kind":"unknown"}, "sources":[],
        "models":{"request_model":"request-model", "response_model":"response-model", "pricing_model":null,
            "pricing_multiplier":null, "multiplier_basis":null},
        "status":"succeeded", "billable":true, "tokens":null,
        "cost":{"source":"unknown", "amount_usd":null},
        "performance":{"first_token_ms":null,"latency_ms":null,"output_tokens_per_second":null}, "suspected_duplicate_of":null
    })).unwrap();
    let identity = super::super::store::UsageSourceIdentity { target_host:"host".into(), app:"codex".into(),
        instance_id:"fixture".into(), kind:UsageSourceKind::Relay, file_identity:"fixture".into(),
        native_session_id:None, native_request_id:"request".into() };
    apply_billing_default(&db, &scope, &identity, &mut record).unwrap();
    assert_eq!(record.models.pricing_model.as_deref(), Some("response-model"));
    assert!(record.models.pricing_multiplier.is_none());
    assert!(record.cost.amount_usd.is_none());
    record.cost = UsageCost { source:UsageCostSource::Native, amount_usd:Some("1.25".into()) };
    apply_billing_default(&db, &scope, &identity, &mut record).unwrap();
    assert_eq!(record.cost.amount_usd.as_deref(), Some("1.25"));
    assert_eq!(record.cost.source, UsageCostSource::Native);
    let mut defaults = load_billing_defaults(&db, &scope, "host").unwrap();
    defaults.insert("codex".into(), BillingModelSource::Request);
    save_billing_defaults(&mut db, &scope, "host", defaults).unwrap();
    record.cost = UsageCost { source: UsageCostSource::Unknown, amount_usd: None };
    apply_billing_default(&db, &scope, &identity, &mut record).unwrap();
    assert_eq!(record.models.pricing_model.as_deref(), Some("request-model"));
}

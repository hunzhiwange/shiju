use crate::usage::*;
use serde_json::json;

#[test]
fn usage17_usage_unknown_cost_and_estimated_fields_round_trip() {
    let payload = json!({
        "id": "request-1",
        "started_at_unix_seconds": 1794614400,
        "provider": {"kind": "unknown"},
        "sources": [{"source_id": "source-1", "kind": "native", "app": "codex",
            "native_request_id": null, "native_session_id": "session-1"}],
        "models": {"request_model": "alias", "response_model": "resolved",
            "pricing_model": "priced", "pricing_multiplier": "1.25",
            "multiplier_basis": "custom_price"},
        "status": "succeeded", "billable": true,
        "tokens": {"input": 100, "output": 20, "reasoning": 10,
            "cache_read": 40, "cache_creation": 15, "cache_creation_1h": 5,
            "input_semantics": "total", "reasoning_semantics": "included_in_output"},
        "cost": {"source": "unknown", "amount_usd": null},
        "performance": {
            "first_token_ms": {"value": 12.0, "kind": "estimated"},
            "latency_ms": {"value": 100.0, "kind": "measured"},
            "output_tokens_per_second": null},
        "suspected_duplicate_of": null
    });
    let record: UsageRecord = serde_json::from_value(payload.clone()).unwrap();
    assert_eq!(record.provider, UsageProvider::Unknown);
    assert_eq!(record.cost.amount_usd, None);
    assert_eq!(record.cost.source, UsageCostSource::Unknown);
    let tokens = record.tokens.as_ref().unwrap();
    assert_eq!(tokens.input_semantics, UsageInputSemantics::Total);
    assert_eq!(tokens.reasoning_semantics, UsageReasoningSemantics::IncludedInOutput);
    assert_eq!(tokens.cache_creation_1h, Some(5));
    assert_eq!(record.models.request_model.as_deref(), Some("alias"));
    assert_eq!(record.models.response_model.as_deref(), Some("resolved"));
    assert_eq!(record.models.pricing_model.as_deref(), Some("priced"));
    assert_eq!(record.performance.first_token_ms.as_ref().unwrap().kind,
        UsageMeasurementKind::Estimated);
    assert_eq!(serde_json::to_value(record).unwrap(), payload);
}

#[test]
fn usage17_usage_zero_native_cost_is_distinct_from_unknown() {
    let cost = UsageCost { source: UsageCostSource::Native, amount_usd: Some("0".into()) };
    let value = serde_json::to_value(&cost).unwrap();
    assert_eq!(value, json!({"source": "native", "amount_usd": "0"}));
    assert_eq!(serde_json::from_value::<UsageCost>(value).unwrap(), cost);
}

#[test]
fn usage17_usage_query_rejects_identity_and_preserves_calendar_range() {
    let payload = json!({"range": {"kind": "calendar", "period": "today",
        "time_zone": "America/New_York"}, "filter": {}, "cursor": null, "limit": 20});
    let query: UsageQuery = serde_json::from_value(payload.clone()).unwrap();
    assert_eq!(query.range, UsageRange::Calendar {
        period: UsageCalendarPeriod::Today, time_zone: "America/New_York".into()
    });
    for field in ["tenant_id", "host_id", "project_id"] {
        let mut forged = payload.clone();
        forged[field] = json!("other");
        assert!(serde_json::from_value::<UsageQuery>(forged).is_err());
        let mut forged = payload.clone();
        forged["filter"][field] = json!("other");
        assert!(serde_json::from_value::<UsageQuery>(forged).is_err());
    }
}

#[test]
fn usage17_usage_failure_without_usage_preserves_null_and_counts() {
    let tokens: Option<UsageTokens> = serde_json::from_value(json!(null)).unwrap();
    assert!(tokens.is_none());
    let counts = UsageRequestCounts {
        total_requests: 2, billable_requests: 1, unknown_cost_requests: 1
    };
    let value = serde_json::to_value(&counts).unwrap();
    assert_eq!(value["total_requests"], 2);
    assert_eq!(value["billable_requests"], 1);
    assert_eq!(serde_json::from_value::<UsageRequestCounts>(value).unwrap(), counts);
}

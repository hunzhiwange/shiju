use super::agent_apps_high_availability::{
    FailoverCandidatesResponse, FailoverQueueEnabledRequest, FailoverQueueEntry,
    FailoverQueueMemberRequest, FailoverQueueQuery, FailoverQueueReorderRequest,
    FailoverQueueResponse, FailoverQueueScope, FailoverQueueVersion,
};
use crate::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};
use serde_json::json;

#[test]
fn ha_queue_dto_roundtrip_keeps_scope_and_order() {
    let query = FailoverQueueQuery {
        project_id: "project-1".into(),
        target_host_id: "host-1".into(),
        app_id: AppId::Claude,
    };
    let query_wire = json!({
        "projectId": "project-1",
        "targetHostId": "host-1",
        "appId": "claude"
    });
    assert_eq!(serde_json::to_value(&query).unwrap(), query_wire);
    assert_eq!(serde_json::from_value::<FailoverQueueQuery>(query_wire.clone()).unwrap(), query);
    let mut unsafe_query = query_wire;
    unsafe_query["companyId"] = json!("tenant-override");
    assert!(serde_json::from_value::<FailoverQueueQuery>(unsafe_query).is_err());

    let scope = FailoverQueueScope {
        project: ProjectScope::new(
            TenantScope::new("platform-1", "company-1").unwrap(),
            query.project_id,
        )
        .unwrap(),
        target_host_id: query.target_host_id,
        app_id: query.app_id,
    };
    let queue = FailoverQueueResponse {
        scope: scope.clone(),
        version: FailoverQueueVersion(17),
        enabled: true,
        entries: vec![
            FailoverQueueEntry { provider_id: "provider-z".into(), name: "Primary".into() },
            FailoverQueueEntry { provider_id: "provider-a".into(), name: "Backup".into() },
        ],
    };
    let scope_wire = json!({
        "project": {
            "platform_id": "platform-1",
            "company_id": "company-1",
            "project_id": "project-1"
        },
        "targetHostId": "host-1",
        "appId": "claude"
    });
    let queue_wire = json!({
        "scope": scope_wire,
        "version": 17,
        "enabled": true,
        "entries": [
            {"providerId": "provider-z", "name": "Primary"},
            {"providerId": "provider-a", "name": "Backup"}
        ]
    });
    assert_eq!(serde_json::to_value(&queue).unwrap(), queue_wire);
    assert_eq!(serde_json::from_value::<FailoverQueueResponse>(queue_wire.clone()).unwrap(), queue);

    let candidates = FailoverCandidatesResponse {
        scope,
        candidates: vec![FailoverQueueEntry {
            provider_id: "provider-b".into(),
            name: "Candidate".into(),
        }],
    };
    let candidates_wire = json!({
        "scope": scope_wire,
        "candidates": [{"providerId": "provider-b", "name": "Candidate"}]
    });
    assert_eq!(serde_json::to_value(&candidates).unwrap(), candidates_wire);
    assert_eq!(
        serde_json::from_value::<FailoverCandidatesResponse>(candidates_wire.clone()).unwrap(),
        candidates
    );

    let mut unsafe_candidates = candidates_wire;
    unsafe_candidates["candidates"][0]["settingsConfig"] = json!({"apiKey": "fixture-secret"});
    assert!(
        serde_json::from_value::<FailoverCandidatesResponse>(unsafe_candidates).is_err(),
        "candidate entries must reject provider configuration"
    );
    let mut unsafe_queue = queue_wire;
    unsafe_queue["entries"][0]["apiKey"] = json!("fixture-secret");
    assert!(serde_json::from_value::<FailoverQueueResponse>(unsafe_queue).is_err());

    let member_wire = json!({"expectedVersion": 17, "providerId": "provider-b"});
    let member = serde_json::from_value::<FailoverQueueMemberRequest>(member_wire.clone()).unwrap();
    assert_eq!(member.expected_version, queue.version);
    assert_eq!(serde_json::to_value(member).unwrap(), member_wire);
    assert!(
        serde_json::from_value::<FailoverQueueMemberRequest>(json!({
            "expectedVersion": 17, "providerId": "provider-b", "accountId": "override"
        }))
        .is_err()
    );

    let reorder_wire = json!({"expectedVersion": 17, "providerIds": ["provider-a", "provider-z"]});
    let reorder =
        serde_json::from_value::<FailoverQueueReorderRequest>(reorder_wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(reorder).unwrap(), reorder_wire);

    let enabled_wire = json!({"expectedVersion": 17, "enabled": false});
    let enabled =
        serde_json::from_value::<FailoverQueueEnabledRequest>(enabled_wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(enabled).unwrap(), enabled_wire);
    assert!(
        serde_json::from_value::<FailoverQueueEnabledRequest>(json!({
            "expectedVersion": 17, "enabled": false, "scope": scope_wire
        }))
        .is_err()
    );
}

#[test]
fn ha_policy_rejects_out_of_range_values() {
    use super::agent_apps_high_availability::FailoverPolicy;

    let minimum = json!({
        "circuitBreaker": {
            "failureThreshold": 1, "successThreshold": 1, "cooldownSecs": 0,
            "errorRateThreshold": 0.0, "minRequests": 5
        },
        "retryTimeout": {
            "maxRetries": 0, "firstByteTimeoutSecs": 1, "idleTimeoutSecs": 0,
            "nonStreamingTimeoutSecs": 60
        }
    });
    let maximum = json!({
        "circuitBreaker": {
            "failureThreshold": 20, "successThreshold": 10, "cooldownSecs": 300,
            "errorRateThreshold": 1.0, "minRequests": 100
        },
        "retryTimeout": {
            "maxRetries": 10, "firstByteTimeoutSecs": 120, "idleTimeoutSecs": 600,
            "nonStreamingTimeoutSecs": 1200
        }
    });
    for wire in [&minimum, &maximum] {
        let policy: FailoverPolicy = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(policy.validate(), Ok(()));
        assert_eq!(&serde_json::to_value(policy).unwrap(), wire);
    }

    for (section, field, value) in [
        ("circuitBreaker", "failureThreshold", json!(0)),
        ("circuitBreaker", "failureThreshold", json!(21)),
        ("circuitBreaker", "successThreshold", json!(0)),
        ("circuitBreaker", "successThreshold", json!(11)),
        ("circuitBreaker", "cooldownSecs", json!(301)),
        ("circuitBreaker", "errorRateThreshold", json!(-0.01)),
        ("circuitBreaker", "errorRateThreshold", json!(1.01)),
        ("circuitBreaker", "minRequests", json!(4)),
        ("circuitBreaker", "minRequests", json!(101)),
        ("retryTimeout", "maxRetries", json!(11)),
        ("retryTimeout", "firstByteTimeoutSecs", json!(0)),
        ("retryTimeout", "firstByteTimeoutSecs", json!(121)),
        ("retryTimeout", "idleTimeoutSecs", json!(601)),
        ("retryTimeout", "nonStreamingTimeoutSecs", json!(59)),
        ("retryTimeout", "nonStreamingTimeoutSecs", json!(1201)),
    ] {
        let mut wire = minimum.clone();
        wire[section][field] = value;
        let policy: FailoverPolicy = serde_json::from_value(wire).unwrap();
        assert!(policy.validate().is_err(), "{field} accepted an out-of-range value");
    }

    let mut policy: FailoverPolicy = serde_json::from_value(minimum.clone()).unwrap();
    for rate in [f64::NAN, f64::NEG_INFINITY, f64::INFINITY] {
        policy.circuit_breaker.error_rate_threshold = rate;
        assert!(policy.validate().is_err(), "accepted non-finite rate {rate}");
    }

    let mut wrong_units = minimum;
    let timeout = wrong_units["retryTimeout"].as_object_mut().unwrap();
    timeout.remove("firstByteTimeoutSecs");
    timeout.insert("firstByteTimeoutMs".into(), json!(1000));
    assert!(serde_json::from_value::<FailoverPolicy>(wrong_units).is_err());
}

#[test]
fn ha_route_and_health_keep_separate_observations() {
    use super::agent_apps_high_availability::{
        FailoverFailureReason, FailoverHealthResponse, FailoverHealthState, FailoverRouteResponse,
    };

    let scope = json!({
        "project": {
            "platform_id": "platform-1", "company_id": "company-1", "project_id": "project-1"
        },
        "targetHostId": "host-1",
        "appId": "claude"
    });
    for (wire_state, state) in [
        ("closed", FailoverHealthState::Closed),
        ("open", FailoverHealthState::Open),
        ("half_open", FailoverHealthState::HalfOpen),
        ("unknown", FailoverHealthState::Unknown),
    ] {
        let retry_after_ms = (state == FailoverHealthState::Open).then_some(1500);
        let wire = json!({
            "scope": scope,
            "providerId": "primary",
            "state": wire_state,
            "retryAfterMs": retry_after_ms
        });
        let health: FailoverHealthResponse = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(health.state, state);
        assert_eq!(health.retry_after_ms, retry_after_ms);
        assert_eq!(serde_json::to_value(health).unwrap(), wire);
    }

    let wire = json!({
        "scope": scope,
        "configuredProviderId": "primary",
        "actualProviderId": "backup",
        "failureReason": "first_byte_timeout"
    });
    let route: FailoverRouteResponse = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(route.configured_provider_id, Some("primary".into()));
    assert_eq!(route.actual_provider_id, Some("backup".into()));
    assert_eq!(route.failure_reason, Some(FailoverFailureReason::FirstByteTimeout));
    assert_eq!(serde_json::to_value(route).unwrap(), wire);

    let mut unknown_route = wire.clone();
    unknown_route["actualProviderId"] = json!(null);
    let route: FailoverRouteResponse = serde_json::from_value(unknown_route.clone()).unwrap();
    assert_eq!(route.actual_provider_id, None);
    assert_eq!(serde_json::to_value(route).unwrap(), unknown_route);

    let mut unsafe_route = wire;
    unsafe_route["failureReason"] = json!("raw upstream error containing a fixture secret");
    assert!(serde_json::from_value::<FailoverRouteResponse>(unsafe_route).is_err());
}

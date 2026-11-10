use super::{HealthAccount, HealthKey, HealthRegistry};
use crate::high_availability::circuit_breaker::AttemptOutcome::{
    Neutral, ProviderFailure, Success,
};
use std::{
    num::NonZeroUsize,
    time::{Duration, Instant},
};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_high_availability::{
        CircuitBreakerPolicy,
        FailoverHealthState::{Closed, HalfOpen, Open, Unknown},
    },
    id::{ProjectScope, TenantScope},
};

#[test]
fn ha_health_reset_is_scope_bound() {
    let tenant = TenantScope::default();
    let target = HealthKey {
        project: ProjectScope::new(tenant.clone(), "project-a".into()).unwrap(),
        target_host_id: "host-a".into(),
        app_id: AppId::Claude,
        provider_id: "provider-a".into(),
        account: HealthAccount::Bound { id: "account-a".into(), generation: 1 },
    };
    let neighbors = [
        HealthKey { target_host_id: "host-b".into(), ..target.clone() },
        HealthKey { app_id: AppId::Codex, ..target.clone() },
        HealthKey {
            project: ProjectScope::new(
                TenantScope::new("platform-b", "company-b").unwrap(),
                "project-a".into(),
            )
            .unwrap(),
            ..target.clone()
        },
        HealthKey {
            project: ProjectScope::new(tenant, "project-b".into()).unwrap(),
            ..target.clone()
        },
        HealthKey { provider_id: "provider-b".into(), ..target.clone() },
        HealthKey {
            account: HealthAccount::Bound { id: "account-b".into(), generation: 1 },
            ..target.clone()
        },
        HealthKey {
            account: HealthAccount::Bound { id: "account-a".into(), generation: 2 },
            ..target.clone()
        },
        HealthKey { account: HealthAccount::Unbound, ..target.clone() },
    ];
    let policy = CircuitBreakerPolicy {
        failure_threshold: 1,
        success_threshold: 1,
        cooldown_secs: 10,
        error_rate_threshold: 0.5,
        min_requests: 5,
    };
    let now = Instant::now();
    let ttl = Duration::from_secs(60);
    let capacity = NonZeroUsize::new(neighbors.len() + 1).unwrap();
    let health = HealthRegistry::new(capacity, ttl);
    let extra = HealthKey { provider_id: "provider-c".into(), ..target.clone() };
    assert_eq!(health.snapshot(&target, &policy, now).state, Unknown);
    assert!(!health.reset(&extra), "reset must not create a circuit");

    let circuit = health.circuit(&target, now).unwrap();
    circuit.allow_request(&policy, now).unwrap().record(ProviderFailure, now);
    let opened = health.snapshot(&target, &policy, now);
    assert_eq!(opened.state, Open);
    assert_eq!(opened.retry_after_ms, Some(10_000));
    assert!(circuit.allow_request(&policy, now).is_none());
    assert!(
        health.circuit(&neighbors[0], now).unwrap().allow_request(&policy, now).is_some(),
        "a provider failure on one host must not block the same provider on another host"
    );
    for neighbor in &neighbors {
        assert_eq!(health.snapshot(neighbor, &policy, now).state, Unknown);
        health
            .circuit(neighbor, now)
            .unwrap()
            .allow_request(&policy, now)
            .unwrap()
            .record(ProviderFailure, now);
    }
    assert!(health.circuit(&extra, now).is_none(), "capacity must preserve open circuits");
    let probe_at = now + Duration::from_secs(10);
    assert_eq!(health.snapshot(&target, &policy, probe_at).state, Open);
    let stale_probe = circuit.allow_request(&policy, probe_at).unwrap();
    assert_eq!(health.snapshot(&target, &policy, probe_at).state, HalfOpen);
    assert!(health.reset(&target));
    assert_eq!(health.snapshot(&target, &policy, probe_at).state, Unknown);
    let recovered = circuit.allow_request(&policy, probe_at).expect("reset reopens admission");
    recovered.record(Neutral, probe_at);
    assert_eq!(health.snapshot(&target, &policy, probe_at).state, Unknown);

    let stale_closed = circuit.allow_request(&policy, probe_at).unwrap();
    assert!(health.reset(&target));
    stale_closed.record(ProviderFailure, probe_at);
    assert_eq!(health.snapshot(&target, &policy, probe_at).state, Unknown);

    // The pre-reset permit cannot release the next generation's probe slot.
    circuit.allow_request(&policy, probe_at).unwrap().record(ProviderFailure, probe_at);
    let next_probe_at = probe_at + Duration::from_secs(10);
    let probe = circuit.allow_request(&policy, next_probe_at).unwrap();
    drop(stale_probe);
    assert!(circuit.allow_request(&policy, next_probe_at).is_none());
    probe.record(Success, next_probe_at);
    assert_eq!(health.snapshot(&target, &policy, next_probe_at).state, Closed);

    let stale_attempt = circuit.allow_request(&policy, next_probe_at).unwrap();
    assert!(health.remove(&target));
    assert_eq!(health.snapshot(&target, &policy, next_probe_at).state, Unknown);
    let replacement = health.circuit(&target, next_probe_at).unwrap();
    stale_attempt.record(ProviderFailure, next_probe_at);
    replacement.allow_request(&policy, next_probe_at).unwrap().record(Success, next_probe_at);
    assert_eq!(health.snapshot(&target, &policy, next_probe_at).state, Closed);
    for neighbor in &neighbors {
        assert_eq!(health.snapshot(neighbor, &policy, next_probe_at).state, Open);
    }

    drop(replacement);
    let expired_at = next_probe_at + ttl;
    assert_eq!(health.snapshot(&target, &policy, expired_at).state, Unknown);
    assert!(health.circuit(&extra, expired_at).is_some(), "an expired idle circuit frees capacity");

    let restarted = HealthRegistry::new(capacity, ttl);
    assert_eq!(restarted.snapshot(&target, &policy, next_probe_at).state, Unknown);
    assert_eq!(restarted.snapshot(&neighbors[0], &policy, next_probe_at).state, Unknown);
}

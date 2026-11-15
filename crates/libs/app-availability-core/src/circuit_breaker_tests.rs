use super::{AttemptOutcome, CircuitBreaker, CircuitPhase};
use std::time::{Duration, Instant};
use vw_api_types::agent_apps_high_availability::{CircuitBreakerPolicy, FailoverHealthState};

#[test]
fn ha_breaker_opens_only_for_provider_failures() {
    use AttemptOutcome::{Neutral, ProviderFailure, Success};
    use FailoverHealthState::{Closed, Open};

    let now = Instant::now();
    let policy = CircuitBreakerPolicy {
        failure_threshold: 2,
        success_threshold: 2,
        cooldown_secs: 60,
        error_rate_threshold: 0.5,
        min_requests: 5,
    };

    let mut consecutive = CircuitBreaker::default();
    for outcome in [ProviderFailure, Success, ProviderFailure] {
        consecutive.record(outcome, &policy, now);
        assert_eq!(consecutive.state(), Closed, "a success clears the failure streak");
    }
    consecutive.record(Neutral, &policy, now); // Business rejection.
    consecutive.record(Neutral, &policy, now); // Cancellation.
    assert_eq!(consecutive.state(), Closed, "neutral results cannot open the circuit");
    consecutive.record(ProviderFailure, &policy, now);
    assert_eq!(consecutive.state(), Open, "the second consecutive provider failure opens");

    let rate_policy = CircuitBreakerPolicy { failure_threshold: 20, ..policy };
    let mut rate = CircuitBreaker::default();
    for outcome in [ProviderFailure, Success, Success, Success, ProviderFailure] {
        rate.record(outcome, &rate_policy, now);
        assert_eq!(rate.state(), Closed, "respect minimum samples and the error-rate threshold");
    }
    rate.record(Neutral, &rate_policy, now);
    rate.record(Neutral, &rate_policy, now);
    assert_eq!(rate.state(), Closed);
    let opened_at = now + Duration::from_secs(1);
    rate.record(ProviderFailure, &rate_policy, opened_at);
    assert_eq!(rate.state(), Open, "three failures in six counted samples reach 50%");
    rate.record(ProviderFailure, &rate_policy, opened_at + Duration::from_secs(1));
    assert!(
        matches!(rate.state.get_mut().unwrap().phase, CircuitPhase::Open { opened_at: at } if at == opened_at),
        "late failures preserve the opening time"
    );
}

#[test]
fn ha_half_open_releases_permit_and_recovers() {
    use AttemptOutcome::{Neutral, ProviderFailure, Success};
    use FailoverHealthState::{Closed, HalfOpen, Open};

    let policy = CircuitBreakerPolicy {
        failure_threshold: 1,
        success_threshold: 2,
        cooldown_secs: 10,
        error_rate_threshold: 0.5,
        min_requests: 5,
    };
    let now = Instant::now();
    let cooldown = Duration::from_secs(u64::from(policy.cooldown_secs));
    let mut breaker = CircuitBreaker::default();
    breaker.record(ProviderFailure, &policy, now);

    let first_probe_at = now + cooldown;
    assert!(breaker.allow_request(&policy, first_probe_at - Duration::from_millis(1)).is_none());
    assert_eq!(breaker.state(), Open);
    let cancelled =
        breaker.allow_request(&policy, first_probe_at).expect("cooldown expiry admits one probe");
    assert_eq!(breaker.state(), HalfOpen);
    assert!(breaker.allow_request(&policy, first_probe_at).is_none());
    drop(cancelled);
    let first = breaker.allow_request(&policy, first_probe_at).expect("drop releases the probe");
    first.record(Success, first_probe_at);
    assert_eq!(breaker.state(), HalfOpen, "one success is below the recovery threshold");

    let failed = breaker.allow_request(&policy, first_probe_at).unwrap();
    let failed_at = first_probe_at + Duration::from_secs(1);
    failed.record(ProviderFailure, failed_at);
    assert_eq!(breaker.state(), Open);
    let retry_at = failed_at + cooldown;
    assert!(breaker.allow_request(&policy, retry_at - Duration::from_millis(1)).is_none());

    let first = breaker.allow_request(&policy, retry_at).unwrap();
    first.record(Success, retry_at);
    assert_eq!(breaker.state(), HalfOpen, "a failed probe clears earlier successes");
    let neutral = breaker.allow_request(&policy, retry_at).unwrap();
    neutral.record(Neutral, retry_at);
    assert_eq!(breaker.state(), HalfOpen);
    let second = breaker.allow_request(&policy, retry_at).expect("neutral releases the probe");
    assert!(breaker.allow_request(&policy, retry_at).is_none());
    second.record(Success, retry_at);
    assert_eq!(breaker.state(), Closed);

    let first = breaker.allow_request(&policy, retry_at).unwrap();
    let second = breaker.allow_request(&policy, retry_at).expect("closed requests may overlap");
    first.record(Success, retry_at);
    second.record(Success, retry_at);
    assert_eq!(breaker.state(), Closed);

    let stale_success = breaker.allow_request(&policy, retry_at).unwrap();
    let stale_failure = breaker.allow_request(&policy, retry_at).unwrap();
    breaker.allow_request(&policy, retry_at).unwrap().record(ProviderFailure, retry_at);

    let recovered_at = retry_at + cooldown;
    let first = breaker.allow_request(&policy, recovered_at).unwrap();
    stale_success.record(Success, recovered_at);
    assert!(breaker.allow_request(&policy, recovered_at).is_none());
    first.record(Success, recovered_at);
    assert_eq!(breaker.state(), HalfOpen, "an older request cannot count as a probe");
    breaker.allow_request(&policy, recovered_at).unwrap().record(Success, recovered_at);
    stale_failure.record(ProviderFailure, recovered_at);
    assert_eq!(breaker.state(), Closed, "an older failure cannot reopen a recovered circuit");
}

#[test]
fn ha_runtime_owned_probe_releases_and_ignores_retired_generations() {
    use AttemptOutcome::{ProviderFailure, Success};
    use FailoverHealthState::{Closed, HalfOpen, Open};
    let breaker = std::sync::Arc::new(CircuitBreaker::default());
    let now = Instant::now();
    let policy = CircuitBreakerPolicy {
        failure_threshold: 1,
        success_threshold: 1,
        cooldown_secs: 1,
        error_rate_threshold: 1.0,
        min_requests: 5,
    };
    breaker.allow_owned_request(&policy, now).unwrap().record(ProviderFailure, now);
    assert_eq!(breaker.state(), Open);
    let later = now + Duration::from_secs(1);
    let probe = breaker.allow_owned_request(&policy, later).unwrap();
    assert_eq!(breaker.state(), HalfOpen);
    assert!(breaker.allow_owned_request(&policy, later).is_none());
    drop(probe); // A dropped response must release the only half-open slot.
    let probe = breaker.allow_owned_request(&policy, later).unwrap();
    probe.record(Success, later);
    assert_eq!(breaker.state(), Closed);
    let stale = breaker.allow_owned_request(&policy, later).unwrap();
    breaker.reset();
    stale.record(ProviderFailure, later);
    assert_eq!(breaker.state(), Closed);
}

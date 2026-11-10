use super::{AttemptOutcome, CircuitBreaker};
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
    assert_eq!(rate.opened_at, Some(opened_at), "late failures preserve the opening time");
}

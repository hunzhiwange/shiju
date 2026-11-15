use super::*;

#[test]
fn ha_desktop_policy_save_uses_server_validation() {
    let policy = FailoverPolicy {
        retry_timeout: RetryTimeoutPolicy { max_retries: 3, first_byte_timeout_secs: 60,
            idle_timeout_secs: 0, non_streaming_timeout_secs: 600 },
        circuit_breaker: CircuitBreakerPolicy { failure_threshold: 4, success_threshold: 2,
            cooldown_secs: 60, error_rate_threshold: 0.6, min_requests: 10 },
    };
    let mut state = State::default();
    state.load(policy);
    assert_eq!(state.fields[7], "60");
    assert_eq!(state.policy().unwrap(), policy);
    // A representable but out-of-range value reaches the server contract unchanged.
    state.fields[4] = "21".into();
    let submitted = state.policy().unwrap();
    assert_eq!(submitted.circuit_breaker.failure_threshold, 21);
    assert!(submitted.validate().is_err());
    state.error = Some("server range error".into());
    state.invalidate();
    assert!(!state.open);
    assert!(!state.loaded);
    assert!(state.scope.is_none());
    assert!(state.fields.iter().all(String::is_empty));
}

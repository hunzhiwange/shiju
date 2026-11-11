use super::*;
use axum::body::Bytes;
use futures_util::stream;
use vw_agent_apps::high_availability::circuit_breaker::CircuitBreaker;

#[tokio::test]
async fn ha_runtime_stream_completion_releases_probe_before_transport_eof() {
    let breaker = Arc::new(CircuitBreaker::default());
    let policy = CircuitBreakerPolicy {
        failure_threshold: 1,
        success_threshold: 1,
        cooldown_secs: 0,
        error_rate_threshold: 1.0,
        min_requests: 5,
    };
    let now = std::time::Instant::now();
    breaker.allow_owned_request(&policy, now).unwrap().record(AttemptOutcome::ProviderFailure, now);
    let probe = breaker.allow_owned_request(&policy, now).unwrap();
    assert!(breaker.allow_owned_request(&policy, now).is_none());
    let sample = crate::model_relay::stream_observer::tests::samples()
        .into_iter()
        .find(|(kind, _, _)| *kind == "anthropic")
        .unwrap()
        .2;
    // The HTTP connection remains open after the protocol's successful terminal.
    let wire = stream::once(async move { Ok::<_, std::io::Error>(Bytes::from(sample)) })
        .chain(stream::pending());
    let response =
        observe(Response::new(Body::from_stream(wire)), probe, CancellationToken::new(), true);
    let mut body = response.into_body().into_data_stream();
    assert!(body.next().await.unwrap().is_ok());
    // A successful terminal closes the circuit. Two ordinary requests may now
    // overlap; a retained half-open lease would block the first or the second.
    let first = breaker.allow_owned_request(&policy, now).unwrap();
    let second = breaker.allow_owned_request(&policy, now).unwrap();
    drop((first, second, body));
}

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
    let mut response = Response::new(Body::from_stream(wire));
    response.headers_mut().insert(header::CONTENT_TYPE, "text/event-stream".parse().unwrap());
    let response = observe(
        response,
        probe,
        CancellationToken::new(),
        Some(Instant::now() + Duration::from_secs(1)),
    )
    .await
    .unwrap();
    let mut body = response.into_body().into_data_stream();
    assert!(body.next().await.unwrap().is_ok());
    // A successful terminal closes the circuit. Two ordinary requests may now
    // overlap; a retained half-open lease would block the first or the second.
    let first = breaker.allow_owned_request(&policy, now).unwrap();
    let second = breaker.allow_owned_request(&policy, now).unwrap();
    drop((first, second, body));
}

#[tokio::test]
async fn ha_first_byte_timeout_stops_silent_upstream_releases_probe() {
    use vw_api_types::agent_apps_high_availability::FailoverHealthState;
    for cancelled in [false, true] {
        let breaker = Arc::new(CircuitBreaker::default());
        let policy = CircuitBreakerPolicy {
            failure_threshold: 1,
            success_threshold: 1,
            cooldown_secs: 0,
            error_rate_threshold: 1.0,
            min_requests: 5,
        };
        let now = std::time::Instant::now();
        breaker
            .allow_owned_request(&policy, now)
            .unwrap()
            .record(AttemptOutcome::ProviderFailure, now);
        let probe = breaker.allow_owned_request(&policy, now).unwrap();
        assert!(breaker.allow_owned_request(&policy, now).is_none());
        let cancel = CancellationToken::new();
        if cancelled {
            cancel.cancel();
        }
        let mut response =
            Response::new(Body::from_stream(stream::pending::<Result<Bytes, std::io::Error>>()));
        response.headers_mut().insert(header::CONTENT_TYPE, "text/event-stream".parse().unwrap());
        assert_eq!(
            observe(response, probe, cancel, Some(Instant::now())).await.unwrap_err(),
            StatusCode::GATEWAY_TIMEOUT
        );
        assert_eq!(
            breaker.state(),
            if cancelled { FailoverHealthState::HalfOpen } else { FailoverHealthState::Open }
        );
        let next = breaker
            .allow_owned_request(&policy, std::time::Instant::now())
            .expect("timeout retained half-open lease");
        drop(next);
    }
}

#[tokio::test]
async fn ha_first_byte_timeout_stops_silent_upstream_preserves_fragmented_prelude() {
    let sample = crate::model_relay::stream_observer::tests::samples()
        .into_iter()
        .find(|(kind, _, _)| *kind == "anthropic")
        .unwrap()
        .2;
    // First output is complete, but the following frame ends in mid-field.
    let split = sample.find("content_block_stop").unwrap() + 5;
    let wire = stream::iter([
        Ok::<_, std::io::Error>(Bytes::copy_from_slice(&sample.as_bytes()[..split])),
        Ok(Bytes::copy_from_slice(&sample.as_bytes()[split..])),
    ]);
    let breaker = Arc::new(CircuitBreaker::default());
    let policy = CircuitBreakerPolicy {
        failure_threshold: 1,
        success_threshold: 1,
        cooldown_secs: 0,
        error_rate_threshold: 1.0,
        min_requests: 5,
    };
    let permit = breaker.allow_owned_request(&policy, std::time::Instant::now()).unwrap();
    let mut response = Response::new(Body::from_stream(wire));
    response.headers_mut().insert(header::CONTENT_TYPE, "text/event-stream".parse().unwrap());
    let response = observe(
        response,
        permit,
        CancellationToken::new(),
        Some(Instant::now() + Duration::from_secs(1)),
    )
    .await
    .unwrap();
    let actual = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024).await.unwrap();
    assert_eq!(actual.as_ref(), sample.as_bytes());
}

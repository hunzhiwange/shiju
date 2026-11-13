use super::*;

fn connection_error() -> String {
    serde_json::to_string(&AssistantError::APIError {
        message: "connection refused".into(),
        status_code: None,
        is_retryable: true,
        response_headers: None,
        response_body: None,
        metadata: Some(std::collections::HashMap::from([(
            "connection_failure".into(),
            "true".into(),
        )])),
    })
    .unwrap()
}

#[test]
fn connection_failure_waits_even_without_service_retry_budget_and_can_cancel() {
    let cancellation = CancellationToken::new();
    let mut phases = Vec::new();
    let result = RetryBudget::new(0, 0).run(
        false,
        Some(&cancellation),
        &mut |event| {
            if let StreamEvent::ModelRecovery(recovery) = event {
                if matches!(recovery.phase, RecoveryPhase::Waiting) {
                    assert_eq!(recovery.wait_ms, 5_000);
                    cancellation.cancel();
                }
                phases.push(recovery.phase);
            }
            true
        },
        |_| Err::<(), _>(connection_error()),
    );
    assert_eq!(result.unwrap_err(), "执行已取消");
    assert!(phases.iter().any(|phase| matches!(phase, RecoveryPhase::Waiting)));
    assert!(matches!(phases.last(), Some(RecoveryPhase::Cancelled)));
}

#[test]
fn connection_failure_recovers_after_service_budget_is_exhausted() {
    let mut attempts = 0;
    let result = RetryBudget::new(0, 0).run(false, None, &mut |_| true, |_| {
        attempts += 1;
        if attempts == 1 { Err(connection_error()) } else { Ok(()) }
    });
    assert!(result.is_ok());
    assert_eq!(attempts, 2);
}

#[test]
fn connection_failure_does_not_replay_visible_output() {
    let mut attempts = 0;
    let result = RetryBudget::new(0, 0).run(false, None, &mut |_| true, |sink| {
        attempts += 1;
        sink(StreamEvent::Delta("partial".into()));
        Err::<(), _>(connection_error())
    });
    assert!(result.is_err());
    assert_eq!(attempts, 1);
}

#[test]
fn service_retry_respects_retry_after_and_remains_cancellable() {
    let cancellation = CancellationToken::new();
    let error = serde_json::to_string(&AssistantError::APIError {
        message: "overloaded".into(),
        status_code: Some(503),
        is_retryable: true,
        response_headers: Some(std::collections::HashMap::from([(
            "Retry-After".into(),
            "12".into(),
        )])),
        response_body: None,
        metadata: None,
    })
    .unwrap();
    let mut waiting = false;
    let result = RetryBudget::new(1, 0).run(
        false,
        Some(&cancellation),
        &mut |event| {
            if let StreamEvent::ModelRecovery(recovery) = event {
                if matches!(recovery.phase, RecoveryPhase::Waiting) {
                    assert_eq!(recovery.wait_ms, 12_000);
                    waiting = true;
                    cancellation.cancel();
                }
            }
            true
        },
        |_| Err::<(), _>(error.clone()),
    );
    assert!(waiting);
    assert_eq!(result.unwrap_err(), "执行已取消");
}

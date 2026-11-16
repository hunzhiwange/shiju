use super::*;

#[test]
fn compute_max_in_flight_messages_clamps_to_configured_bounds() {
    assert_eq!(compute_max_in_flight_messages(0), CHANNEL_MIN_IN_FLIGHT_MESSAGES);
    assert_eq!(compute_max_in_flight_messages(1), CHANNEL_MIN_IN_FLIGHT_MESSAGES);
    assert_eq!(compute_max_in_flight_messages(4), 4 * CHANNEL_PARALLELISM_PER_CHANNEL);
    assert_eq!(compute_max_in_flight_messages(usize::MAX), CHANNEL_MAX_IN_FLIGHT_MESSAGES);
}

#[tokio::test]
async fn log_worker_join_result_ignores_success_and_records_join_errors() {
    log_worker_join_result(Ok(()));

    let panicked = tokio::spawn(async {
        panic!("worker failed");
    })
    .await;
    log_worker_join_result(panicked.map(|_| ()));

    let handle = tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    });
    handle.abort();
    log_worker_join_result(handle.await.map(|_| ()));
}

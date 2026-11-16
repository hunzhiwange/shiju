use super::*;

pub async fn cron_handlers_reject_requests_when_pairing_is_required(state: CronState) {
    let headers = HeaderMap::new();

    let (status, body) = response_json(
        handle_api_cron_list(State(state.clone()), headers.clone()).await,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"].as_str().unwrap().contains("Unauthorized"));

    let (status, _) = response_json(
        handle_api_cron_runs(
            State(state.clone()),
            headers.clone(),
            Path("missing".to_string()),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = response_json(
        handle_api_cron_delete(
            State(state.clone()),
            headers,
            Path("missing".to_string()),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

pub async fn cron_mutating_handlers_reject_requests_before_writing_when_pairing_is_required(state: CronState) {
    let headers = HeaderMap::new();

    let (status, body) = response_json(
        handle_api_cron_add(
            State(state.clone()),
            headers.clone(),
            JsonResponse(add_body()),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"].as_str().unwrap().contains("Unauthorized"));

    let (status, body) = response_json(
        handle_api_cron_update(
            State(state.clone()),
            headers,
            Path("missing".to_string()),
            JsonResponse(update_body()),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"].as_str().unwrap().contains("Unauthorized"));
}

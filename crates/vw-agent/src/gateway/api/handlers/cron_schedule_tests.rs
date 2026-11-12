//! 定时规则协议、日期转换与无副作用预览验收。
use super::*;

#[tokio::test]
async fn wb_schedule_api_timezone_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let config = temp_config(dir.path());
    let body = serde_json::from_value(serde_json::json!({
        "schedule":"0 9 * * *", "timezone":"Asia/Shanghai", "command":"echo ok"
    }))
    .unwrap();
    let (status, result) = response_json(
        handle_api_cron_add(
            State(state_with_config(config, false)),
            HeaderMap::new(),
            JsonResponse(body),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["job"]["timezone"], "Asia/Shanghai");
}

#[tokio::test]
async fn wb_schedule_api_biweekly_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let body = serde_json::from_value(serde_json::json!({
        "schedule_kind":"biweekly", "anchor_date":"2026-11-09", "weekdays":[1,3],
        "hour":9, "minute":30, "timezone":"Asia/Shanghai", "command":"echo ok"
    }))
    .unwrap();
    let (status, result) = response_json(
        handle_api_cron_add(
            State(state_with_config(temp_config(dir.path()), false)),
            HeaderMap::new(),
            JsonResponse(body),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["job"]["weekdays"], serde_json::json!([1, 3]));
}

async fn wb_add_json(value: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let dir = tempfile::tempdir().unwrap();
    response_json(
        handle_api_cron_add(
            State(state_with_config(temp_config(dir.path()), false)),
            HeaderMap::new(),
            JsonResponse(serde_json::from_value(value).unwrap()),
        )
        .await,
    )
    .await
}

#[tokio::test]
async fn wb_schedule_api_rejects_conflicting_at() {
    let (status, _) =
        wb_add_json(serde_json::json!({"schedule_kind":"at", "at":"2099-01-01T00:00:00Z",
        "local_at":"2099-01-01T09:00", "timezone":"Asia/Shanghai", "command":"echo ok"}))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn wb_schedule_api_rejects_conflicting_windows() {
    let (status, _) = wb_add_json(serde_json::json!({"schedule":"0 9 * * *", "timezone":"UTC",
        "starts_at":"2099-01-01T00:00:00Z", "start_date":"2099-01-01", "command":"echo ok"}))
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn wb_schedule_api_rejects_invalid_timezone() {
    let (status, _) = wb_add_json(
        serde_json::json!({"schedule":"0 9 * * *", "timezone":"Mars/Here", "command":"echo ok"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

async fn wb_preview_json(value: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let dir = tempfile::tempdir().unwrap();
    response_json(
        handle_api_cron_preview(
            State(state_with_config(temp_config(dir.path()), false)),
            HeaderMap::new(),
            JsonResponse(serde_json::from_value(value).unwrap()),
        )
        .await,
    )
    .await
}

#[tokio::test]
async fn wb_schedule_api_preview_matches_persisted_schedule() {
    let dir = tempfile::tempdir().unwrap();
    let config = temp_config(dir.path());
    let state = || State(state_with_config(config.clone(), false));
    let input = serde_json::json!({"schedule_kind":"biweekly", "anchor_date":"2099-01-01", "weekdays":[1,3],
        "hour":9, "minute":30, "timezone":"Asia/Shanghai", "start_date":"2099-01-01", "end_date":"2099-03-01", "command":"echo ok"});
    // Initialize SQLite, then prove preview leaves both jobs and run history unchanged.
    assert!(crate::cron::list_jobs(&config.lock()).unwrap().is_empty());
    let before_runs = crate::cron::list_runs(&config.lock(), "missing", 100).unwrap().len();
    let (status, preview) = response_json(
        handle_api_cron_preview(
            state(),
            HeaderMap::new(),
            JsonResponse(serde_json::from_value(input.clone()).unwrap()),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["candidates"].as_array().unwrap().len(), 5);
    assert!(crate::cron::list_jobs(&config.lock()).unwrap().is_empty());
    assert_eq!(crate::cron::list_runs(&config.lock(), "missing", 100).unwrap().len(), before_runs);
    let (status, saved) = response_json(
        handle_api_cron_add(
            state(),
            HeaderMap::new(),
            JsonResponse(serde_json::from_value(input).unwrap()),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["job"]["next_run"], preview["candidates"][0]);
    for key in ["timezone", "anchor_date", "weekdays", "hour", "minute", "starts_at", "ends_at"] {
        assert_eq!(saved["job"][key], preview["normalized"][key], "{key}");
    }
    let job_id = saved["job"]["id"].as_str().unwrap();
    let recorded_at = Utc.with_ymd_and_hms(2026, 11, 12, 0, 0, 0).unwrap();
    crate::cron::record_run(
        &config.lock(),
        job_id,
        recorded_at,
        recorded_at,
        "success",
        Some("preview fixture"),
        0,
    )
    .unwrap();
    let before_jobs =
        serde_json::to_value(crate::cron::list_jobs(&config.lock()).unwrap()).unwrap();
    let before_runs = crate::cron::list_runs(&config.lock(), job_id, 100).unwrap().len();
    assert_eq!(before_runs, 1);
    let (_, repeated) = response_json(
        handle_api_cron_preview(
            state(),
            HeaderMap::new(),
            JsonResponse(
                serde_json::from_value(
                    serde_json::json!({"schedule":"0 9 * * *", "timezone":"Asia/Shanghai"}),
                )
                .unwrap(),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(repeated["candidates"].as_array().unwrap().len(), 5);
    assert_eq!(
        serde_json::to_value(crate::cron::list_jobs(&config.lock()).unwrap()).unwrap(),
        before_jobs
    );
    assert_eq!(crate::cron::list_runs(&config.lock(), job_id, 100).unwrap().len(), before_runs);
}

#[tokio::test]
async fn wb_schedule_api_preview_requires_authentication() {
    let dir = tempfile::tempdir().unwrap();
    let config = temp_config(dir.path());
    let (status, _) = response_json(
        handle_api_cron_preview(
            State(state_with_config(config.clone(), true)),
            HeaderMap::new(),
            JsonResponse(add_body()),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(crate::cron::list_jobs(&config.lock()).unwrap().is_empty());
}

#[tokio::test]
async fn wb_schedule_api_local_at_converts_and_returns_one_candidate() {
    let (status, result) = wb_preview_json(serde_json::json!({"schedule_kind":"at", "local_at":"2099-01-01T09:30", "timezone":"Asia/Shanghai"})).await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["candidates"], serde_json::json!(["2099-01-01T01:30:00+00:00"]));
    assert_eq!(result["normalized"]["at"], result["candidates"][0]);
}

#[tokio::test]
async fn wb_schedule_api_local_at_rejects_dst_gap_and_overlap() {
    for local in ["2027-03-14T02:30", "2027-11-07T01:30"] {
        let (status, result) = wb_preview_json(serde_json::json!({"schedule_kind":"at", "local_at":local, "timezone":"America/New_York"})).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{result}");
        assert!(result["error"].as_str().unwrap().contains("local_at"));
    }
}

#[tokio::test]
async fn wb_schedule_api_date_window_uses_next_local_midnight() {
    let body: CronAddBody = serde_json::from_value(serde_json::json!({"schedule":"0 * * * *", "timezone":"America/New_York", "start_date":"2027-03-14", "end_date":"2027-03-14"})).unwrap();
    let result =
        cron_schedule::preview(&body, Utc.with_ymd_and_hms(2027, 3, 1, 0, 0, 0).unwrap()).unwrap();
    assert_eq!(result["normalized"]["starts_at"], "2027-03-14T05:00:00+00:00");
    assert_eq!(result["normalized"]["ends_at"], "2027-03-15T04:00:00+00:00");
}

#[tokio::test]
async fn wb_schedule_api_window_patch_and_timezone_only_update() {
    let dir = tempfile::tempdir().unwrap();
    let config = temp_config(dir.path());
    let state = || State(state_with_config(config.clone(), false));
    let input = serde_json::json!({"schedule":"0 9 * * *", "timezone":"UTC", "start_date":"2099-01-01", "end_date":"2099-12-31", "command":"echo ok"});
    let (status, saved) = response_json(
        handle_api_cron_add(
            state(),
            HeaderMap::new(),
            JsonResponse(serde_json::from_value(input).unwrap()),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let id = saved["job"]["id"].as_str().unwrap();
    for (patch, clear) in [
        (serde_json::json!({"timezone":"Asia/Shanghai"}), false),
        (serde_json::json!({"starts_at":null,"ends_at":null}), true),
    ] {
        let (status, result) = response_json(
            handle_api_cron_update(
                state(),
                HeaderMap::new(),
                Path(id.into()),
                JsonResponse(serde_json::from_value(patch).unwrap()),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{result}");
        assert_eq!(result["job"]["timezone"], "Asia/Shanghai");
        assert_eq!(result["job"]["starts_at"].is_null(), clear);
        assert_eq!(result["job"]["ends_at"].is_null(), clear);
        assert_eq!(result["job"]["expression"], "0 0 9 * * *");
    }
    let (status, result) = response_json(handle_api_cron_update(state(), HeaderMap::new(), Path(id.into()), JsonResponse(serde_json::from_value(serde_json::json!({"starts_at":"2099-01-01T00:00:00Z", "ends_at":"2099-01-02T00:00:00Z"})).unwrap())).await).await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["job"]["starts_at"], "2099-01-01T00:00:00+00:00");
    assert_eq!(result["job"]["ends_at"], "2099-01-02T00:00:00+00:00");
    let before = serde_json::to_value(crate::cron::get_job(&config.lock(), id).unwrap()).unwrap();
    let (status, _) = response_json(
        handle_api_cron_update(
            state(),
            HeaderMap::new(),
            Path(id.into()),
            JsonResponse(
                serde_json::from_value(serde_json::json!({"timezone":"invalid"})).unwrap(),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        serde_json::to_value(crate::cron::get_job(&config.lock(), id).unwrap()).unwrap(),
        before
    );
}

#[tokio::test]
async fn wb_schedule_api_rejects_invalid_parameters_and_empty_windows() {
    for value in [
        serde_json::json!({"schedule_kind":"unknown"}),
        serde_json::json!({"schedule_kind":"every", "every_ms":0}),
        serde_json::json!({"schedule_kind":"every", "every_ms":u64::MAX}),
        serde_json::json!({"schedule_kind":"biweekly", "anchor_date":"2099-02-30", "weekdays":[1], "hour":9, "minute":0}),
        serde_json::json!({"schedule_kind":"biweekly", "anchor_date":"2099-01-01", "weekdays":[], "hour":9, "minute":0}),
        serde_json::json!({"schedule_kind":"biweekly", "anchor_date":"2099-01-01", "weekdays":[8], "hour":24, "minute":60}),
        serde_json::json!({"schedule":"0 9 * * *", "starts_at":"2099-01-02T00:00:00Z", "ends_at":"2099-01-01T00:00:00Z"}),
        serde_json::json!({"schedule":"0 9 * * *", "starts_at":"2099-01-01T00:00:00Z", "ends_at":"2099-01-01T01:00:00Z"}),
        serde_json::json!({"schedule_kind":"at", "local_at":"2099-01-01T09:00"}),
        serde_json::json!({"schedule":"0 9 * * *", "start_date":"2099-01-01"}),
    ] {
        let (status, result) = wb_preview_json(value.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{value}: {result}");
        assert!(result["error"].is_string());
    }
}

#[test]
fn wb_schedule_api_date_boundary_rejects_missing_and_ambiguous_midnight() {
    for (tz, day) in [("Pacific/Apia", "2011-12-30"), ("America/Havana", "2026-11-01")] {
        let body: CronAddBody = serde_json::from_value(
            serde_json::json!({"schedule":"0 9 * * *", "timezone":tz, "start_date":day}),
        )
        .unwrap();
        let result =
            cron_schedule::preview(&body, Utc.with_ymd_and_hms(2010, 1, 1, 0, 0, 0).unwrap());
        assert!(result.unwrap_err().contains("nonexistent or ambiguous"));
    }
}

#[tokio::test]
async fn wb_schedule_api_reads_ended_jobs() {
    let dir = tempfile::tempdir().unwrap();
    let config = temp_config(dir.path());
    let saved = crate::cron::add_shell_job(
        &config.lock(),
        None,
        Schedule::Every { every_ms: 1000 },
        "echo ok",
    )
    .unwrap();
    crate::cron::update_job(
        &config.lock(),
        &saved.id,
        crate::cron::CronJobPatch {
            ends_at: Some(Some(Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap())),
            ..Default::default()
        },
    )
    .unwrap();
    let (status, result) = response_json(
        handle_api_cron_list(State(state_with_config(config, false)), HeaderMap::new()).await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["jobs"][0]["ends_at"], "2020-01-01T00:00:00+00:00");
    assert_eq!(result["jobs"][0]["enabled"], false);
}

#[tokio::test]
async fn wb_schedule_api_preview_routes_enforce_auth_and_return_candidates() {
    use axum::{Router, body::Body, http::Request};
    use tower::ServiceExt;
    for pairing in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let config = temp_config(dir.path());
        let state = state_with_config(config.clone(), pairing);
        let router =
            crate::gateway::bootstrap::client_router(state.clone(), Router::new(), Router::new())
                .with_state(state);
        for path in ["/api/cron/preview", "/v1/cron/preview"] {
            let request = Request::post(path)
                .header("X-Platform-ID", "cron-preview-test")
                .header("X-Company-ID", "cron-preview-test")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"schedule_kind":"every","every_ms":1000}"#))
                .unwrap();
            let (status, result) =
                response_json(router.clone().oneshot(request).await.unwrap()).await;
            assert_eq!(
                status,
                if pairing { StatusCode::UNAUTHORIZED } else { StatusCode::OK },
                "{result}"
            );
            if !pairing {
                assert_eq!(result["candidates"].as_array().unwrap().len(), 5);
            }
        }
        assert!(crate::cron::list_jobs(&config.lock()).unwrap().is_empty());
    }
}

#[test]
fn wb_schedule_api_calendar_cron_and_every_share_domain_candidates() {
    let now = Utc.with_ymd_and_hms(2026, 11, 12, 0, 0, 0).unwrap();
    for input in [
        serde_json::json!({"schedule":"0 9 * * 1,3", "timezone":"Asia/Shanghai"}),
        serde_json::json!({"schedule":"0 9 1,15 * *", "timezone":"Asia/Shanghai"}),
        serde_json::json!({"schedule":"0 9 29 2 *", "timezone":"UTC"}),
        serde_json::json!({"schedule_kind":"every", "every_ms":86400000}),
    ] {
        let body: CronAddBody = serde_json::from_value(input).unwrap();
        let schedule = cron_add_schedule(&body).unwrap();
        let result = cron_schedule::preview(&body, now).unwrap();
        let mut from = now;
        for actual in result["candidates"].as_array().unwrap() {
            let expected = crate::cron::next_run_for_schedule(&schedule, from).unwrap();
            assert_eq!(actual, &serde_json::json!(expected.to_rfc3339()));
            from = expected;
        }
        assert_eq!(result["candidates"].as_array().unwrap().len(), 5);
    }
}

#[tokio::test]
async fn wb_schedule_api_local_at_patch_requires_timezone() {
    let dir = tempfile::tempdir().unwrap();
    let config = temp_config(dir.path());
    let state = || State(state_with_config(config.clone(), false));
    let original = Utc.with_ymd_and_hms(2099, 1, 1, 0, 0, 0).unwrap();
    let saved =
        crate::cron::add_shell_job(&config.lock(), None, Schedule::At { at: original }, "echo ok")
            .unwrap();
    let (status, result) = response_json(
        handle_api_cron_update(
            state(),
            HeaderMap::new(),
            Path(saved.id.clone()),
            JsonResponse(
                serde_json::from_value(serde_json::json!({"local_at":"2099-01-02T09:00"})).unwrap(),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{result}");
    assert!(result["error"].as_str().unwrap().contains("timezone"));
    assert_eq!(
        crate::cron::get_job(&config.lock(), &saved.id).unwrap().schedule,
        Schedule::At { at: original }
    );
    let (status, result) = response_json(
        handle_api_cron_update(
            state(),
            HeaderMap::new(),
            Path(saved.id),
            JsonResponse(
                serde_json::from_value(
                    serde_json::json!({"local_at":"2099-01-02T09:00", "timezone":"Asia/Shanghai"}),
                )
                .unwrap(),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["job"]["at"], "2099-01-02T01:00:00+00:00");
}

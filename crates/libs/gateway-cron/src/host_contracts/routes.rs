use super::*;
use tower::ServiceExt;
use axum::{Router, body::Body, http::Request};

pub async fn cron_history_endpoint_requires_auth(router: Router) {
    let (status, _) = response_json(router.oneshot(
        Request::builder().uri("/v1/cron/history")
            .header("X-Platform-ID", "test-platform")
            .header("X-Company-ID", "test-company")
            .body(Body::empty()).unwrap()
    ).await.unwrap()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

pub async fn wb_schedule_api_preview_routes_enforce_auth_and_return_candidates(
    router: Router, config: Arc<parking_lot::Mutex<Config>>, pairing: bool,
) {
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
    assert!(agent_cron::list_jobs(&config.lock()).unwrap().is_empty());
}

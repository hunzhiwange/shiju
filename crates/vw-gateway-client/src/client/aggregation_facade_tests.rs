//! Compatibility facade retains the domain error and request behavior.
use super::{AggregationApiError, test_support};
use vw_api_app_aggregation::AggregationQuery;
use vw_api_app_identity::AppId;

#[tokio::test]
async fn aggregation_facade_preserves_structured_errors() {
    let body = serde_json::json!({"error":{"code":"forbidden"}});
    let server = test_support::server(vec![(403, body.clone())]);
    let error = server.client().aggregation_state(AppId::Claude, &AggregationQuery {
        target_host_id: "host".into(), project_id: "project".into(),
    }).await.unwrap_err();
    assert_eq!(error, AggregationApiError::Http { status: 403, body });
    assert_eq!(server.take_request().method, "GET");
    server.join();
}

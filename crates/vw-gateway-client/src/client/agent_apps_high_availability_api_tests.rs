use super::{AgentAppsHighAvailabilityError, test_support};
use vw_api_types::agent_apps_high_availability::FailoverQueueQuery;

#[tokio::test]
async fn failover_facade_preserves_http_error() {
    let server = test_support::server_raw(vec![(503, "upstream".into())]);
    let query = FailoverQueueQuery {
        project_id: "project".into(), target_host_id: "host".into(),
        app_id: vw_api_app_identity::AppId::Codex,
    };
    let error: gateway_app_failover::AgentAppsHighAvailabilityError =
        server.client().agent_apps_failover_queue(&query, None).await.unwrap_err();
    assert_eq!(error, AgentAppsHighAvailabilityError::Http { status: 503, body: None });
    assert_eq!(server.take_request().method, "GET");
    server.join();
}

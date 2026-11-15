use super::{AgentAppsLifecycleError, test_support};
use vw_api_transport::id::TaskId;

#[tokio::test]
async fn lifecycle_facade_preserves_operation_response_errors() {
    let server = test_support::server(vec![(409, serde_json::json!({"code": "conflict"}))]);
    assert_eq!(
        server.client().agent_apps_operation("host", &TaskId::from("op /?")).await.unwrap_err(),
        AgentAppsLifecycleError::Http { status: 409, body: serde_json::json!({"code": "conflict"}) }
    );
    assert_eq!(server.take_request().path, "/v1/agent-apps/operations/op%20%2F%3F?targetHostId=host");
    server.join();
}

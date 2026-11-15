//! 兼容门面的请求与领域类型契约。
use crate::client::test_support;

#[tokio::test]
async fn workflow_facade_preserves_list_request_and_tenant() {
    let server = test_support::server(vec![(200, serde_json::json!([]))]);
    let records = server.client().workflow_applications_list().await.expect("list");
    assert!(records.is_empty());
    let request = server.take_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/workflow/applications");
    assert_eq!(request.header("x-platform-id"), Some("10000"));
    assert_eq!(request.header("x-company-id"), Some("100000000"));
    server.join();
}

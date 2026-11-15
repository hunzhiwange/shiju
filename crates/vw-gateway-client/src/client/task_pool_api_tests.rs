//! 任务池兼容门面的转发契约。

use serde_json::json;
use crate::client::test_support;

#[tokio::test]
async fn task_plan_run_stream_stop_uses_request_scoped_route() {
    let server = test_support::server(vec![(200, json!({ "cancelled": true }))]);

    let cancelled = server
        .client()
        .task_plan_run_stream_stop("desktop-plan-request")
        .await
        .expect("plan stream stop");

    assert!(cancelled);
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/task-plan-run/stream/desktop-plan-request/stop");
    assert_eq!(request.body, json!({}));
    server.join();
}

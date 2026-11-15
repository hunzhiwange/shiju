use crate::client::test_support::server;
use serde_json::json;

#[tokio::test]
async fn dynamic_tools_facade_preserves_registration() {
    let server = server(vec![(200, json!({"host_id":"host", "binding_token":"binding"}))]);
    let binding = server.client().register_dynamic_tool_host("session", None).await.unwrap();
    assert_eq!(binding.host_id, "host");
    assert_eq!(binding.binding_token, "binding");
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/session/session/dynamic-tools/hosts");
    server.join();
}

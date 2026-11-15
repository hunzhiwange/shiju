use crate::test_support;
use serde_json::json;

#[tokio::test]
async fn control_routes_preserve_credentials_and_directory() {
    let server = test_support::server(vec![(200, json!(true)); 4]);
    assert!(server.client().auth_set("  ", &json!({})).await.is_err());
    assert!(server.client().auth_set("openai /合作", &json!({"key": "secret-key"})).await.unwrap());
    assert!(server.client().auth_remove("openai /合作").await.unwrap());
    assert!(server.client().global_dispose().await.unwrap());
    assert!(server.client().instance_dispose(Some("/tmp/instance one")).await.unwrap());
    let request = server.take_request();
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, "/v1/auth/openai%20%2F%E5%90%88%E4%BD%9C");
    assert_eq!(request.body, json!({"key": "secret-key"}));
    let request = server.take_request();
    assert_eq!(request.method, "DELETE");
    assert_eq!(request.path, "/v1/auth/openai%20%2F%E5%90%88%E4%BD%9C");
    assert_eq!(request.body, json!(null));
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/global/dispose");
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/instance/dispose?directory=%2Ftmp%2Finstance+one");
    server.join();
}

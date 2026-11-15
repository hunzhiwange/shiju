use super::{ShijuCredential, test_support};

#[tokio::test]
async fn shiju_account_facade_forwards_sensitive_request() {
    let server = test_support::server(vec![(200, serde_json::json!({"status": "pending"}))]);
    server.client().shiju_account_login_poll("state").await.expect("poll");
    let request = server.take_request();
    assert_eq!(request.path, "/v1/shiju/account/login/poll");
    assert_eq!(request.body, serde_json::json!({"state": "state"}));
    let credential: gateway_shiju_account::ShijuCredential = ShijuCredential::new("secret").unwrap();
    assert!(!format!("{credential:?}").contains("secret"));
    server.join();
}

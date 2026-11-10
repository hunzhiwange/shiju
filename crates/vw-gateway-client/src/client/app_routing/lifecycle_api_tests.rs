use super::AppRoutingError;
use crate::{GatewayAuth, GatewayClient, client::test_support};
use serde_json::{Value, json};
use vw_api_types::app_routing::RoutingState;

#[tokio::test]
async fn cc_route_010_sdk_round_trip_preserves_status_and_business_errors() {
    let running = json!({
        "state": "running", "boundAddress": "::1", "boundPort": 12345,
        "startedAt": "2026-11-10T12:00:00Z"
    });
    let stopped = json!({
        "state": "stopped", "boundAddress": null, "boundPort": null, "startedAt": null
    });
    let server = test_support::server(vec![
        (200, stopped.clone()),
        (503, json!({"error":{"code":"not_ready","message":"Listener is not running"}})),
        (200, running),
        (200, json!({"ready":true})),
        (409, json!({"error":{"code":"apps_attached","message":"Applications are attached"}})),
        (200, stopped),
    ]);
    let client = GatewayClient::new(
        server
            .client()
            .endpoint()
            .clone()
            .with_auth(GatewayAuth { skey: Some("gateway-fixture".into()) }),
    )
    .unwrap();
    let host = "mac /one?#";
    let token = "fixture-application-management-token";
    assert_eq!(client.app_routing_status(host, token).await.unwrap().state, RoutingState::Stopped);
    assert!(matches!(
        client.app_routing_health(host, token).await,
        Err(AppRoutingError::Http { status: 503, body: Some(body) }) if body.error.code == "not_ready"
    ));
    let started = client.app_routing_start(host, token).await.unwrap();
    assert_eq!(started.state, RoutingState::Running);
    assert_eq!(started.bound_port, Some(12345));
    assert!(client.app_routing_health(host, token).await.unwrap().ready);
    assert!(matches!(
        client.app_routing_stop(host, token).await,
        Err(AppRoutingError::Http { status: 409, body: Some(body) }) if body.error.code == "apps_attached"
    ));
    assert_eq!(client.app_routing_stop(host, token).await.unwrap().state, RoutingState::Stopped);

    for (method, action) in [
        ("GET", "status"),
        ("GET", "health"),
        ("POST", "start"),
        ("GET", "health"),
        ("POST", "stop"),
        ("POST", "stop"),
    ] {
        let request = server.take_request();
        assert_eq!(request.method, method);
        assert_eq!(request.path, format!("/v1/app-routing/hosts/mac%20%2Fone%3F%23/{action}"));
        assert_eq!(request.body, if method == "POST" { json!({}) } else { Value::Null });
        assert_eq!(request.header("x-agent-apps-token"), Some(token));
        assert_eq!(request.header("authorization"), Some("Bearer gateway-fixture"));
        assert!(request.header("x-platform-id").is_some());
        assert!(request.header("x-company-id").is_some());
    }
    server.join();
}

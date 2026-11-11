use super::AppRoutingError;
use crate::client::test_support;
use serde_json::{Value, json};
use vw_api_types::app_routing::{RoutingEgressMode, RoutingEgressUpdate};

#[tokio::test]
async fn cc_route_028_sdk_preserves_egress_status_and_errors() {
    let direct = json!({
        "configuredMode": "system", "effectiveMode": "direct",
        "scheme": null, "hasCredentials": false
    });
    let custom = json!({
        "configuredMode": "custom", "effectiveMode": "custom",
        "scheme": "socks5h", "hasCredentials": true
    });
    let conflict = json!({"error":{"code":"egress_not_applied","message":"Egress update failed"}});
    let mut unexpected = custom.clone();
    unexpected["proxyUrl"] = json!("http://fixture.invalid");
    let server = test_support::server(vec![
        (200, direct),
        (200, custom),
        (409, conflict),
        (200, unexpected),
    ]);
    let client = server.client();
    let host = "mac /?#";
    let token = "fixture-routing-token";
    let url = "socks5h://fixture-user:fixture-password@localhost:1080";
    let update =
        RoutingEgressUpdate { mode: RoutingEgressMode::Custom, proxy_url: Some(url.into()) };

    let saved = client.app_routing_egress(host, token).await.unwrap();
    assert_eq!(saved.configured_mode, RoutingEgressMode::System);
    assert_eq!(saved.effective_mode, RoutingEgressMode::Direct);
    assert_eq!(saved.scheme, None);
    let saved = client.app_routing_set_egress(host, token, &update).await.unwrap();
    assert_eq!(saved.effective_mode, RoutingEgressMode::Custom);
    assert_eq!(saved.scheme.as_deref(), Some("socks5h"));
    assert!(saved.has_credentials);
    assert!(matches!(
        client.app_routing_set_egress(host, token, &update).await,
        Err(AppRoutingError::Http { status: 409, body: Some(body) })
            if body.error.code == "egress_not_applied"
    ));
    assert_eq!(client.app_routing_egress(host, token).await, Err(AppRoutingError::Decode));
    for (method, body) in [
        ("GET", Value::Null),
        ("PUT", json!({"mode":"custom","proxyUrl":url})),
        ("PUT", json!({"mode":"custom","proxyUrl":url})),
        ("GET", Value::Null),
    ] {
        let request = server.take_request();
        assert_eq!(request.method, method);
        assert_eq!(request.path, "/v1/app-routing/hosts/mac%20%2F%3F%23/egress");
        assert_eq!(request.header("x-agent-apps-token"), Some(token));
        assert_eq!(request.body, body);
    }
    server.join();
    let debug = format!("{update:?}");
    assert!(!debug.contains("fixture-password"), "the write-only request must redact credentials");
}

use super::AppRoutingError;
use crate::client::test_support;
use serde_json::{Value, json};
use vw_api_types::{agent_apps::AppId, app_routing::AppRoutingConfig};

#[tokio::test]
async fn cc_route_012_sdk_preserves_config_and_errors() {
    let listener = json!({
        "enabled": false, "listenAddress": "::1", "listenPort": 0, "loggingEnabled": true
    });
    let timeouts = json!({
        "firstByteTimeoutSecs": 120, "idleTimeoutSecs": 0, "nonStreamingTimeoutSecs": 1200
    });
    let invalid =
        json!({"error":{"code":"invalid_input","message":"Invalid routing configuration"}});
    let server = test_support::server(vec![
        (200, listener.clone()),
        (200, listener.clone()),
        (200, timeouts.clone()),
        (200, timeouts.clone()),
        (400, invalid),
        (
            200,
            json!({
                "firstByteTimeoutSecs": 120, "idleTimeoutSecs": 0, "nonStreamingTimeoutSecs": 1200,
                "unexpected": true
            }),
        ),
    ]);
    let client = server.client();
    let host = "mac /?#";
    let token = "fixture-application-routing-token";
    let project = "one /two".into();
    let app = AppId::GrokBuild;

    let saved = client.app_routing_config(host, token).await.unwrap();
    assert_eq!(saved.listen_port, 0);
    assert_eq!(client.app_routing_set_config(host, token, &saved).await.unwrap(), saved);
    let saved = client.app_routing_app_config(host, &project, app, token).await.unwrap();
    assert_eq!(saved.idle_timeout_secs, 0);
    assert_eq!(
        client.app_routing_set_app_config(host, &project, app, token, &saved).await.unwrap(),
        saved
    );
    let invalid = AppRoutingConfig { first_byte_timeout_secs: 0, ..saved };
    assert!(matches!(
        client.app_routing_set_app_config(host, &project, app, token, &invalid).await,
        Err(AppRoutingError::Http { status: 400, body: Some(body) }) if body.error.code == "invalid_input"
    ));
    assert_eq!(
        client.app_routing_app_config(host, &project, app, token).await,
        Err(AppRoutingError::Decode),
        "SDK must reject a config response with an unknown field"
    );

    let base = "/v1/app-routing/hosts/mac%20%2F%3F%23";
    let app_path = "/projects/one%20%2Ftwo/apps/grokbuild/config";
    for (method, suffix, body) in [
        ("GET", "/config", Value::Null),
        ("PUT", "/config", listener),
        ("GET", app_path, Value::Null),
        ("PUT", app_path, timeouts),
        ("PUT", app_path, serde_json::to_value(invalid).unwrap()),
        ("GET", app_path, Value::Null),
    ] {
        let request = server.take_request();
        assert_eq!(request.method, method);
        assert_eq!(request.path, format!("{base}{suffix}"));
        assert_eq!(request.header("x-agent-apps-token"), Some(token));
        assert_eq!(request.body, body);
    }
    server.join();
}

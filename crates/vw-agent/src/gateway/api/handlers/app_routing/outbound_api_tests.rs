use super::lifecycle_api_tests::{
    HOST, MANAGER, MEMBER, READER, assert_error, client, spawn_gateway, write_policy,
};
use crate::global_database::GlobalDatabase;
use reqwest::Method;
use serde_json::{Value, json};
use vw_api_types::app_routing::{RoutingEgressMode, RoutingEgressUpdate};

async fn request(
    port: u16,
    method: Method,
    token: Option<&str>,
    body: Option<Value>,
) -> reqwest::Response {
    let mut request = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap()
        .request(method, format!("http://127.0.0.1:{port}/v1/app-routing/hosts/{HOST}/egress"))
        .header("x-platform-id", "p")
        .header("x-company-id", "owner");
    if let Some(token) = token {
        request = request.header("x-agent-apps-token", token);
    }
    if let Some(body) = body {
        request = request.json(&body);
    }
    request.send().await.unwrap()
}

#[tokio::test]
async fn cc_route_028_outbound_api_expected_behavior() {
    let home = tempfile::tempdir().unwrap();
    let policy = home.path().join("access.json");
    write_policy(&policy, home.path(), 0);
    let (gateway, port) = spawn_gateway(home.path(), &policy).await;
    let owner = client(port, "owner");
    let direct = RoutingEgressUpdate { mode: RoutingEgressMode::Direct, proxy_url: None };
    let secret = "fixture-password";
    let custom = json!({"mode":"custom", "proxyUrl": format!("http://fixture-user:{secret}@127.0.0.1:7890")});

    let response = request(port, Method::PUT, Some(MANAGER), Some(custom.clone())).await;
    assert_eq!(response.status(), 200, "an authorized, valid custom egress must be published");
    assert_eq!(response.headers()["cache-control"], "no-store");
    let redacted = json!({
        "configuredMode":"custom", "effectiveMode":"custom", "scheme":"http", "hasCredentials":true
    });
    assert_eq!(response.json::<Value>().await.unwrap(), redacted);
    let saved = owner.app_routing_egress(HOST, READER).await.unwrap();
    assert_eq!(serde_json::to_value(&saved).unwrap(), redacted);

    let unauthorized = request(port, Method::PUT, None, Some(custom.clone())).await;
    assert_eq!(unauthorized.status(), 401);
    assert_error(owner.app_routing_egress(HOST, MEMBER).await.unwrap_err(), 403, "forbidden");
    assert_error(
        owner.app_routing_egress("other-mac", READER).await.unwrap_err(),
        403,
        "forbidden",
    );
    assert_error(
        client(port, "other").app_routing_egress(HOST, READER).await.unwrap_err(),
        403,
        "forbidden",
    );
    assert_error(
        owner.app_routing_set_egress(HOST, READER, &direct).await.unwrap_err(),
        403,
        "forbidden",
    );

    for invalid in [
        json!({"mode":"custom","proxyUrl":format!("ftp://fixture-user:{secret}@127.0.0.1")}),
        json!({"mode":"custom"}),
        json!({"mode":"direct","proxyUrl":"http://127.0.0.1:7890"}),
        json!({"mode":"system","unexpected":true}),
        json!(["direct", null]),
    ] {
        let response = request(port, Method::PUT, Some(MANAGER), Some(invalid)).await;
        assert_eq!(response.status(), 400);
        let text = response.text().await.unwrap();
        assert!(!text.contains(secret));
        assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["error"]["code"], "invalid_input");
    }
    assert_eq!(owner.app_routing_egress(HOST, READER).await.unwrap(), saved);

    let db = GlobalDatabase::for_home(home.path()).unwrap().connect_sqlite().unwrap();
    let payload: String = db.query_row(
        "SELECT payload_json FROM gateway_settings WHERE namespace='agent_apps.routing.egress.v1'",
        [],
        |row| row.get(0),
    ).unwrap();
    assert!(!payload.contains(secret));
    db.execute_batch(
        "CREATE TRIGGER reject_egress BEFORE UPDATE ON gateway_settings
         WHEN NEW.namespace = 'agent_apps.routing.egress.v1'
         BEGIN SELECT RAISE(ABORT, 'fixture write failure'); END;",
    )
    .unwrap();
    assert_error(
        owner.app_routing_set_egress(HOST, MANAGER, &direct).await.unwrap_err(),
        409,
        "egress_not_applied",
    );
    assert_eq!(owner.app_routing_egress(HOST, READER).await.unwrap(), saved);
    db.execute_batch("DROP TRIGGER reject_egress;").unwrap();
    drop(db);
    drop(gateway);

    let (_gateway, port) = spawn_gateway(home.path(), &policy).await;
    let owner = client(port, "owner");
    assert_eq!(owner.app_routing_egress(HOST, READER).await.unwrap(), saved);
    let status = owner.app_routing_set_egress(HOST, MANAGER, &direct).await.unwrap();
    assert_eq!(status.configured_mode, RoutingEgressMode::Direct);
    assert_eq!(status.effective_mode, RoutingEgressMode::Direct);
    assert_eq!(status.scheme, None);
    assert!(!status.has_credentials);

    let system = RoutingEgressUpdate { mode: RoutingEgressMode::System, proxy_url: None };
    let status = owner.app_routing_set_egress(HOST, MANAGER, &system).await.unwrap();
    assert_eq!(status.configured_mode, RoutingEgressMode::System);
    assert_eq!(status.scheme.is_some(), status.effective_mode == RoutingEgressMode::System);
    assert_eq!(owner.app_routing_egress(HOST, READER).await.unwrap(), status);
}

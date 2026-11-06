use super::{AgentAppsReadError, GatewayClient, test_support};
use crate::endpoint::{GatewayAuth, GatewayEndpoint};
use serde_json::json;
use vw_api_types::{
    agent_apps::{AppId, AppReadResponse},
    id::TenantScope,
};

#[tokio::test]
async fn cc_apps_read_sdk_roundtrip_success_and_query_encoding() {
    let payload = json!({"installations":[],"desktop":{"supported":true,"appBundles":["/Applications/应用.app"],"configDirectories":[]}});
    let server = test_support::server_with_tenant_scope(
        vec![
            (
                200,
                json!({"apps":[{"id":"claude-desktop","name":"桌面","cliCommand":null,"runtimeKinds":["desktop_app"]}]}),
            ),
            (200, payload.clone()),
            (200, payload.clone()),
            (200, payload.clone()),
        ],
        TenantScope::new("p", "c").unwrap(),
    );
    let client = GatewayClient::new(
        server
            .client()
            .endpoint()
            .clone()
            .with_auth(GatewayAuth { skey: Some("read-token".into()) }),
    )
    .unwrap();
    assert_eq!(
        client.agent_apps_catalog("主机 /?&#%+").await.unwrap().apps[0].id,
        AppId::ClaudeDesktop
    );
    let expected: AppReadResponse = serde_json::from_value(payload).unwrap();
    assert_eq!(
        client.agent_apps_installations("主机 /?&#%+", AppId::ClaudeDesktop).await.unwrap(),
        expected
    );
    assert_eq!(
        client.agent_apps_diagnostics("主机 /?&#%+", AppId::ClaudeDesktop).await.unwrap(),
        expected
    );
    assert_eq!(
        client.agent_apps_versions("主机 /?&#%+", AppId::ClaudeDesktop).await.unwrap(),
        expected
    );
    for resource in ["catalog", "installations", "diagnostics", "versions"] {
        let request = server.take_request();
        assert_eq!(request.method, "GET");
        let suffix = if resource == "catalog" { "" } else { "&appId=claude-desktop" };
        assert_eq!(
            request.path,
            format!(
                "/v1/agent-apps/{resource}?targetHostId=%E4%B8%BB%E6%9C%BA+%2F%3F%26%23%25%2B{suffix}"
            )
        );
        assert_eq!(request.header("authorization"), Some("Bearer read-token"));
        assert_eq!(request.header("x-platform-id"), Some("p"));
        assert_eq!(request.header("x-company-id"), Some("c"));
        assert_eq!(request.body, serde_json::Value::Null);
    }
    server.join();
}

#[tokio::test]
async fn cc_apps_read_sdk_roundtrip_preserves_errors() {
    for status in [400, 401, 403, 404, 501, 503] {
        let body =
            json!({"code":"host_unavailable","message":"主机不可用","details":{"retry":false}});
        let server = test_support::server(vec![(status, body.clone())]);
        assert_eq!(
            server.client().agent_apps_diagnostics("remote", AppId::Codex).await.unwrap_err(),
            AgentAppsReadError::Http { status, body }
        );
        assert!(server.take_request().path.contains("targetHostId=remote"));
        server.join();
    }
    let server = test_support::server_raw(vec![(503, String::new())]);
    assert_eq!(
        server.client().agent_apps_catalog("offline").await.unwrap_err(),
        AgentAppsReadError::Http { status: 503, body: json!("") }
    );
    server.join();
}

#[tokio::test]
async fn cc_apps_read_sdk_roundtrip_rejects_invalid_payload_and_unreachable_gateway() {
    let server = test_support::server(vec![(200, json!({"unexpected":true}))]);
    assert!(matches!(
        server.client().agent_apps_versions("host", AppId::Codex).await,
        Err(AgentAppsReadError::Decode(_))
    ));
    server.join();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    assert!(matches!(
        client.agent_apps_installations("remote", AppId::Codex).await,
        Err(AgentAppsReadError::Transport(_))
    ));
}

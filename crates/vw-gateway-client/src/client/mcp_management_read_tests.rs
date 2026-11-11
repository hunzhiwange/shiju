use super::{AgentAppsReadError, GatewayClient, test_support};
use crate::endpoint::GatewayAuth;
use serde_json::json;
use vw_api_types::{
    TenantScope,
    mcp_management::{McpAppTarget, McpCatalogResponse, McpNativePreview, McpPresetsResponse},
};

#[tokio::test]
async fn cc_mcp_t40_reads_typed_views_with_scoped_credentials_and_encoded_targets() {
    let catalog = json!({"records":[{"record":{"id":"docs","revision":2},"scope":{"kind":"tenant","scope":{"platform_id":"p","company_id":"c"}},"name":"文档","transport":"stdio","connectionRevision":1}],"bindings":[{"record":{"id":"docs","revision":2},"target":{"appId":"codex","installationId":"install","targetHostId":"host"},"enabled":true,"nativeStatus":"not_verified"}],"capabilities":[{"appId":"agy","stdio":"supported","streamableHttp":"supported","sse":"unsupported"}]});
    let presets = json!({"presets":[{"id":"docs","name":"文档","tags":[],"homepage":"https://example.com","documentation":"https://example.com/docs","connection":{"transport":"stdio","command":"npx","args":["fixture"]},"enabled":false}]});
    let native = json!({"target":{"appId":"claude","installationId":"安装 /?&+","targetHostId":"主机 /?&+"},"status":"present","entries":[{"id":"docs","status":"present","enabled":false,"transport":"sse"}]});
    let server = test_support::server_with_tenant_scope(
        vec![(200, catalog.clone()), (200, presets.clone()), (200, native.clone())],
        TenantScope::new("p", "c").unwrap(),
    );
    let client = GatewayClient::new(
        server
            .client()
            .endpoint()
            .clone()
            .with_auth(GatewayAuth { skey: Some("fixture-skey".into()) }),
    )
    .unwrap();
    assert_eq!(
        client.mcp_management_catalog("主机 /?&+", "fixture-grant").await.unwrap(),
        serde_json::from_value::<McpCatalogResponse>(catalog).unwrap()
    );
    let request = server.take_request();
    assert_eq!(request.header("x-agent-apps-token"), Some("fixture-grant"));
    assert_eq!(request.path, "/v1/agent-apps/mcp?targetHostId=%E4%B8%BB%E6%9C%BA+%2F%3F%26%2B");
    assert!(
        client.mcp_management_presets("主机 /?&+", "fixture-grant").await.unwrap()
            == serde_json::from_value::<McpPresetsResponse>(presets).unwrap()
    );
    let preset_request = server.take_request();
    assert_eq!(
        preset_request.path,
        "/v1/agent-apps/mcp/presets?targetHostId=%E4%B8%BB%E6%9C%BA+%2F%3F%26%2B"
    );
    let target: McpAppTarget = serde_json::from_value(native["target"].clone()).unwrap();
    assert_eq!(
        client.mcp_management_native(&target, Some("账号 /?&+"), "fixture-grant").await.unwrap(),
        serde_json::from_value::<McpNativePreview>(native).unwrap()
    );
    let native_request = server.take_request();
    assert_eq!(
        native_request.path,
        "/v1/agent-apps/mcp/native?targetHostId=%E4%B8%BB%E6%9C%BA+%2F%3F%26%2B&appId=claude&installationId=%E5%AE%89%E8%A3%85+%2F%3F%26%2B&accountId=%E8%B4%A6%E5%8F%B7+%2F%3F%26%2B"
    );
    for request in [request, preset_request, native_request] {
        assert_eq!(request.method, "GET");
        assert_eq!(request.header("authorization"), Some("Bearer fixture-skey"));
        assert_eq!(request.header("x-agent-apps-token"), Some("fixture-grant"));
        assert_eq!(request.header("x-platform-id"), Some("p"));
        assert_eq!(request.header("x-company-id"), Some("c"));
        assert!(request.body.is_null());
    }
    server.join();
}

#[tokio::test]
async fn cc_mcp_t40_preserves_http_errors_and_rejects_malformed_views() {
    for status in [400, 401, 403, 404, 409, 422, 501, 503, 307] {
        let body = json!({"code":"permission_denied"});
        let server = test_support::server(vec![(status, body.clone())]);
        assert_eq!(
            server.client().mcp_management_catalog("remote", "fixture-grant").await.unwrap_err(),
            AgentAppsReadError::Http { status, body }
        );
        assert!(server.take_request().path.contains("targetHostId=remote"));
        server.join();
    }
    let server = test_support::server_raw(vec![
        (503, "unavailable".into()),
        (200, "{}".into()),
        (200, "not JSON".into()),
    ]);
    assert_eq!(
        server.client().mcp_management_catalog("remote", "fixture-grant").await.unwrap_err(),
        AgentAppsReadError::Http { status: 503, body: json!("unavailable") }
    );
    assert!(matches!(
        server.client().mcp_management_catalog("remote", "fixture-grant").await,
        Err(AgentAppsReadError::Decode(_))
    ));
    assert!(matches!(
        server.client().mcp_management_presets("remote", "fixture-grant").await,
        Err(AgentAppsReadError::Decode(_))
    ));
    server.join();
}

#[tokio::test]
async fn cc_mcp_t40_native_without_account_preserves_unsupported_and_rejects_private_fields() {
    let target: McpAppTarget = serde_json::from_value(
        json!({"appId":"openclaw","installationId":"install","targetHostId":"remote"}),
    )
    .unwrap();
    let native = json!({"target":target,"status":"unsupported","entries":[]});
    let mut private = native.clone();
    private["connection"] = json!({"command":"private"});
    let server = test_support::server(vec![(200, native), (200, private)]);
    assert_eq!(
        server.client().mcp_management_native(&target, None, "fixture-grant").await.unwrap().status,
        vw_api_types::mcp_management::McpNativeStatus::Unsupported
    );
    assert_eq!(
        server.take_request().path,
        "/v1/agent-apps/mcp/native?targetHostId=remote&appId=openclaw&installationId=install"
    );
    assert!(matches!(
        server.client().mcp_management_native(&target, None, "fixture-grant").await,
        Err(AgentAppsReadError::Decode(_))
    ));
    server.join();
}

#[tokio::test]
async fn cc_mcp_t40_invalid_credential_and_unreachable_gateway_return_transport_errors() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let client =
        GatewayClient::new(crate::endpoint::GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    assert!(matches!(
        client.mcp_management_catalog("remote", "fixture\ninjected").await,
        Err(AgentAppsReadError::Transport(_))
    ));
    assert!(matches!(
        client.mcp_management_presets("remote", "fixture-grant").await,
        Err(AgentAppsReadError::Transport(_))
    ));
}

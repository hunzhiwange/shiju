use super::{AgentAppsSettingsError, GatewayClient, test_support};
use crate::endpoint::GatewayAuth;
use serde_json::json;
use vw_api_types::{TenantScope, mcp_management::*};

fn create_request() -> McpCreateRequest {
    serde_json::from_value(json!({"targetHostId":"主机 /?&+","projectId":"project","id":"文档 /?&+","name":"文档","tags":["docs"],"connection":{"transport":"streamable_http","url":"https://example.com/mcp","headers":{"Authorization":{"kind":"credential","reference":{"service":"claude_official","accountId":"fixture-account"}}}}})).unwrap()
}

fn record(revision: u64) -> serde_json::Value {
    json!({"record":{"id":"文档 /?&+","revision":revision},"scope":{"kind":"project","scope":{"platform_id":"p","company_id":"c","project_id":"project"}},"name":"文档","transport":"streamable_http","connectionRevision":revision})
}

#[tokio::test]
async fn cc_mcp_t41_create_sends_typed_definition_and_scoped_credentials() {
    let response = record(1);
    let server = test_support::server_with_tenant_scope(
        vec![(201, response.clone())],
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
    let body = create_request();
    assert_eq!(
        client.mcp_management_create(&body, "fixture-grant").await.unwrap(),
        serde_json::from_value::<McpCatalogRecord>(response).unwrap()
    );
    let request = server.take_request();
    assert_eq!(request.header("x-agent-apps-token"), Some("fixture-grant"));
    assert_eq!(request.header("authorization"), Some("Bearer fixture-skey"));
    assert_eq!(request.header("x-platform-id"), Some("p"));
    assert_eq!(request.header("x-company-id"), Some("c"));
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/agent-apps/mcp");
    assert_eq!(request.body, serde_json::to_value(body).unwrap());
    server.join();
}

#[tokio::test]
async fn cc_mcp_t41_update_and_delete_preserve_revision_and_partial_cleanup() {
    let response = record(8);
    let result = json!({"outcome":"partially_succeeded","items":[
        {"record":{"id":"文档 /?&+","revision":8},"target":{"appId":"codex","installationId":"one","targetHostId":"remote"},"enabled":false,"desiredCommitted":true,"nativeOutcome":{"status":"applied"}},
        {"record":{"id":"文档 /?&+","revision":8},"target":{"appId":"claude","installationId":"two","targetHostId":"remote"},"enabled":false,"desiredCommitted":true,"nativeOutcome":{"status":"failed","code":"credential_unavailable"}}
    ]});
    let server = test_support::server(vec![(200, response.clone()), (200, result.clone())]);
    let id = McpRecordId("文档 /?&+".into());
    let update: McpUpdateRequest = serde_json::from_value(json!({"targetHostId":"remote","projectId":"project","expectedRevision":7,"connection":{"transport":"stdio","command":"fixture","args":["--docs"],"env":{"MODE":{"kind":"literal","value":"docs"}}}})).unwrap();
    assert_eq!(
        server.client().mcp_management_update(&id, &update, "fixture-grant").await.unwrap(),
        serde_json::from_value::<McpCatalogRecord>(response).unwrap()
    );
    let delete = McpDeleteRequest {
        target_host_id: "remote".into(),
        project_id: None,
        expected_revision: 8,
    };
    let actual =
        server.client().mcp_management_delete(&id, &delete, "fixture-grant").await.unwrap();
    assert_eq!(actual, serde_json::from_value::<McpDistributionResult>(result).unwrap());
    assert_eq!(actual.outcome, McpBatchOutcome::PartiallySucceeded);
    assert!(actual.items.iter().all(|item| item.desired_committed));
    for (method, body) in [
        ("PUT", serde_json::to_value(&update).unwrap()),
        ("DELETE", serde_json::to_value(&delete).unwrap()),
    ] {
        let request = server.take_request();
        assert_eq!(request.method, method);
        assert_eq!(request.path, "/v1/agent-apps/mcp/%E6%96%87%E6%A1%A3%20%2F%3F&+");
        assert_eq!(request.body, body);
        assert_eq!(request.header("x-agent-apps-token"), Some("fixture-grant"));
    }
    server.join();
}

#[tokio::test]
async fn cc_mcp_t41_preserves_http_errors_and_rejects_malformed_success() {
    let delete = McpDeleteRequest {
        target_host_id: "remote".into(),
        project_id: None,
        expected_revision: 9,
    };
    let id = McpRecordId("docs".into());
    for status in [400, 401, 403, 404, 409, 422, 503, 307] {
        let body = json!({"code":"stale_revision"});
        let server = test_support::server(vec![(status, body.clone())]);
        assert_eq!(
            server.client().mcp_management_delete(&id, &delete, "fixture-grant").await.unwrap_err(),
            AgentAppsSettingsError::Http { status, body }
        );
        assert_eq!(server.take_request().method, "DELETE");
        server.join();
    }
    let server = test_support::server_raw(vec![
        (503, "unavailable".into()),
        (200, "{}".into()),
        (201, "not JSON".into()),
    ]);
    assert_eq!(
        server.client().mcp_management_delete(&id, &delete, "fixture-grant").await.unwrap_err(),
        AgentAppsSettingsError::Http { status: 503, body: json!("unavailable") }
    );
    assert!(matches!(
        server.client().mcp_management_delete(&id, &delete, "fixture-grant").await,
        Err(AgentAppsSettingsError::Decode(_))
    ));
    assert!(matches!(
        server.client().mcp_management_create(&create_request(), "fixture-grant").await,
        Err(AgentAppsSettingsError::Decode(_))
    ));
    server.join();
}

#[tokio::test]
async fn cc_mcp_t41_rejects_invalid_credential_and_path_identity_without_fallback() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let client =
        GatewayClient::new(crate::endpoint::GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    let delete = McpDeleteRequest {
        target_host_id: "remote".into(),
        project_id: None,
        expected_revision: 1,
    };
    for id in ["", ".", ".."] {
        assert_eq!(
            client
                .mcp_management_delete(&McpRecordId(id.into()), &delete, "fixture-grant")
                .await
                .unwrap_err(),
            AgentAppsSettingsError::Transport("invalid MCP record identity".into())
        );
    }
    assert_eq!(
        client.mcp_management_create(&create_request(), "bad\ncredential").await.unwrap_err(),
        AgentAppsSettingsError::Transport("invalid application credential".into())
    );
    assert!(matches!(
        client.mcp_management_create(&create_request(), "fixture-grant").await,
        Err(AgentAppsSettingsError::Transport(_))
    ));
}

#[tokio::test]
async fn cc_mcp_t41_existing_mcp_inspection_and_bound_call_keep_their_routes() {
    let server = test_support::server(vec![
        (
            200,
            json!({"servers":[{"name":"docs","transport":"http","enabled":true,"required":false,"connection_status":"connected","tool_count":0,"resource_count":0,"prompt_count":0,"tools":[],"resources":[],"prompts":[]}]}),
        ),
        (200, json!({"content":[]})),
    ]);
    assert_eq!(server.client().mcp_inspect().await.unwrap().servers[0].name, "docs");
    let inspect = server.take_request();
    assert_eq!(inspect.method, "GET");
    assert_eq!(inspect.path, "/v1/mcp/inspect");
    assert!(inspect.body.is_null());
    assert_eq!(
        server
            .client()
            .mcp_app_call("session", "docs", "binding", "search", &json!({"query":"fixture"}))
            .await
            .unwrap(),
        json!({"content":[]})
    );
    let call = server.take_request();
    assert_eq!(call.method, "POST");
    assert_eq!(call.path, "/v1/mcp/apps/call");
    assert_eq!(
        call.body,
        json!({"session":"session","server":"docs","binding":"binding","name":"search","arguments":{"query":"fixture"}})
    );
    server.join();
}

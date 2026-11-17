use super::*;
use crate::{server_summary};
use agent_client_protocol::{McpServerHttp, HttpHeader};
use rmcp::model::{ClientJsonRpcMessage, CreateElicitationRequest, CreateMessageRequest, ElicitationSchema, ErrorCode, ListRootsRequest, PromptListChangedNotification, RequestId, ResourceListChangedNotification, Role, SamplingMessage, ServerJsonRpcMessage, ServerNotification, ServerRequest, ToolListChangedNotification};
use rmcp::transport::OneshotTransport;

async fn run_noop_client_request(request: ServerRequest, id: i64) -> rmcp::model::ErrorData {
    let (transport, mut responses) =
        OneshotTransport::new(ServerJsonRpcMessage::request(request, RequestId::Number(id)));
    let running: rmcp::service::RunningService<RoleClient, NoopClient> =
        rmcp::service::serve_directly(NoopClient::default(), transport, None);
    let response = tokio::time::timeout(Duration::from_secs(1), responses.recv())
        .await
        .expect("noop response timeout")
        .expect("noop response");
    let _ = running.cancel().await;

    match response {
        ClientJsonRpcMessage::Error(error) => error.error,
        other => panic!("expected error response, got {other:?}"),
    }
}

async fn run_noop_client_notification(
    notification: ServerNotification,
) -> Option<ClientJsonRpcMessage> {
    let (transport, mut responses) =
        OneshotTransport::new(ServerJsonRpcMessage::notification(notification));
    let running: rmcp::service::RunningService<RoleClient, NoopClient> =
        rmcp::service::serve_directly(NoopClient::default(), transport, None);
    let response =
        tokio::time::timeout(Duration::from_millis(50), responses.recv()).await.ok().flatten();
    let _ = running.cancel().await;
    response
}

#[test]
fn mcp_common_remote_headers_are_preserved_for_transport() {
    let headers = vec![
        HttpHeader::new("Authorization", "Bearer test-token"),
        HttpHeader::new("X-Trace-Id", "trace-123"),
    ];
    let header_map = remote_header_map(&headers).expect("headers should be valid");

    assert_eq!(header_map.get("authorization").expect("authorization header"), "Bearer test-token");
    assert_eq!(header_map.get("x-trace-id").expect("trace header"), "trace-123");

    let server =
        McpServer::Http(McpServerHttp::new("gateway", "https://example.com/mcp").headers(headers));
    let summary = server_summary(&server);
    assert_eq!(
        summary,
        json!({
            "name": "gateway",
            "transport": "http",
            "url": "https://example.com/mcp"
        })
    );
    assert!(!summary.to_string().contains("Bearer test-token"));
}

#[test]
fn mcp_common_remote_headers_reject_invalid_names_and_values() {
    let invalid_name = remote_header_map(&[HttpHeader::new("invalid name", "value")])
        .expect_err("header names containing spaces should be rejected");
    assert!(invalid_name.to_string().contains("invalid MCP remote header name 'invalid name'"));

    let invalid_value = remote_header_map(&[HttpHeader::new("X-Test", "line-one\nline-two")])
        .expect_err("header values containing newlines should be rejected");
    assert!(invalid_value.to_string().contains("invalid MCP remote header value for 'X-Test'"));
}

#[test]
fn mcp_common_remote_http_client_accepts_valid_headers() {
    remote_http_client(
        &[
            HttpHeader::new("Authorization", "Bearer test-token"),
            HttpHeader::new("X-Client", "vw-agent"),
        ],
        Duration::from_millis(250),
    )
    .expect("valid remote client headers should build a reqwest client");
}

#[test]
fn mcp_capability_client_declares_only_supported_capabilities() {
    let info = <NoopClient as rmcp::ClientHandler>::get_info(&NoopClient::default());
    let capabilities = serde_json::to_value(&info.capabilities).expect("capabilities json");

    assert_eq!(capabilities, json!({}));
    assert!(info.capabilities.sampling.is_none());
    assert!(info.capabilities.elicitation.is_none());
    assert!(info.capabilities.roots.is_none());
}

#[test]
fn mcp_unsupported_capabilities_report_stable_policy_details() {
    assert_eq!(
        unsupported_mcp_capabilities_json(),
        json!([
            {
                "scope": "client",
                "capability": "sampling",
                "reason": "vw-agent MCP client does not broker model sampling requests from servers."
            },
            {
                "scope": "client",
                "capability": "elicitation",
                "reason": "vw-agent MCP client does not collect interactive user input for servers."
            },
            {
                "scope": "client",
                "capability": "roots",
                "reason": "vw-agent MCP client does not expose workspace roots through the MCP roots API."
            },
            {
                "scope": "server",
                "capability": "resources.subscribe",
                "reason": "vw-agent MCP client performs one-shot resource reads and does not keep resource subscriptions open."
            }
        ])
    );
}

#[test]
fn mcp_unsupported_client_capability_error_identifies_rejected_capability() {
    let error = unsupported_client_capability_error("sampling");

    assert_eq!(error.code, ErrorCode::INVALID_REQUEST);
    assert_eq!(error.message, "MCP client capability 'sampling' is not supported by vw-agent");
    assert_eq!(error.data, Some(json!({ "capability": "sampling" })));
}

#[tokio::test]
async fn noop_client_rejects_unsupported_server_requests_through_rmcp_transport() {
    let sampling_error = run_noop_client_request(
        ServerRequest::CreateMessageRequest(CreateMessageRequest::new(CreateMessageRequestParam {
            messages: vec![SamplingMessage { role: Role::User, content: rmcp::model::Content::text("hello") }],
            model_preferences: None,
            system_prompt: None,
            include_context: None,
            temperature: None,
            max_tokens: 8,
            stop_sequences: None,
            metadata: None,
        })),
        1,
    )
    .await;
    assert_eq!(sampling_error.code, ErrorCode::INVALID_REQUEST);
    assert_eq!(sampling_error.data, Some(json!({ "capability": "sampling" })));

    let roots_error =
        run_noop_client_request(ServerRequest::ListRootsRequest(ListRootsRequest::default()), 2)
            .await;
    assert_eq!(roots_error.code, ErrorCode::INVALID_REQUEST);
    assert_eq!(roots_error.data, Some(json!({ "capability": "roots" })));

    let elicitation_error = run_noop_client_request(
        ServerRequest::CreateElicitationRequest(CreateElicitationRequest::new(
            CreateElicitationRequestParam {
                message: "Pick a value".to_string(),
                requested_schema: ElicitationSchema::new(Default::default()),
            },
        )),
        3,
    )
    .await;
    assert_eq!(elicitation_error.code, ErrorCode::INVALID_REQUEST);
    assert_eq!(elicitation_error.data, Some(json!({ "capability": "elicitation" })));
}

#[tokio::test]
async fn noop_client_ignores_unadvertised_list_changed_notifications() {
    assert!(
        run_noop_client_notification(ServerNotification::ResourceListChangedNotification(
            ResourceListChangedNotification::default(),
        ))
        .await
        .is_none()
    );
    assert!(
        run_noop_client_notification(ServerNotification::ToolListChangedNotification(
            ToolListChangedNotification::default(),
        ))
        .await
        .is_none()
    );
    assert!(
        run_noop_client_notification(ServerNotification::PromptListChangedNotification(
            PromptListChangedNotification::default(),
        ))
        .await
        .is_none()
    );
}

#[tokio::test]
async fn mcp_connect_timeout_preserves_completed_operation_results() {
    let value = with_connect_timeout("fast-mcp", Duration::from_secs(1), async {
        Ok::<_, anyhow::Error>(42)
    })
    .await
    .expect("completed operation should return its value");
    assert_eq!(value, 42);

    let error = with_connect_timeout("fast-mcp", Duration::from_secs(1), async {
        Err::<(), _>(anyhow::anyhow!("operation failed"))
    })
    .await
    .expect_err("completed operation error should be preserved");
    assert_eq!(error.to_string(), "operation failed");
}

#[tokio::test]
async fn mcp_connect_timeout_reports_server_and_duration() {
    let error = with_connect_timeout(
        "slow-mcp",
        Duration::ZERO,
        std::future::pending::<anyhow::Result<()>>(),
    )
    .await
    .expect_err("pending operation should exceed a zero duration timeout");

    assert_eq!(error.to_string(), "timed out connecting to MCP server 'slow-mcp' after 0 ms");
}

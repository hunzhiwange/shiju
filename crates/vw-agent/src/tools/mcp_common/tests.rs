//! MCP 工具共享逻辑测试。
//!
//! 这些测试覆盖 server 摘要、连接错误传播、资源读取输入校验以及 MCP 工具别名，
//! 确保多个 MCP 工具依赖的公共行为保持一致。

use super::super::*;
use super::*;
use crate::app::agent::config::{Config, schema::ConfigExt};
use crate::plugins::PluginDiscoveryOptions;
use agent_client_protocol::{
    EnvVariable, HttpHeader, McpServer, McpServerHttp, McpServerSse, McpServerStdio,
};
use rmcp::model::{
    ClientJsonRpcMessage, Content, CreateElicitationRequest, CreateElicitationRequestParam,
    CreateMessageRequest, CreateMessageRequestParam, ElicitationSchema, ErrorCode,
    ListRootsRequest, PromptListChangedNotification, RequestId, ResourceContents,
    ResourceListChangedNotification, Role, SamplingMessage, ServerJsonRpcMessage,
    ServerNotification, ServerRequest, ToolListChangedNotification,
};
use rmcp::service::RoleClient;
use rmcp::transport::OneshotTransport;
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;
use vw_config_types::plugins::PluginsConfig;

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner())
}

struct EnvVarGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var(key).ok();
        unsafe { std::env::set_var(key, value) };
        Self { key, original }
    }

    fn unset(key: &'static str) -> Self {
        let original = std::env::var(key).ok();
        unsafe { std::env::remove_var(key) };
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.original {
            unsafe { std::env::set_var(self.key, value) };
        } else {
            unsafe { std::env::remove_var(self.key) };
        }
    }
}

struct CurrentDirGuard {
    original: PathBuf,
}

impl CurrentDirGuard {
    fn set(path: &Path) -> Self {
        let original = env::current_dir().expect("current dir");
        env::set_current_dir(path).expect("set current dir");
        Self { original }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        let _ = env::set_current_dir(&self.original);
    }
}

fn write_json(path: &Path, value: serde_json::Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("json parent");
    }
    fs::write(path, serde_json::to_vec_pretty(&value).expect("json bytes")).expect("write json");
}

fn write_plugin_manifest(plugin_root: &Path, name: &str, mcp_path: &str) {
    let manifest_dir = plugin_root.join(".vibewindow-plugin");
    fs::create_dir_all(&manifest_dir).expect("manifest dir");
    fs::write(
        manifest_dir.join("plugin.json"),
        serde_json::to_vec_pretty(&json!({
            "name": name,
            "version": "0.1.0",
            "description": format!("{name} test plugin"),
            "mcpServers": mcp_path,
        }))
        .expect("manifest json"),
    )
    .expect("write manifest");
}

fn server_names(servers: &[McpServer]) -> Vec<&str> {
    servers.iter().map(server_name).collect()
}

fn resolved_config_with_servers(servers: Vec<McpServer>) -> vw_acp::ResolvedAcpxConfig {
    vw_acp::ResolvedAcpxConfig {
        default_agent: "main".to_string(),
        default_permissions: vw_acp::PermissionMode::ApproveAll,
        non_interactive_permissions: vw_acp::NonInteractivePermissionPolicy::Deny,
        auth_policy: vw_acp::AuthPolicy::Skip,
        ttl_ms: 300_000,
        timeout_ms: None,
        queue_max_depth: 16,
        format: vw_acp::OutputFormat::Text,
        agents: HashMap::new(),
        auth: HashMap::new(),
        disable_exec: false,
        mcp_servers: servers,
        mcp_server_management: Vec::new(),
        global_path: "global.json".to_string(),
        project_path: ".vwacprc.json".to_string(),
        has_global_config: false,
        has_project_config: false,
    }
}

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
fn server_helpers_preserve_transport_details() {
    let http = McpServer::Http(McpServerHttp::new("gateway", "https://example.com/mcp"));
    assert_eq!(server_name(&http), "gateway");
    assert_eq!(
        server_summary(&http),
        json!({
            "name": "gateway",
            "transport": "http",
            "url": "https://example.com/mcp"
        })
    );

    let sse = McpServer::Sse(McpServerSse::new("events", "https://example.com/sse"));
    assert_eq!(server_name(&sse), "events");
    assert_eq!(
        server_summary(&sse),
        json!({
            "name": "events",
            "transport": "sse",
            "url": "https://example.com/sse"
        })
    );

    let stdio = McpServer::Stdio(
        McpServerStdio::new("local", PathBuf::from("npx")).args(vec!["mcp".into()]),
    );
    assert_eq!(server_name(&stdio), "local");
    assert_eq!(
        server_summary(&stdio),
        json!({
            "name": "local",
            "transport": "stdio",
            "command": "npx",
            "args": ["mcp"]
        })
    );
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

#[tokio::test]
async fn with_server_peer_remote_transports_report_header_build_errors_before_connecting() {
    let http = McpServer::Http(
        McpServerHttp::new("bad-http", "https://example.com/mcp")
            .headers(vec![HttpHeader::new("invalid header", "value")]),
    );
    let http_error = with_server_peer_timeout(&http, Some(Duration::from_millis(50)), |_peer| {
        Box::pin(async move { Ok(()) })
    })
    .await
    .expect_err("invalid HTTP headers should fail before connecting");
    let http_message = format!("{http_error:#}");
    assert!(http_message.contains("failed to build HTTP MCP client for 'bad-http'"));
    assert!(http_message.contains("invalid MCP remote header name 'invalid header'"));

    let sse = McpServer::Sse(
        McpServerSse::new("bad-sse", "https://example.com/sse")
            .headers(vec![HttpHeader::new("X-Test", "line-one\nline-two")]),
    );
    let sse_error = with_server_peer_timeout(&sse, Some(Duration::from_millis(50)), |_peer| {
        Box::pin(async move { Ok(()) })
    })
    .await
    .expect_err("invalid SSE headers should fail before connecting");
    let sse_message = format!("{sse_error:#}");
    assert!(sse_message.contains("failed to build SSE MCP client for 'bad-sse'"));
    assert!(sse_message.contains("invalid MCP remote header value for 'X-Test'"));
}

#[test]
fn mcp_common_server_lookup_rejects_unknown_names() {
    let servers = vec![
        McpServer::Http(McpServerHttp::new("gateway", "https://example.com/mcp")),
        McpServer::Sse(McpServerSse::new("events", "https://example.com/sse")),
    ];

    let found = find_server_by_name(&servers, "events").expect("known server should be found");
    assert_eq!(server_name(found), "events");

    let error = find_server_by_name(&servers, "missing")
        .expect_err("unknown server name should be rejected");
    assert_eq!(error.to_string(), "unknown MCP server 'missing'");
}

#[test]
fn mcp_common_server_lookup_trims_names_and_rejects_blank_input() {
    let servers = vec![McpServer::Http(McpServerHttp::new("gateway", "https://example.com/mcp"))];

    let found = find_server_by_name(&servers, "  gateway  ")
        .expect("surrounding whitespace should be ignored");
    assert_eq!(server_name(found), "gateway");

    let error =
        find_server_by_name(&servers, " \t ").expect_err("a blank server name should be rejected");
    assert_eq!(error.to_string(), "MCP server name must not be empty");
}

#[test]
fn global_mcp_servers_append_without_overriding_acp_servers() {
    let mut config = resolved_config_with_servers(vec![McpServer::Stdio(McpServerStdio::new(
        "project-mcp",
        PathBuf::from("project-command"),
    ))]);
    let raw_servers = vec![
        json!({ "name": "project-mcp", "command": "global-ignored" }),
        json!({
            "name": "global-mcp",
            "type": "http",
            "url": "https://example.test/mcp",
            "enabled": false,
            "timeout": 2,
            "allowedTools": ["demo_echo"]
        }),
    ];

    merge_global_mcp_server_values(&mut config, &raw_servers, "vibewindow.json")
        .expect("global MCP should merge");

    assert_eq!(server_names(&config.mcp_servers), vec!["project-mcp", "global-mcp"]);
    let management = config
        .mcp_server_management
        .iter()
        .find(|entry| entry.name == "global-mcp")
        .expect("global MCP management should be preserved");
    assert!(!management.enabled);
    assert_eq!(management.timeout_ms, Some(2_000));
    assert_eq!(management.allowed_tools, vec!["demo_echo"]);
}

#[test]
fn global_mcp_servers_empty_input_preserves_existing_config() {
    let server = McpServer::Stdio(McpServerStdio::new("project-mcp", "project-command"));
    let mut config = resolved_config_with_servers(vec![server.clone()]);

    merge_global_mcp_server_values(&mut config, &[], "vibewindow.json")
        .expect("empty global MCP input should be a no-op");

    assert_eq!(config.mcp_servers, vec![server]);
    assert!(config.mcp_server_management.is_empty());
}

#[test]
fn global_mcp_servers_invalid_entry_reports_source_path() {
    let mut config = resolved_config_with_servers(Vec::new());

    let error = merge_global_mcp_server_values(&mut config, &[json!(null)], "vibewindow.json")
        .expect_err("non-object MCP server entry should be rejected");

    assert_eq!(error.to_string(), "Invalid mcpServers[0] in vibewindow.json: expected object");
    assert!(config.mcp_servers.is_empty());
    assert!(config.mcp_server_management.is_empty());
}

#[test]
fn mcp_server_overrides_update_runtime_management() {
    let mut config = resolved_config_with_servers(vec![McpServer::Stdio(McpServerStdio::new(
        "plugin-mcp",
        PathBuf::from("plugin-command"),
    ))]);
    let raw_overrides = vec![json!({
        "name": "plugin-mcp",
        "enabled": false,
        "required": true,
        "timeout": 4,
        "deniedTools": ["demo_sum"]
    })];

    apply_mcp_server_override_values(&mut config, &raw_overrides, "vibewindow.json")
        .expect("runtime override should apply");

    let management = management_for_server(&config, &config.mcp_servers[0]);
    assert!(!management.enabled);
    assert!(management.required);
    assert_eq!(management.timeout_ms, Some(4_000));
    assert_eq!(management.denied_tools, vec!["demo_sum"]);
}

#[test]
fn mcp_server_override_parser_trims_values_and_rounds_fractional_timeout() {
    let parsed = parse_mcp_server_management_override(
        &json!({
            "name": "  plugin-mcp  ",
            "enabled": true,
            "required": false,
            "timeout": 1.2345,
            "allowedTools": ["  demo_echo  "],
            "deniedTools": []
        }),
        "vibewindow.json",
        0,
    )
    .expect("valid override should parse");

    assert_eq!(parsed.name, "plugin-mcp");
    assert_eq!(parsed.enabled, Some(true));
    assert_eq!(parsed.required, Some(false));
    assert_eq!(parsed.timeout_ms, Some(Some(1_235)));
    assert_eq!(parsed.allowed_tools, Some(vec!["demo_echo".to_string()]));
    assert_eq!(parsed.denied_tools, Some(Vec::new()));
}

#[test]
fn mcp_server_override_parser_rejects_invalid_fields_with_paths() {
    let cases = [
        (json!([]), "expected object"),
        (json!({ "name": "  " }), ".name: expected non-empty string"),
        (json!({ "name": "plugin-mcp", "enabled": "yes" }), ".enabled: expected boolean"),
        (
            json!({ "name": "plugin-mcp", "timeout": 0 }),
            ".timeout: expected positive seconds or null",
        ),
        (
            json!({ "name": "plugin-mcp", "allowedTools": "demo_echo" }),
            ".allowedTools: expected array",
        ),
        (
            json!({ "name": "plugin-mcp", "deniedTools": ["  "] }),
            ".deniedTools[0]: expected non-empty string",
        ),
    ];

    for (index, (value, expected)) in cases.into_iter().enumerate() {
        let error = parse_mcp_server_management_override(&value, "vibewindow.json", index)
            .expect_err("invalid override should be rejected");
        let message = error.to_string();
        assert!(message.contains(&format!("mcpServerOverrides[{index}] in vibewindow.json")));
        assert!(message.contains(expected), "unexpected error: {message}");
    }
}

#[test]
fn mcp_server_override_parser_rejects_wrong_timeout_and_tool_element_types() {
    let cases = [
        (
            json!({ "name": "plugin-mcp", "timeout": "fast" }),
            ".timeout: expected positive seconds or null",
        ),
        (
            json!({ "name": "plugin-mcp", "allowedTools": [7] }),
            ".allowedTools[0]: expected non-empty string",
        ),
    ];

    for (index, (value, expected)) in cases.into_iter().enumerate() {
        let error = parse_mcp_server_management_override(&value, "vibewindow.json", index)
            .expect_err("wrongly typed override field should be rejected");
        assert!(error.to_string().contains(expected));
    }
}

#[test]
fn mcp_server_overrides_skip_unknown_servers() {
    let mut config = resolved_config_with_servers(vec![McpServer::Stdio(McpServerStdio::new(
        "known-mcp",
        "known-command",
    ))]);

    apply_mcp_server_override_values(
        &mut config,
        &[json!({ "name": "missing-mcp", "enabled": false })],
        "vibewindow.json",
    )
    .expect("unknown server override should be ignored");

    assert!(config.mcp_server_management.is_empty());
}

#[test]
fn mcp_server_overrides_update_existing_entry_and_preserve_omitted_fields() {
    let server = McpServer::Stdio(McpServerStdio::new("plugin-mcp", "plugin-command"));
    let mut config = resolved_config_with_servers(vec![server]);
    let mut existing = vw_acp::McpServerManagement::default_for_server("plugin-mcp");
    existing.enabled = false;
    existing.required = true;
    existing.timeout_ms = Some(5_000);
    existing.allowed_tools = vec!["old-allowed".to_string()];
    existing.denied_tools = vec!["old-denied".to_string()];
    config.mcp_server_management.push(existing);

    apply_mcp_server_override_values(
        &mut config,
        &[json!({
            "name": "plugin-mcp",
            "timeout": null,
            "allowedTools": ["new-allowed"]
        })],
        "vibewindow.json",
    )
    .expect("override should update the existing management entry");

    assert_eq!(config.mcp_server_management.len(), 1);
    let management = &config.mcp_server_management[0];
    assert!(!management.enabled);
    assert!(management.required);
    assert_eq!(management.timeout_ms, None);
    assert_eq!(management.allowed_tools, vec!["new-allowed"]);
    assert_eq!(management.denied_tools, vec!["old-denied"]);
}

#[test]
fn mcp_server_overrides_create_management_when_entry_is_missing() {
    let server = McpServer::Stdio(McpServerStdio::new("plugin-mcp", "plugin-command"));
    let mut config = resolved_config_with_servers(vec![server]);

    apply_mcp_server_override_values(
        &mut config,
        &[json!({
            "name": "plugin-mcp",
            "required": true,
            "allowedTools": ["demo_echo"],
            "deniedTools": ["demo_delete"]
        })],
        "vibewindow.json",
    )
    .expect("override should create missing management entry");

    assert_eq!(config.mcp_server_management.len(), 1);
    let management = &config.mcp_server_management[0];
    assert_eq!(management.name, "plugin-mcp");
    assert!(management.enabled);
    assert!(management.required);
    assert_eq!(management.timeout_ms, None);
    assert_eq!(management.allowed_tools, vec!["demo_echo"]);
    assert_eq!(management.denied_tools, vec!["demo_delete"]);
}

#[test]
fn mcp_server_overrides_empty_input_preserves_management() {
    let server = McpServer::Stdio(McpServerStdio::new("plugin-mcp", "plugin-command"));
    let mut config = resolved_config_with_servers(vec![server]);
    let management = vw_acp::McpServerManagement::default_for_server("plugin-mcp");
    config.mcp_server_management.push(management.clone());

    apply_mcp_server_override_values(&mut config, &[], "vibewindow.json")
        .expect("empty overrides should be a no-op");

    assert_eq!(config.mcp_server_management, vec![management]);
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
                "reason": "MCP form input requires an active session UI handler."
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
            messages: vec![SamplingMessage { role: Role::User, content: Content::text("hello") }],
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

#[tokio::test]
async fn with_server_peer_stdio_surfaces_spawn_errors() {
    let server = McpServer::Stdio(
        McpServerStdio::new("broken", PathBuf::from("vw-missing-mcp-command"))
            .args(vec!["--stdio".to_string()])
            .env(vec![EnvVariable::new("VW_TEST_MCP_ENV", "present")]),
    );

    let error = with_server_peer(&server, |_peer| Box::pin(async move { Ok(()) }))
        .await
        .expect_err("missing stdio command should fail");
    assert!(!error.to_string().is_empty());
}

#[tokio::test]
async fn with_server_peer_remote_transports_surface_connection_errors_after_client_build() {
    let http = McpServer::Http(McpServerHttp::new("offline-http", "http://127.0.0.1:9/mcp"));
    let http_error = with_server_peer_timeout(&http, Some(Duration::from_millis(20)), |_peer| {
        Box::pin(async move { Ok(()) })
    })
    .await
    .expect_err("offline HTTP server should fail after the client is built");
    assert!(
        format!("{http_error:#}").contains("failed to initialize MCP peer 'offline-http'"),
        "{http_error:#}"
    );

    let sse = McpServer::Sse(McpServerSse::new("offline-sse", "http://127.0.0.1:9/sse"));
    let sse_error = with_server_peer_timeout(&sse, Some(Duration::from_millis(20)), |_peer| {
        Box::pin(async move { Ok(()) })
    })
    .await
    .expect_err("offline SSE server should fail after the client is built");
    assert!(
        format!("{sse_error:#}").contains("failed to start SSE MCP transport for 'offline-sse'"),
        "{sse_error:#}"
    );
}

#[test]
fn mcp_management_helpers_apply_defaults_and_timeout() {
    let server = McpServer::Http(McpServerHttp::new("gateway", "https://example.com/mcp"));
    let config = resolved_config_with_servers(vec![server.clone()]);

    let mut management = management_for_server(&config, &server);
    assert_eq!(management.name, "gateway");
    assert!(management.enabled);
    assert!(!management.required);
    assert_eq!(management_timeout(&management), None);

    management.timeout_ms = Some(2_500);
    assert_eq!(management_timeout(&management), Some(Duration::from_millis(2_500)));
}

#[test]
fn mcp_tool_policy_honors_allow_lists_and_deny_precedence() {
    let mut management = vw_acp::McpServerManagement::default_for_server("gateway");
    assert!(mcp_tool_allowed(&management, "echo", "gateway__echo"));

    management.allowed_tools = vec!["echo".to_string(), "gateway__sum".to_string()];
    assert!(mcp_tool_allowed(&management, "echo", "gateway__echo"));
    assert!(mcp_tool_allowed(&management, "sum", "gateway__sum"));
    assert!(!mcp_tool_allowed(&management, "hidden", "gateway__hidden"));

    management.denied_tools = vec!["echo".to_string(), "gateway__sum".to_string()];
    assert!(!mcp_tool_allowed(&management, "echo", "gateway__echo"));
    assert!(!mcp_tool_allowed(&management, "sum", "gateway__sum"));
}

#[test]
fn mcp_auth_method_names_are_sorted_without_exposing_values() {
    let auth = HashMap::from([
        ("oauth".to_string(), "opaque-oauth-value".to_string()),
        ("api_key".to_string(), "opaque-api-value".to_string()),
    ]);

    assert_eq!(auth_method_names(&auth), vec!["api_key", "oauth"]);
    assert!(auth_method_names(&HashMap::new()).is_empty());
}

#[test]
fn mcp_server_transport_reports_each_supported_transport() {
    let http = McpServer::Http(McpServerHttp::new("http", "https://example.com/mcp"));
    let sse = McpServer::Sse(McpServerSse::new("sse", "https://example.com/sse"));
    let stdio = McpServer::Stdio(McpServerStdio::new("stdio", "mcp-command"));

    assert_eq!(server_transport(&http), "http");
    assert_eq!(server_transport(&sse), "sse");
    assert_eq!(server_transport(&stdio), "stdio");
}

#[test]
fn mcp_resource_contents_json_preserves_text_blob_and_empty_inputs() {
    let contents = vec![
        ResourceContents::TextResourceContents {
            uri: "file:///notes.txt".to_string(),
            mime_type: Some("text/plain".to_string()),
            text: "hello".to_string(),
            meta: None,
        },
        ResourceContents::BlobResourceContents {
            uri: "file:///image.bin".to_string(),
            mime_type: None,
            blob: "AAEC".to_string(),
            meta: None,
        },
    ];

    assert_eq!(
        resource_contents_json(&contents),
        json!([
            {
                "uri": "file:///notes.txt",
                "mime_type": "text/plain",
                "text": "hello",
                "kind": "text"
            },
            {
                "uri": "file:///image.bin",
                "mime_type": null,
                "blob": "AAEC",
                "kind": "blob"
            }
        ])
    );
    assert_eq!(resource_contents_json(&[]), json!([]));
}

#[tokio::test]
async fn plugin_mcp_servers_are_visible_in_resolved_acp_config() {
    let _lock = env_lock();
    let dir = tempfile::tempdir().expect("temp dir");
    let home_dir = dir.path().join("home");
    let _home = EnvVarGuard::set("HOME", home_dir.to_str().expect("utf-8 home"));
    let _config_dir_env = EnvVarGuard::unset("VIBEWINDOW_CONFIG_DIR");
    let _workspace_env = EnvVarGuard::unset("VIBEWINDOW_WORKSPACE");

    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    write_json(
        &workspace_dir.join(".vwacprc.json"),
        json!({
            "mcpServers": [
                { "name": "user-mcp", "command": "user-mcp" }
            ]
        }),
    );

    let plugin_root = workspace_dir.join(".vibewindow/plugins/plugin-pack");
    write_plugin_manifest(&plugin_root, "plugin-pack", "./.mcp.json");
    write_json(
        &plugin_root.join(".mcp.json"),
        json!([{ "name": "plugin-mcp", "command": "plugin-mcp" }]),
    );

    let plugin_options =
        PluginDiscoveryOptions::new(workspace_dir.clone(), config_dir, PluginsConfig::default());
    let config = load_resolved_acp_config_from_with_plugin_options(&workspace_dir, &plugin_options)
        .await
        .expect("resolved config");

    println!("resolved plugin mcp order: {:?}", server_names(&config.mcp_servers));
    assert_eq!(server_names(&config.mcp_servers), vec!["user-mcp", "plugin-mcp"]);
}

#[tokio::test]
async fn resolved_acp_config_from_merges_global_servers_and_management_overrides() {
    let dir = tempfile::tempdir().expect("temp dir");
    let config_dir = dir.path().join("config");
    let workspace_dir = dir.path().join("workspace");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&workspace_dir).expect("workspace dir");
    write_json(
        &workspace_dir.join(".vwacprc.json"),
        json!({
            "mcpServers": [
                { "name": "project-mcp", "command": "project-mcp" }
            ]
        }),
    );

    let agent_config = Config {
        config_path: config_dir.join("vibewindow.json"),
        workspace_dir: config_dir.join("workspace"),
        mcp_servers: vec![json!({
            "name": "global-mcp",
            "type": "http",
            "url": "https://example.test/mcp"
        })],
        mcp_server_overrides: vec![json!({
            "name": "global-mcp",
            "enabled": false,
            "required": true,
            "timeout": 3,
            "allowedTools": ["demo_echo"]
        })],
        ..Config::default()
    };

    let mut config = vw_acp::load_resolved_config(&workspace_dir).await.expect("resolved config");
    merge_global_mcp_servers_from_agent_config(&mut config, &agent_config)
        .expect("merge global mcp servers");
    apply_global_mcp_server_overrides_from_agent_config(&mut config, &agent_config)
        .expect("apply global mcp server overrides");

    assert_eq!(server_names(&config.mcp_servers), vec!["project-mcp", "global-mcp"]);
    let management = config
        .mcp_server_management
        .iter()
        .find(|entry| entry.name == "global-mcp")
        .expect("global management override");
    assert!(!management.enabled);
    assert!(management.required);
    assert_eq!(management.timeout_ms, Some(3_000));
    assert_eq!(management.allowed_tools, vec!["demo_echo"]);
}

#[tokio::test]
async fn resolved_acp_config_uses_current_directory_entrypoint() {
    let _lock = env_lock();
    let dir = tempfile::tempdir().expect("temp dir");
    let home_dir = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let workspace_dir = dir.path().join("workspace");
    let _home = EnvVarGuard::set("HOME", home_dir.to_str().expect("utf-8 home"));
    let _config_dir_env =
        EnvVarGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.to_str().expect("utf-8 config"));
    let _workspace_env = EnvVarGuard::unset("VIBEWINDOW_WORKSPACE");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&workspace_dir).expect("workspace dir");
    write_json(
        &workspace_dir.join(".vwacprc.json"),
        json!({
            "mcpServers": [
                { "name": "cwd-mcp", "command": "cwd-mcp" }
            ]
        }),
    );
    let _cwd = CurrentDirGuard::set(&workspace_dir);

    let config = load_resolved_acp_config().await.expect("resolved current dir config");

    assert_eq!(server_names(&config.mcp_servers), vec!["cwd-mcp"]);
}

#[tokio::test]
async fn resolved_acp_config_keeps_project_servers_when_runtime_config_cannot_load() {
    let _lock = env_lock();
    let dir = tempfile::tempdir().expect("temp dir");
    let home_dir = dir.path().join("home");
    let config_file = dir.path().join("config-file");
    let workspace_dir = dir.path().join("workspace");
    let _home = EnvVarGuard::set("HOME", home_dir.to_str().expect("utf-8 home"));
    let _config_dir_env =
        EnvVarGuard::set("VIBEWINDOW_CONFIG_DIR", config_file.to_str().expect("utf-8 config"));
    let _workspace_env = EnvVarGuard::unset("VIBEWINDOW_WORKSPACE");
    fs::write(&config_file, "not a directory").expect("config sentinel");
    write_json(
        &workspace_dir.join(".vwacprc.json"),
        json!({
            "mcpServers": [
                { "name": "project-only", "command": "project-only" }
            ]
        }),
    );

    let config = load_resolved_acp_config_from(&workspace_dir).await.expect("resolved config");

    assert_eq!(server_names(&config.mcp_servers), vec!["project-only"]);
}

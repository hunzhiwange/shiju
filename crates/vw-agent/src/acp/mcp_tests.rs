use agent_client_protocol::{EnvVariable, McpServer, McpServerStdio};
use tempfile::TempDir;
use vw_acp::server::SessionBackend;
use vw_api_types::id::TenantScope;

use super::backend::VibeWindowBackend;
use super::mcp::prepare_session_tools;
use crate::app::agent::tools::{ToolRuntimeContext, tool_specs_for_context};

fn fixture_server(root: &std::path::Path, env: Vec<EnvVariable>) -> McpServer {
    let script = root.join("fixture_mcp.py");
    std::fs::write(
        &script,
        r#"
import json
import sys
tool = {"name":"echo","description":"echo","inputSchema":{"type":"object"}}
for line in sys.stdin:
    message = json.loads(line)
    method = message.get("method")
    request_id = message.get("id")
    if method == "initialize":
        result = {
            "protocolVersion": (message.get("params") or {}).get("protocolVersion", "2025-06-18"),
            "capabilities": {"tools": {"listChanged": False}},
            "serverInfo": {"name": "fixture", "version": "1"},
        }
    elif method == "tools/list":
        result = {"tools": [tool]}
    else:
        result = {}
    if request_id is not None:
        print(json.dumps({"jsonrpc":"2.0","id":request_id,"result":result}), flush=True)
"#,
    )
    .expect("fixture script");
    McpServer::Stdio(
        McpServerStdio::new(
            "fixture",
            std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string()),
        )
        .args(vec![script.to_string_lossy().into_owned()])
        .env(env),
    )
}

#[tokio::test]
async fn mcp_servers_are_scoped_to_bound_session() {
    let root = TempDir::new().expect("temporary root");
    let tools = prepare_session_tools(&[fixture_server(root.path(), Vec::new())])
        .await
        .expect("session tools");
    let with_mcp = ToolRuntimeContext::new(
        "with-mcp",
        Some(root.path().to_string_lossy().into_owned()),
    )
    .with_session_tools(tools);
    let without_mcp = ToolRuntimeContext::new(
        "without-mcp",
        Some(root.path().to_string_lossy().into_owned()),
    );

    assert!(
        tool_specs_for_context(&with_mcp).iter().any(|spec| spec.id == "mcp__fixture__echo")
    );
    assert!(
        !tool_specs_for_context(&without_mcp).iter().any(|spec| spec.id == "mcp__fixture__echo")
    );
}

#[tokio::test]
async fn mcp_conversion_preserves_supported_transport_without_logging_env() {
    let root = TempDir::new().expect("temporary root");
    let secret = "Bearer session-secret";
    let tools = prepare_session_tools(&[fixture_server(
        root.path(),
        vec![EnvVariable::new("AUTHORIZATION", secret)],
    )])
    .await
    .expect("stdio transport");

    assert_eq!(tools.len(), 1);
    assert!(!format!("{tools:?}").contains(secret));
}

#[tokio::test]
async fn unsupported_or_failed_mcp_is_rejected_atomically() {
    let duplicate = vec![
        McpServer::Stdio(McpServerStdio::new("same", "missing-command-one")),
        McpServer::Stdio(McpServerStdio::new("same", "missing-command-two")),
    ];
    let duplicate_error =
        prepare_session_tools(&duplicate).await.expect_err("duplicate names must fail");
    assert!(duplicate_error.to_string().contains("duplicate MCP server name"));
    assert!(!duplicate_error.to_string().contains("missing-command"));

    let failed = [McpServer::Stdio(McpServerStdio::new(
        "broken",
        "vibewindow-missing-mcp-command",
    ))];
    let failure = prepare_session_tools(&failed).await.expect_err("startup must fail");
    assert_eq!(failure.to_string(), "invalid backend request: MCP server 'broken' failed to start");
}

#[tokio::test]
async fn loaded_session_rebuilds_requested_mcp_scope() {
    crate::project::instance::dispose_all().await;
    let root = TempDir::new().expect("temporary root");
    let tenant =
        TenantScope::new("acp-mcp-platform", "acp-mcp-company").expect("valid tenant scope");
    let backend = VibeWindowBackend::new(tenant);
    let created = SessionBackend::create_session_with_mcp(
        &backend,
        root.path().to_path_buf(),
        vec![fixture_server(root.path(), Vec::new())],
    )
    .await
    .expect("create with MCP");
    assert_eq!(backend.session_tool_count(&created.id), Some(1));

    SessionBackend::load_session_with_mcp(
        &backend,
        &created.id,
        root.path().to_path_buf(),
        Vec::new(),
    )
    .await
    .expect("reload without MCP");
    assert_eq!(backend.session_tool_count(&created.id), Some(0));
    crate::project::instance::dispose_all().await;
}

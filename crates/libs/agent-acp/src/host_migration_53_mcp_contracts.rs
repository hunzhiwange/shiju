//! 真实连接端口由宿主薄验收注入，断言归 ACP 所有。
use std::sync::Arc;
use agent_client_protocol::{EnvVariable, McpServer, McpServerStdio};
use plugin_mcp::{connection_host::ConnectionHost, mcp_connections::McpSessionConnections};
use crate::mcp::prepare_session_tools;
pub fn fixture_server(root: &std::path::Path, env: Vec<EnvVariable>) -> McpServer {
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

pub async fn mcp_conversion_preserves_supported_transport_without_logging_env(root: &std::path::Path, host: Arc<dyn ConnectionHost>) {
    let secret = "Bearer session-secret";
    let tools = prepare_session_tools(
        &[fixture_server(root, vec![EnvVariable::new("AUTHORIZATION", secret)])],
        Arc::new(McpSessionConnections::new(host)),
    )
    .await
    .expect("stdio transport");

    assert_eq!(tools.len(), 1);
    assert!(!format!("{tools:?}").contains(secret));
}

pub async fn unsupported_or_failed_mcp_is_rejected_atomically(_root: &std::path::Path, host: Arc<dyn ConnectionHost>) {
    let duplicate = vec![
        McpServer::Stdio(McpServerStdio::new("same", "missing-command-one")),
        McpServer::Stdio(McpServerStdio::new("same", "missing-command-two")),
    ];
    let duplicate_error =
        prepare_session_tools(&duplicate, Arc::new(McpSessionConnections::new(host.clone()))).await.expect_err("duplicate names must fail");
    assert!(duplicate_error.to_string().contains("duplicate MCP server name"));
    assert!(!duplicate_error.to_string().contains("missing-command"));

    let failed =
        [McpServer::Stdio(McpServerStdio::new("broken", "vibewindow-missing-mcp-command"))];
    let failure = prepare_session_tools(&failed, Arc::new(McpSessionConnections::new(host))).await.expect_err("startup must fail");
    assert_eq!(failure.to_string(), "invalid backend request: MCP server 'broken' failed to start");
}

use crate::protocol::{VibeWindowBackend, Host};
use acp_server::server::SessionBackend;
use vw_api_transport::TenantScope;
pub async fn loaded_session_rebuilds_requested_mcp_scope<H: Host + Default>() {
    agent_project::instance::dispose_all().await;
    let root = tempfile::TempDir::new().expect("temporary root");
    let tenant =
        TenantScope::new("acp-mcp-platform", "acp-mcp-company").expect("valid tenant scope");
    let backend = VibeWindowBackend::<H>::new(tenant);
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
    agent_project::instance::dispose_all().await;
}


pub async fn foreign_session_load_rejects_before_starting_mcp_processes<H: Host + Default>() {
    let root = tempfile::TempDir::new().unwrap();
    let owner = VibeWindowBackend::<H>::new(TenantScope::new("acp-load-platform", "owner").unwrap());
    let created = owner.create_session(root.path().into()).await.unwrap();
    let foreign = VibeWindowBackend::<H>::new(TenantScope::new("acp-load-platform", "foreign").unwrap());
    let server = fixture_server(root.path(), Vec::new());
    let script = root.path().join("fixture_mcp.py");
    let contents = std::fs::read_to_string(&script).unwrap();
    std::fs::write(
        &script,
        format!(
            "from pathlib import Path\nPath(__file__).with_suffix('.started').touch()\n{contents}"
        ),
    )
    .unwrap();
    assert!(
        foreign.load_session_with_mcp(&created.id, root.path().into(), vec![server]).await.is_err()
    );
    assert!(
        !script.with_extension("started").exists(),
        "MCP must not start for an unowned session"
    );
}

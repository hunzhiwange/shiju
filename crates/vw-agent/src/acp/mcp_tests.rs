//! 测试 ACP 会话中 MCP 服务的作用域、传输转换与原子失败边界。
//! 临时 MCP 进程用于确认环境变量不泄漏，且部分准备失败不会留下半初始化工具。

use agent_client_protocol::{EnvVariable, McpServer, McpServerStdio};
use tempfile::TempDir;
use vw_acp::server::SessionBackend;
use vw_api_types::id::TenantScope;

use super::backend::VibeWindowBackend;
use super::mcp::prepare_session_tools;
use crate::app::agent::tools::{ToolRuntimeContext, tool_specs_for_context};

// 临时脚本只回显协议消息，不读取测试传入的敏感环境变量。
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
    let with_mcp =
        ToolRuntimeContext::new("with-mcp", Some(root.path().to_string_lossy().into_owned()))
            .with_session_tools(tools);
    let without_mcp =
        ToolRuntimeContext::new("without-mcp", Some(root.path().to_string_lossy().into_owned()));

    assert!(tool_specs_for_context(&with_mcp).iter().any(|spec| spec.id == "mcp__fixture__echo"));
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

    let failed =
        [McpServer::Stdio(McpServerStdio::new("broken", "vibewindow-missing-mcp-command"))];
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

#[tokio::test]
async fn foreign_session_load_rejects_before_starting_mcp_processes() {
    let root = TempDir::new().unwrap();
    let owner = VibeWindowBackend::new(TenantScope::new("acp-load-platform", "owner").unwrap());
    let created = owner.create_session(root.path().into()).await.unwrap();
    let foreign = VibeWindowBackend::new(TenantScope::new("acp-load-platform", "foreign").unwrap());
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

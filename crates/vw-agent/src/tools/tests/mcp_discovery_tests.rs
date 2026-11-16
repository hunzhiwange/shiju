//! Real application MCP configuration, stdio discovery and server-bound connections.

use super::mcp_common::with_server_peer;
use super::mcp_tool::{McpTool, discover_mcp_tool_specs};
use super::{Tool, ToolSpec};
use agent_client_protocol::{HttpHeader, McpServer, McpServerHttp, McpServerStdio};
use serde_json::json;

fn dynamic_spec() -> ToolSpec {
    ToolSpec::new(
        "mcp__docs__search",
        "Search docs.",
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "query": { "type": "string" }
            },
            "required": ["query"]
        }),
    )
    .with_display_name("docs: search")
    .with_read_only(true)
    .with_destructive(false)
    .with_concurrency_safe(true)
    .with_requires_user_interaction(false)
    .with_strict(true)
}

// fixture 服务用于验证真实 stdio 握手；其它结果转换测试使用内存处理器保持聚焦。
fn write_mcp_fixture_server(dir: &std::path::Path) -> std::path::PathBuf {
    let script = dir.join("fixture_mcp_server.py");
    std::fs::write(
        &script,
        r#"
import json
import sys

TOOL = {
    "name": "Echo Tool",
    "description": "Echo a message.",
    "inputSchema": {"type": "object", "properties": {"message": {"type": "string"}}},
    "annotations": {"readOnlyHint": True, "destructiveHint": False, "idempotentHint": True}
}

def send(payload):
    sys.stdout.write(json.dumps(payload, separators=(",", ":")) + "\n")
    sys.stdout.flush()

for line in sys.stdin:
    if not line.strip():
        continue
    message = json.loads(line)
    method = message.get("method")
    request_id = message.get("id")
    if method == "initialize":
        params = message.get("params") or {}
        send({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "protocolVersion": params.get("protocolVersion", "2025-06-18"),
                "capabilities": {"tools": {"listChanged": False}},
                "serverInfo": {"name": "fixture-mcp", "version": "1.0.0"},
                "instructions": "Fixture instructions."
            }
        })
    elif method == "tools/list":
        send({"jsonrpc": "2.0", "id": request_id, "result": {"tools": [TOOL]}})
    elif method == "tools/call":
        params = message.get("params") or {}
        arguments = params.get("arguments") or {}
        send({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "content": [{"type": "text", "text": "echo:" + str(arguments.get("message", ""))}],
                "structuredContent": {"echoed": arguments.get("message", "")},
                "isError": False
            }
        })
    elif request_id is not None:
        send({"jsonrpc": "2.0", "id": request_id, "result": {}})
"#,
    )
    .expect("write MCP fixture server");
    script
}

fn fixture_server(script: &std::path::Path) -> McpServer {
    McpServer::Stdio(
        McpServerStdio::new(
            "Fixture Server",
            std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string()),
        )
        .args(vec![script.to_string_lossy().to_string()]),
    )
}

#[tokio::test]
async fn discover_and_call_configured_mcp_tool_through_stdio_fixture() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let script = write_mcp_fixture_server(temp_dir.path());
    let server = fixture_server(&script);

    let specs = with_server_peer(&server, |peer| {
        Box::pin(async move {
            discover_mcp_tool_specs(peer, "Fixture Server", Some("Fixture instructions."), &[])
                .await
        })
    })
    .await
    .expect("fixture should list tools");
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].id, "mcp__fixture_server__echo_tool_2fb49d0879816c90");

    let tool = McpTool::from_configured_server(server, "Echo Tool", specs[0].clone());
    let result =
        tool.call(json!({ "message": "hello" })).await.expect("fixture tool should execute");

    assert!(result.is_success());
    assert_eq!(
        result.model_result,
        json!({
            "content":[{"type":"text","text":"echo:hello"}],
            "structuredContent":{"echoed":"hello"}
        })
    );
    assert_eq!(result.data["structured_content"], json!({ "echoed": "hello" }));
}

#[tokio::test]
async fn mcp_tool_from_spec_default_handler_reports_missing_configured_server() {
    let tool = McpTool::from_spec("missing-docs", "search", dynamic_spec());

    let error = tool.call(json!({})).await.unwrap_err();

    assert_eq!(error.to_string(), "failed to call MCP tool 'missing-docs::search'");
}

#[tokio::test]
async fn mcp_tool_from_configured_server_uses_server_bound_handler() {
    let server = McpServer::Http(
        McpServerHttp::new("bad-http", "https://example.com/mcp")
            .headers(vec![HttpHeader::new("invalid header", "value")]),
    );
    let tool = McpTool::from_configured_server(server, "search", dynamic_spec());

    let error = tool.call(json!({})).await.unwrap_err();

    assert_eq!(error.to_string(), "failed to call MCP tool 'bad-http::search'");
    assert!(format!("{error:#}").contains("failed to build HTTP MCP client for 'bad-http'"));
}

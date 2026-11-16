//! Real stdio fixture and ACP workspace configuration shared by host acceptance.
use std::path::{Path, PathBuf};
use serde_json::json;
fn write_mcp_toolset_server(tmp: &Path) -> PathBuf {
    let script = tmp.join("fixture_mcp_server.py");
    std::fs::write(
        &script,
        r#"
import json
import sys

TOOL = {
    "name": "Echo Tool",
    "description": "Echo a message from the MCP fixture.",
    "inputSchema": {
        "type": "object",
        "additionalProperties": False,
        "properties": {
            "message": {"type": "string"}
        },
        "required": ["message"]
    },
    "annotations": {
        "readOnlyHint": True,
        "destructiveHint": False,
        "idempotentHint": True
    }
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
                "instructions": "Fixture instructions for dynamic MCP tool tests."
            }
        })
    elif method == "tools/list":
        send({"jsonrpc": "2.0", "id": request_id, "result": {"tools": [TOOL]}})
    elif method == "tools/call":
        params = message.get("params") or {}
        arguments = params.get("arguments") or {}
        text = "echo:" + str(arguments.get("message", ""))
        send({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "content": [{"type": "text", "text": text}],
                "structuredContent": {"echoed": arguments.get("message", "")},
                "isError": False
            }
        })
    elif request_id is not None:
        send({"jsonrpc": "2.0", "id": request_id, "result": {}})
"#,
    )
    .expect("fixture MCP server script should be written");
    script
}

pub fn write_mcp_toolset_workspace(tmp: &Path) -> PathBuf {
    let workspace = tmp.join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace should be created");
    let script = write_mcp_toolset_server(tmp);
    let config = json!({
        "mcpServers": [{
            "name": "Fixture Server",
            "command": std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string()),
            "args": [script.to_string_lossy()]
        }]
    });
    std::fs::write(
        workspace.join(".vwacprc.json"),
        serde_json::to_vec_pretty(&config).expect("MCP config should serialize"),
    )
    .expect("MCP config should be written");
    workspace
}

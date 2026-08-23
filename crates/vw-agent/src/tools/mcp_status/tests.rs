//! 验证 MCP 状态工具的规格、连接诊断、筛选与敏感字段脱敏。

use super::*;
use crate::app::agent::tools::{Tool, ToolRuntimeContext, tool_specs_for_context};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const MCP_STATUS_TOOL_ID: &str = "mcp__fixture_server__echo_tool";

fn write_json(path: &Path, value: Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("json parent should be created");
    }
    fs::write(path, serde_json::to_vec_pretty(&value).expect("json bytes")).expect("write json");
}

fn write_fixture_server(tmp: &TempDir) -> PathBuf {
    let script = tmp.path().join("mcp_status_fixture.py");
    fs::write(
        &script,
        r#"
import json
import sys

TOOL = {
    "name": "Echo Tool",
    "description": "Echo a message from the MCP status fixture.",
    "inputSchema": {
        "type": "object",
        "additionalProperties": False,
        "properties": {"message": {"type": "string"}},
        "required": ["message"]
    },
    "annotations": {"readOnlyHint": True, "destructiveHint": False}
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
        send({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "protocolVersion": "2025-06-18",
                "capabilities": {"tools": {"listChanged": False}},
                "serverInfo": {"name": "fixture-mcp", "version": "1.0.0"}
            }
        })
    elif method == "tools/list":
        send({"jsonrpc": "2.0", "id": request_id, "result": {"tools": [TOOL]}})
    elif request_id is not None:
        send({"jsonrpc": "2.0", "id": request_id, "result": {}})
"#,
    )
    .expect("fixture server should be written");
    script
}

fn workspace_with_config(tmp: &TempDir, config: Value) -> PathBuf {
    let workspace = tmp.path().join("workspace");
    write_json(&workspace.join(".vwacprc.json"), config);
    workspace
}

fn context(workspace: &Path) -> ToolRuntimeContext {
    ToolRuntimeContext::new("mcp-status-test", Some(workspace.to_string_lossy().to_string()))
}

#[test]
fn mcp_status_spec_declares_serial_read_only_alias_contract() {
    let tool = McpStatusTool::new();
    let spec = tool.spec();

    assert_eq!(tool.name(), "McpStatus");
    assert_eq!(spec.display_name, "McpStatus");
    assert!(spec.aliases.iter().any(|alias| alias == "mcp_status"));
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(!spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
    assert_eq!(spec.input_schema["additionalProperties"], json!(false));
}

#[test]
fn mcp_status_mcp_server_management_disabled_servers_are_omitted() {
    let tmp = TempDir::new().expect("temp dir");
    let script = write_fixture_server(&tmp);
    let workspace = workspace_with_config(
        &tmp,
        json!({
            "mcpServers": [{
                "name": "Fixture Server",
                "command": std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string()),
                "args": [script.to_string_lossy()],
                "enabled": false
            }]
        }),
    );

    let specs = tool_specs_for_context(&context(&workspace));

    assert!(specs.iter().any(|spec| spec.id == "McpStatus"));
    assert!(!specs.iter().any(|spec| spec.id == MCP_STATUS_TOOL_ID));
}

#[tokio::test]
async fn mcp_status_redacts_sensitive_fields() {
    let tmp = TempDir::new().expect("temp dir");
    let workspace = workspace_with_config(
        &tmp,
        json!({
            "auth": {
                "github": "ghp_secret_token"
            },
            "mcpServers": [{
                "name": "remote",
                "type": "http",
                "url": "http://127.0.0.1:9/mcp",
                "headers": [{"name": "Authorization", "value": "Bearer secret-token"}],
                "env": [{"name": "SHOULD_NOT_APPEAR", "value": "env-secret"}],
                "enabled": false,
                "required": true,
                "allowedTools": ["safe-tool"],
                "deniedTools": ["danger-tool"]
            }]
        }),
    );

    let output = McpStatusTool::new_for_workspace(workspace.clone())
        .execute(json!({ "server": "remote" }))
        .await
        .expect("status output")
        .output;

    let status: Value = serde_json::from_str(&output).expect("status json");
    assert_eq!(status["servers"][0]["name"], json!("remote"));
    assert_eq!(status["servers"][0]["enabled"], json!(false));
    assert_eq!(status["servers"][0]["auth_methods"], json!(["github"]));
    assert_eq!(status["servers"][0]["allowed_tools"], json!(["safe-tool"]));
    assert_eq!(status["servers"][0]["denied_tools"], json!(["danger-tool"]));

    assert!(!output.contains("ghp_secret_token"));
    assert!(!output.contains("secret-token"));
    assert!(!output.contains("Bearer"));
    assert!(!output.contains("env-secret"));
    assert!(!output.contains("SHOULD_NOT_APPEAR"));
}

#[tokio::test]
async fn mcp_capability_status_reports_unsupported_capabilities() {
    let tmp = TempDir::new().expect("temp dir");
    let workspace = workspace_with_config(
        &tmp,
        json!({
            "mcpServers": []
        }),
    );

    let output = McpStatusTool::new_for_workspace(workspace)
        .execute(json!({}))
        .await
        .expect("status output")
        .output;

    let status: Value = serde_json::from_str(&output).expect("status json");
    let unsupported = status["unsupported_capabilities"]
        .as_array()
        .expect("unsupported capabilities should be listed");
    let unsupported_ids = unsupported
        .iter()
        .map(|entry| entry["capability"].as_str().expect("capability id"))
        .collect::<Vec<_>>();

    assert!(unsupported_ids.contains(&"sampling"));
    assert!(unsupported_ids.contains(&"elicitation"));
    assert!(unsupported_ids.contains(&"roots"));
    assert!(unsupported_ids.contains(&"resources.subscribe"));
    assert!(unsupported_ids.contains(&"resources.listChanged"));
    assert!(unsupported_ids.contains(&"prompts.listChanged"));
    assert!(unsupported_ids.contains(&"tools.listChanged"));
}

#[tokio::test]
async fn mcp_status_required_server_failure_is_reported() {
    let tmp = TempDir::new().expect("temp dir");
    let workspace = workspace_with_config(
        &tmp,
        json!({
            "mcpServers": [{
                "name": "broken",
                "command": "vw-missing-mcp-status-command",
                "required": true,
                "timeout": 0.2
            }]
        }),
    );

    let output = McpStatusTool::new_for_workspace(workspace)
        .execute(json!({ "server": "broken" }))
        .await
        .expect("status output")
        .output;

    let status: Value = serde_json::from_str(&output).expect("status json");
    let server = &status["servers"][0];
    assert_eq!(server["name"], json!("broken"));
    assert_eq!(server["required"], json!(true));
    assert_eq!(server["connection_status"], json!("error"));
    assert!(server["error"].as_str().is_some_and(|error| !error.trim().is_empty()));
}

#[tokio::test]
async fn mcp_status_enabled_server_reports_connected_tool_count() {
    let tmp = TempDir::new().expect("temp dir");
    let script = write_fixture_server(&tmp);
    let workspace = workspace_with_config(
        &tmp,
        json!({
            "mcpServers": [{
                "name": "fixture-server",
                "command": std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string()),
                "args": [script.to_string_lossy()],
                "enabled": true,
                "timeout": 2
            }]
        }),
    );

    let output = McpStatusTool::new_for_workspace(workspace)
        .execute(json!({ "server": "fixture-server" }))
        .await
        .expect("status output")
        .output;

    let status: Value = serde_json::from_str(&output).expect("status json");
    let server = &status["servers"][0];
    assert_eq!(server["name"], json!("fixture-server"));
    assert_eq!(server["connection_status"], json!("connected"));
    assert_eq!(server["tool_count"], json!(1));
}

#[tokio::test]
async fn mcp_status_trims_server_filter_and_reports_management_timeout() {
    let tmp = TempDir::new().expect("temp dir");
    let workspace = workspace_with_config(
        &tmp,
        json!({
            "mcpServers": [
                { "name": "alpha", "command": "unused-alpha", "enabled": false },
                {
                    "name": "beta",
                    "command": "unused-beta",
                    "enabled": false,
                    "timeout": 1.5
                }
            ]
        }),
    );

    let output = McpStatusTool::new_for_workspace(workspace)
        .execute(json!({ "server": "  beta  " }))
        .await
        .expect("filtered status output")
        .output;

    let status: Value = serde_json::from_str(&output).expect("status json");
    assert_eq!(status["servers"].as_array().map(Vec::len), Some(1));
    assert_eq!(status["servers"][0]["name"], "beta");
    assert_eq!(status["servers"][0]["connection_status"], "disabled");
    assert_eq!(status["servers"][0]["tool_count"], Value::Null);
    assert_eq!(status["servers"][0]["timeout_ms"], json!(1_500));
}

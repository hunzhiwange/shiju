use super::ListMcpResourcesTool;
use crate::app::agent::tools::Tool;
use serde_json::json;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner())
}

struct EnvVarGuard {
    key: &'static str,
    original: Option<OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &Path) -> Self {
        let original = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, original }
    }

    fn unset(key: &'static str) -> Self {
        let original = std::env::var_os(key);
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
        let original = std::env::current_dir().expect("current dir should be readable");
        std::env::set_current_dir(path).expect("test cwd should be set");
        Self { original }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
}

#[test]
fn list_mcp_resources_exposes_strict_read_only_contract() {
    let tool = ListMcpResourcesTool::new();

    assert_eq!(tool.name(), "ListMcpResources");
    assert!(tool.description().contains("MCP"));

    let schema = tool.parameters_schema();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["server"]["type"], "string");

    let spec = tool.spec();
    assert_eq!(spec.display_name, "ListMcpResources");
    assert_eq!(spec.aliases, vec!["list_mcp_resources".to_string()]);
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(!spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
}

#[tokio::test]
async fn list_mcp_resources_rejects_non_string_server_before_loading_config() {
    let result = ListMcpResourcesTool::new().execute(json!({ "server": [] })).await;
    let Err(error) = result else {
        panic!("non-string server should be rejected");
    };

    let message = error.to_string();
    assert!(message.contains("invalid type"), "unexpected error: {message}");
}

#[tokio::test]
async fn list_mcp_resources_returns_empty_array_when_config_has_no_servers() {
    let _lock = env_lock();
    let temp = tempfile::tempdir().expect("temp dir");
    let workspace = temp.path().join("workspace");
    let home = temp.path().join("home");
    let config_dir = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace dir");
    let _home = EnvVarGuard::set("HOME", &home);
    let _config = EnvVarGuard::set("VIBEWINDOW_CONFIG_DIR", &config_dir);
    let _workspace = EnvVarGuard::unset("VIBEWINDOW_WORKSPACE");
    let _cwd = CurrentDirGuard::set(&workspace);

    let result = ListMcpResourcesTool::new()
        .execute(json!({ "server": "   " }))
        .await
        .expect("empty config should execute");

    assert!(result.success);
    assert_eq!(result.error, None);
    assert_eq!(serde_json::from_str::<serde_json::Value>(&result.output).unwrap(), json!([]));
}

#[tokio::test]
async fn list_mcp_resources_enters_matching_server_branch_and_propagates_connection_errors() {
    let _lock = env_lock();
    let temp = tempfile::tempdir().expect("temp dir");
    let workspace = temp.path().join("workspace");
    let home = temp.path().join("home");
    let config_dir = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace dir");
    std::fs::write(
        workspace.join(".vwacprc.json"),
        serde_json::to_vec_pretty(&json!({
            "mcpServers": [
                { "name": "broken-resources", "command": "/definitely/missing/vw-agent-mcp" }
            ]
        }))
        .expect("config json"),
    )
    .expect("write project config");
    let _home = EnvVarGuard::set("HOME", &home);
    let _config = EnvVarGuard::set("VIBEWINDOW_CONFIG_DIR", &config_dir);
    let _workspace = EnvVarGuard::unset("VIBEWINDOW_WORKSPACE");
    let _cwd = CurrentDirGuard::set(&workspace);

    let error = ListMcpResourcesTool::new()
        .execute(json!({ "server": " broken-resources " }))
        .await
        .expect_err("matching broken stdio server should surface connection errors");

    let message = error.to_string();
    assert!(
        message.contains("broken-resources") || message.contains("No such file"),
        "unexpected error: {message}"
    );
}

#[tokio::test]
async fn list_mcp_resources_lists_resources_from_matching_stdio_server() {
    let _lock = env_lock();
    let temp = tempfile::tempdir().expect("temp dir");
    let workspace = temp.path().join("workspace");
    let home = temp.path().join("home");
    let config_dir = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace dir");
    let server_script = workspace.join("fake_resources_mcp.py");
    std::fs::write(&server_script, fake_resources_mcp_server()).expect("write fake MCP server");
    std::fs::write(
        workspace.join(".vwacprc.json"),
        serde_json::to_vec_pretty(&json!({
            "mcpServers": [
                {
                    "name": "resource-server",
                    "command": std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string()),
                    "args": [server_script]
                }
            ]
        }))
        .expect("config json"),
    )
    .expect("write project config");
    let _home = EnvVarGuard::set("HOME", &home);
    let _config = EnvVarGuard::set("VIBEWINDOW_CONFIG_DIR", &config_dir);
    let _workspace = EnvVarGuard::unset("VIBEWINDOW_WORKSPACE");
    let _cwd = CurrentDirGuard::set(&workspace);

    let result = ListMcpResourcesTool::new()
        .execute(json!({ "server": "resource-server" }))
        .await
        .expect("fake MCP server should list resources");
    let output: serde_json::Value = serde_json::from_str(&result.output).unwrap();

    assert!(result.success);
    assert_eq!(result.error, None);
    assert_eq!(output[0]["server"]["name"], "resource-server");
    assert_eq!(output[0]["server"]["transport"], "stdio");
    assert_eq!(output[0]["resources"][0]["name"], "notes");
    assert_eq!(output[0]["resources"][0]["uri"], "file:///notes.md");
    assert_eq!(output[0]["resources"][0]["mimeType"], "text/markdown");
}

fn fake_resources_mcp_server() -> &'static str {
    r#"
import json
import sys

for line in sys.stdin:
    message = json.loads(line)
    method = message.get("method")
    if "id" not in message:
        continue
    if method == "initialize":
        result = {
            "protocolVersion": "2025-06-18",
            "capabilities": {"resources": {}},
            "serverInfo": {"name": "fake-resources", "version": "0.1.0"},
        }
    elif method == "resources/list":
        result = {
            "resources": [
                {
                    "uri": "file:///notes.md",
                    "name": "notes",
                    "description": "Workspace notes",
                    "mimeType": "text/markdown",
                }
            ]
        }
    else:
        result = {}
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": message["id"], "result": result}) + "\n")
    sys.stdout.flush()
"#
}

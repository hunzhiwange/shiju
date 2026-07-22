use super::super::traits::Tool;
use super::ReadMcpResourceTool;
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
fn tool_metadata_declares_strict_read_only_surface() {
    let tool = ReadMcpResourceTool::new();
    let spec = tool.spec();

    assert_eq!(tool.name(), "ReadMcpResource");
    assert!(!tool.description().is_empty());
    assert_eq!(tool.parameters_schema()["required"], json!(["server", "uri"]));
    assert!(spec.aliases.iter().any(|alias| alias == "read_mcp_resource"));
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(!spec.concurrency_safe);
    assert!(spec.strict);
}

#[tokio::test]
async fn execute_rejects_blank_server_or_uri_before_loading_external_config() {
    let tool = ReadMcpResourceTool::new();

    for input in [
        json!({ "server": "   ", "uri": "resource://item" }),
        json!({ "server": "local", "uri": "   " }),
    ] {
        let error =
            tool.execute(input).await.expect_err("blank identifiers must be rejected locally");
        assert!(error.to_string().contains("must not be empty"));
    }
}

#[tokio::test]
async fn execute_rejects_non_string_inputs_before_loading_external_config() {
    let tool = ReadMcpResourceTool::new();

    for input in [json!({ "server": [], "uri": "file:///notes.md" }), json!({ "server": "local" })]
    {
        let error = tool.execute(input).await.expect_err("invalid args should be rejected");
        let message = error.to_string();
        assert!(
            message.contains("invalid type") || message.contains("missing field"),
            "unexpected error: {message}"
        );
    }
}

#[tokio::test]
async fn execute_reports_unknown_server_from_isolated_empty_config() {
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

    let error = ReadMcpResourceTool::new()
        .execute(json!({ "server": "missing", "uri": "file:///notes.md" }))
        .await
        .expect_err("empty config should not contain the requested server");

    assert_eq!(error.to_string(), "unknown MCP server 'missing'");
}

#[tokio::test]
async fn execute_enters_matching_server_branch_and_propagates_stdio_spawn_errors() {
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
                { "name": "broken-reader", "command": "/definitely/missing/read-resource-mcp" }
            ]
        }))
        .expect("config json"),
    )
    .expect("write project config");
    let _home = EnvVarGuard::set("HOME", &home);
    let _config = EnvVarGuard::set("VIBEWINDOW_CONFIG_DIR", &config_dir);
    let _workspace = EnvVarGuard::unset("VIBEWINDOW_WORKSPACE");
    let _cwd = CurrentDirGuard::set(&workspace);

    let error = ReadMcpResourceTool::new()
        .execute(json!({ "server": " broken-reader ", "uri": " file:///notes.md " }))
        .await
        .expect_err("matching broken stdio server should surface connection errors");

    let message = error.to_string();
    assert!(
        message.contains("broken-reader") || message.contains("No such file"),
        "unexpected error: {message}"
    );
}

#[tokio::test]
async fn execute_reads_resource_contents_from_matching_stdio_server() {
    let _lock = env_lock();
    let temp = tempfile::tempdir().expect("temp dir");
    let workspace = temp.path().join("workspace");
    let home = temp.path().join("home");
    let config_dir = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace dir");
    let server_script = workspace.join("fake_read_resource_mcp.py");
    std::fs::write(&server_script, fake_read_resource_mcp_server()).expect("write fake server");
    std::fs::write(
        workspace.join(".vwacprc.json"),
        serde_json::to_vec_pretty(&json!({
            "mcpServers": [
                {
                    "name": "resource-reader",
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

    let result = ReadMcpResourceTool::new()
        .execute(json!({ "server": "resource-reader", "uri": " file:///notes.md " }))
        .await
        .expect("fake MCP server should read resource");
    let output: serde_json::Value = serde_json::from_str(&result.output).unwrap();

    assert!(result.success);
    assert_eq!(result.error, None);
    assert_eq!(output["server"]["name"], "resource-reader");
    assert_eq!(output["server"]["transport"], "stdio");
    assert_eq!(output["uri"], " file:///notes.md ");
    assert_eq!(
        output["contents"],
        json!([
            {
                "uri": "file:///notes.md",
                "mime_type": "text/markdown",
                "text": "# Notes",
                "kind": "text"
            }
        ])
    );
}

fn fake_read_resource_mcp_server() -> &'static str {
    r##"
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
            "serverInfo": {"name": "fake-reader", "version": "0.1.0"},
        }
    elif method == "resources/read":
        result = {
            "contents": [
                {
                    "uri": message.get("params", {}).get("uri"),
                    "mimeType": "text/markdown",
                    "text": "# Notes",
                }
            ]
        }
    else:
        result = {}
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": message["id"], "result": result}) + "\n")
    sys.stdout.flush()
"##
}

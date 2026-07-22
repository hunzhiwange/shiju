use super::*;
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
fn mcp_auth_spec_declares_read_only_strict_alias_contract() {
    let tool = McpAuthTool::new();
    let spec = tool.spec();

    assert_eq!(tool.name(), "McpAuth");
    assert_eq!(spec.display_name, "McpAuth");
    assert!(spec.aliases.iter().any(|alias| alias == "mcp_auth"));
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
    assert_eq!(spec.input_schema["additionalProperties"], json!(false));
    assert_eq!(spec.input_schema["properties"]["server"]["type"], "string");
}

#[tokio::test]
async fn mcp_auth_rejects_non_object_input_before_loading_config() {
    let error = McpAuthTool::new()
        .execute(json!({ "server": [] }))
        .await
        .expect_err("array server should not deserialize as MCP auth arguments");

    assert!(error.to_string().contains("invalid type"));
}

#[tokio::test]
async fn mcp_auth_returns_empty_auth_summary_for_empty_config() {
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

    let result = McpAuthTool::new()
        .execute(json!({ "server": "   " }))
        .await
        .expect("empty config should execute");
    let output: serde_json::Value = serde_json::from_str(&result.output).unwrap();

    assert!(result.success);
    assert_eq!(result.error, None);
    assert_eq!(output["auth_policy"], "skip");
    assert_eq!(output["auth_methods"], json!([]));
    assert_eq!(output["servers"], json!([]));
}

#[tokio::test]
async fn mcp_auth_filters_servers_and_never_outputs_secret_auth_values() {
    let _lock = env_lock();
    let temp = tempfile::tempdir().expect("temp dir");
    let workspace = temp.path().join("workspace");
    let home = temp.path().join("home");
    let config_dir = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace dir");
    std::fs::write(
        workspace.join(".vwacprc.json"),
        serde_json::to_vec_pretty(&json!({
            "authPolicy": "fail",
            "auth": {
                "slack": "slack-secret",
                "github": "github-secret"
            },
            "mcpServers": [
                { "name": "local", "command": "local-mcp" },
                { "name": "remote", "type": "http", "url": "https://example.test/mcp" },
                { "name": "events", "type": "sse", "url": "https://example.test/sse" }
            ]
        }))
        .expect("config json"),
    )
    .expect("write project config");
    let _home = EnvVarGuard::set("HOME", &home);
    let _config = EnvVarGuard::set("VIBEWINDOW_CONFIG_DIR", &config_dir);
    let _workspace = EnvVarGuard::unset("VIBEWINDOW_WORKSPACE");
    let _cwd = CurrentDirGuard::set(&workspace);

    let result = McpAuthTool::new()
        .execute(json!({ "server": "remote" }))
        .await
        .expect("configured auth should execute");
    let output: serde_json::Value = serde_json::from_str(&result.output).unwrap();

    assert!(result.success);
    assert_eq!(output["auth_policy"], "fail");
    assert_eq!(output["auth_methods"], json!(["github", "slack"]));
    assert_eq!(output["servers"].as_array().unwrap().len(), 1);
    assert_eq!(output["servers"][0]["name"], "remote");
    assert_eq!(output["servers"][0]["transport"], "http");
    assert!(!result.output.contains("slack-secret"));
    assert!(!result.output.contains("github-secret"));
}

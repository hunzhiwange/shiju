use super::{GetMcpPromptTool, prompt_arguments, prompt_messages_json};
use crate::app::agent::tools::Tool;
use rmcp::model::{PromptMessage, PromptMessageRole};
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

#[tokio::test]
async fn mcp_prompt_resource_tools_get_prompt_rejects_blank_inputs() {
    let tool = GetMcpPromptTool::new();
    let error = tool
        .execute(json!({
            "server": "   ",
            "name": ""
        }))
        .await
        .expect_err("blank inputs should be rejected");

    assert!(error.to_string().contains("must not be empty"));
}

#[test]
fn get_mcp_prompt_exposes_strict_read_only_contract() {
    let tool = GetMcpPromptTool::new();

    assert_eq!(tool.name(), "GetMcpPrompt");
    assert!(tool.description().contains("MCP prompt"));

    let schema = tool.parameters_schema();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["required"], json!(["server", "name"]));
    assert_eq!(schema["properties"]["arguments"]["type"], "object");

    let spec = tool.spec();
    assert_eq!(spec.display_name, "GetMcpPrompt");
    assert_eq!(spec.aliases, vec!["get_mcp_prompt".to_string()]);
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(!spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
}

#[test]
fn prompt_arguments_accepts_absent_null_and_object_values() {
    assert_eq!(prompt_arguments(None).unwrap(), None);
    assert_eq!(prompt_arguments(Some(json!(null))).unwrap(), None);
    assert_eq!(
        prompt_arguments(Some(json!({"topic": "coverage", "count": 0}))).unwrap(),
        json!({"topic": "coverage", "count": 0}).as_object().cloned()
    );
}

#[test]
fn prompt_arguments_rejects_non_object_values() {
    for value in [json!("text"), json!([]), json!(true), json!(1)] {
        let error = prompt_arguments(Some(value)).unwrap_err();
        assert_eq!(error.to_string(), "'arguments' must be a JSON object when provided");
    }
}

#[test]
fn mcp_prompt_resource_tools_get_prompt_maps_messages_to_json() {
    let messages = vec![
        PromptMessage::new_text(PromptMessageRole::User, "Review this"),
        PromptMessage::new_resource(
            PromptMessageRole::Assistant,
            "file:///notes.md".to_string(),
            Some("text/markdown".to_string()),
            Some("# Notes".to_string()),
            None,
            None,
            None,
        ),
    ];

    let value = prompt_messages_json(&messages);

    assert_eq!(
        value,
        json!([
            {
                "role": "user",
                "content": {
                    "type": "text",
                    "text": "Review this"
                }
            },
            {
                "role": "assistant",
                "content": {
                    "type": "resource",
                    "resource": {
                        "uri": "file:///notes.md",
                        "mimeType": "text/markdown",
                        "text": "# Notes"
                    }
                }
            }
        ])
    );
}

#[test]
fn prompt_messages_json_tags_image_and_resource_link_content() {
    let messages: Vec<PromptMessage> = serde_json::from_value(json!([
        {
            "role": "assistant",
            "content": {
                "type": "image",
                "data": "aW1n",
                "mimeType": "image/png"
            }
        },
        {
            "role": "user",
            "content": {
                "type": "resource_link",
                "uri": "file:///notes.md",
                "name": "notes",
                "mimeType": "text/markdown"
            }
        }
    ]))
    .expect("prompt messages should deserialize");

    let value = prompt_messages_json(&messages);

    assert_eq!(value[0]["content"]["type"], "image");
    assert_eq!(value[0]["content"]["data"], "aW1n");
    assert_eq!(value[0]["content"]["mimeType"], "image/png");
    assert_eq!(value[1]["content"]["type"], "resource_link");
    assert_eq!(value[1]["content"]["uri"], "file:///notes.md");
    assert_eq!(value[1]["content"]["name"], "notes");
}

#[test]
fn prompt_messages_json_preserves_empty_message_lists() {
    assert_eq!(prompt_messages_json(&[]), json!([]));
}

#[tokio::test]
async fn get_mcp_prompt_reports_unknown_server_from_resolved_config() {
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

    let error = GetMcpPromptTool::new()
        .execute(json!({ "server": "missing", "name": "review" }))
        .await
        .expect_err("unknown server should be rejected after config load");

    assert_eq!(error.to_string(), "unknown MCP server 'missing'");
}

#[tokio::test]
async fn get_mcp_prompt_fetches_prompt_from_matching_stdio_server_with_arguments() {
    let _lock = env_lock();
    let temp = tempfile::tempdir().expect("temp dir");
    let workspace = temp.path().join("workspace");
    let home = temp.path().join("home");
    let config_dir = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace dir");
    let server_script = workspace.join("fake_get_prompt_mcp.py");
    std::fs::write(&server_script, fake_get_prompt_mcp_server()).expect("write fake MCP server");
    std::fs::write(
        workspace.join(".vwacprc.json"),
        serde_json::to_vec_pretty(&json!({
            "mcpServers": [
                {
                    "name": "prompt-server",
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

    let result = GetMcpPromptTool::new()
        .execute(json!({
            "server": " prompt-server ",
            "name": " review ",
            "arguments": { "topic": "coverage" }
        }))
        .await
        .expect("fake MCP server should return prompt messages");
    let output: serde_json::Value = serde_json::from_str(&result.output).unwrap();

    assert!(result.success);
    assert_eq!(result.error, None);
    assert_eq!(output["server"]["name"], "prompt-server");
    assert_eq!(output["name"], "review");
    assert_eq!(output["description"], "Review coverage");
    assert_eq!(output["messages"][0]["role"], "user");
    assert_eq!(output["messages"][0]["content"]["text"], "Please review coverage");
}

fn fake_get_prompt_mcp_server() -> &'static str {
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
            "capabilities": {"prompts": {}},
            "serverInfo": {"name": "fake-get-prompt", "version": "0.1.0"},
        }
    elif method == "prompts/get":
        params = message.get("params", {})
        topic = params.get("arguments", {}).get("topic", "unknown")
        result = {
            "description": "Review " + topic,
            "messages": [
                {
                    "role": "user",
                    "content": {"type": "text", "text": "Please review " + topic},
                }
            ],
        }
    else:
        result = {}
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": message["id"], "result": result}) + "\n")
    sys.stdout.flush()
"#
}

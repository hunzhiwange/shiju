//! Tool results through a real local JSON-RPC process, not an in-memory call substitute.
use crate::tools::{Tool, ToolCallResult, ToolSpec, mcp_tool::McpTool};
use agent_client_protocol::{McpServer, McpServerHttp, McpServerSse, McpServerStdio};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn payload(source: &str, failed: bool) -> Value {
    let mut result: Value =
        serde_json::from_slice(include_bytes!("../../../testdata/plugin_compat/t21/result.json"))
            .unwrap();
    result["_meta"]["source"] = json!(source);
    result["structuredContent"]["source"] = json!(source);
    result["isError"] = json!(failed);
    result
}

fn server(root: &Path, name: &str) -> McpServer {
    fs::create_dir_all(root).unwrap();
    let script = root.join("server.py");
    fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t21/server.py")).unwrap();
    fs::write(
        root.join("result.json"),
        include_bytes!("../../../testdata/plugin_compat/t21/result.json"),
    )
    .unwrap();
    McpServer::Stdio(
        McpServerStdio::new(name, "python3")
            .args(vec![script.display().to_string(), name.to_owned()]),
    )
}

fn tool_from_server(server: McpServer) -> McpTool {
    let name = crate::tools::mcp_common::server_name(&server).to_owned();
    McpTool::from_configured_server(
        server,
        "report",
        ToolSpec::new(format!("mcp__{name}__report"), "Fixture result", json!({"type":"object"})),
    )
}

fn tool(root: &Path, name: &str) -> McpTool {
    tool_from_server(server(root, name))
}

fn assert_wire_result(result: &ToolCallResult, expected: &Value) {
    assert_eq!(
        &result.data["mcp_result"], expected,
        "the complete protocol result, including all metadata and media, must survive the real call"
    );
    assert_eq!(result.data["content"], expected["content"]);
    let dto = result.to_dto_with_meta(Some("fixture-tool-id"), Some("fixture-call-id"));
    assert_eq!(dto.data["mcp_result"], *expected);
    assert_eq!(
        dto.content,
        expected["content"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| vw_api_types::tools::ToolResultContentDto::Json { value: value.clone() })
            .collect::<Vec<_>>()
    );
    assert_eq!(dto.tool_use_id.as_deref(), Some("fixture-call-id"));
}

#[tokio::test]
async fn codex_plugin_compat_t21_preserves_structured_and_media_content() {
    let root = tempfile::tempdir().unwrap();
    let tool = tool(root.path(), "first");
    let original = fs::read(root.path().join("result.json")).unwrap();
    let result = tool.call(json!({})).await.unwrap();
    assert_wire_result(&result, &payload("first", false));
    assert!(result.is_success());
    assert_eq!(result.model_result["content"].as_array().unwrap().len(), 6);
    assert_eq!(result.model_result["structuredContent"]["answer"], 42);
    assert_eq!(original, fs::read(root.path().join("result.json")).unwrap());
}

#[tokio::test]
async fn codex_plugin_compat_t21_rejects_invalid_input() {
    let root = tempfile::tempdir().unwrap();
    let tool = tool(root.path(), "validation");
    assert!(tool.call(json!(["invalid arguments"])).await.is_err());
    let result = tool.call(json!({"fail":true})).await.unwrap();
    assert!(!result.is_success(), "isError must remain a tool failure, not a transport error");
    assert_wire_result(&result, &payload("validation", true));
    assert_eq!(result.to_dto().success, Some(false));
}

#[tokio::test]
async fn codex_plugin_compat_t21_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let first = tool(&root.path().join("a"), "first");
    let second = tool(&root.path().join("b"), "second");
    let (a, b) = tokio::join!(first.call(json!({})), second.call(json!({"fail":true})));
    let a = a.unwrap();
    let b = b.unwrap();
    assert_eq!(a.data["server"], "first");
    assert_eq!(b.data["server"], "second");
    assert_eq!(a.data["tool"], "report");
    assert_eq!(b.data["tool"], "report");
    assert!(a.is_success());
    assert!(!b.is_success(), "remote metadata must not overwrite host success");
    assert_wire_result(&a, &payload("first", false));
    assert_wire_result(&b, &payload("second", true));
}

#[tokio::test]
async fn codex_plugin_compat_t21_preserves_large_and_empty_results() {
    let root = tempfile::tempdir().unwrap();
    let tool = tool(root.path(), "large");
    let result = tool.call(json!({"mode":"large"})).await.unwrap();
    let mut expected = payload("large", false);
    expected["content"][0]["text"] = json!("L".repeat(600_000));
    expected["content"][1]["data"] = json!("AAAA".repeat(150_000));
    assert_wire_result(&result, &expected);
    assert_eq!(result.model_result["content"][1]["data"], expected["content"][1]["data"]);
    for (mode, expected) in [
        ("empty", json!({"content":[],"_meta":{"source":"large"}})),
        ("structured", json!({"structuredContent":{"answer":42},"_meta":{"source":"large"}})),
    ] {
        let result = tool.call(json!({"mode":mode})).await.unwrap();
        assert_eq!(result.data["mcp_result"], expected);
        assert!(result.is_success());
        assert!(!result.data["mcp_result"].as_object().unwrap().contains_key("isError"));
    }
}

#[tokio::test]
async fn codex_plugin_compat_t21_null_structured_content_does_not_expose_host_metadata() {
    let root = tempfile::tempdir().unwrap();
    let tool = tool(root.path(), "null");
    let result = tool.call(json!({"mode":"structured-null"})).await.unwrap();
    assert_eq!(
        result.data["mcp_result"],
        json!({
            "content":[], "structuredContent":null, "_meta":{"marker":"host-only-null"}
        })
    );
    assert!(
        !result.model_text().contains("host-only-null"),
        "null must not fall back to host data"
    );
    assert!(!result.to_dto().model_result.to_string().contains("host-only-null"));
}

#[tokio::test]
async fn codex_plugin_compat_t21_rejects_malformed_results_without_exposing_payload() {
    let root = tempfile::tempdir().unwrap();
    let tool = tool(root.path(), "invalid");
    for mode in
        ["invalid-block", "invalid-meta", "invalid-error", "invalid-resource", "unknown-content"]
    {
        let error = tool.call(json!({"mode":mode})).await.unwrap_err();
        assert!(format!("{error:#}").contains("MCP"));
        assert!(!format!("{error:#}").contains("host-only-text"));
        assert!(!format!("{error:#}").contains("UklGRg"));
    }
}

#[tokio::test]
async fn codex_plugin_compat_t21_keeps_existing_stdio_notification_compatibility() {
    let root = tempfile::tempdir().unwrap();
    let tool = tool(root.path(), "notification");
    let result = tool
        .call(json!({"mode":"notification"}))
        .await
        .expect("non-standard notifications remain ignored by the stdio compatibility boundary");
    assert_wire_result(&result, &payload("notification", false));
    assert!(
        tool.call(json!({"mode":"invalid-notification"})).await.is_err(),
        "malformed standard protocol messages remain rejected"
    );
}

#[tokio::test]
async fn codex_plugin_compat_t21_preserves_extensions_without_response_or_host_collision() {
    let root = tempfile::tempdir().unwrap();
    let tool = tool(root.path(), "collision");
    let result = tool.call(json!({"mode":"collision"})).await.unwrap();
    let mut expected = payload("collision", false);
    expected["tools"] = json!([]);
    expected["messages"] = json!([]);
    expected["_meta"]["vibewindow/internal-wire-result"] = json!({"content":[],"isError":true});
    assert_wire_result(&result, &expected);
    assert!(result.is_success());
    let result = tool.call(json!({"mode":"link-size"})).await.unwrap();
    assert_eq!(result.data["content"][5]["size"], 1_u64 << 40);
    assert!(!result.model_result.to_string().contains("host-only-text"));
    assert!(!result.model_result.to_string().contains("remote-forgery"));
    for block in result.model_result["content"].as_array().unwrap() {
        assert!(block.get("_meta").is_none(), "protocol metadata stays host-side");
        if let Some(resource) = block.get("resource") {
            assert!(resource.get("_meta").is_none());
        }
    }
}

#[tokio::test]
async fn codex_plugin_compat_t21_distinguishes_call_timeout_from_tool_failure() {
    let root = tempfile::tempdir().unwrap();
    let options = crate::plugins::PluginDiscoveryOptions::new(
        root.path().join("workspace"),
        root.path().join("config"),
        vw_config_types::plugins::PluginsConfig::default(),
    );
    let package = options.workspace_dir.join(".vibewindow/plugins/timeout");
    let _ = server(&package, "timeout");
    fs::create_dir_all(package.join(".codex-plugin")).unwrap();
    fs::write(
        package.join(".codex-plugin/plugin.json"),
        serde_json::to_vec(&json!({
            "name":"timeout", "mcpServers":{"local":{
                "command":"python3", "args":[package.join("server.py"), "timeout"],
                "startup_timeout_sec":2, "tool_timeout_sec":0.1
            }}
        }))
        .unwrap(),
    )
    .unwrap();
    let mut servers = Vec::new();
    crate::plugins::merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert_eq!(servers.len(), 1);
    let tool = tool_from_server(servers.pop().unwrap());
    assert!(!tool.call(json!({"fail":true})).await.unwrap().is_success());
    let error = tool.call(json!({"delay_ms":1000})).await.unwrap_err();
    assert!(format!("{error:#}").contains("timed out"));
}

#[tokio::test]
async fn codex_plugin_compat_t21_preserves_results_over_http_json_and_both_sse_paths() {
    use tokio::{
        io::{AsyncBufReadExt, BufReader},
        process::Command,
    };
    for mode in ["json", "http-sse", "sse"] {
        let root = tempfile::tempdir().unwrap();
        let _ = server(root.path(), mode);
        let script = root.path().join("http_server.py");
        fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t21/http_server.py"))
            .unwrap();
        let mut child = Command::new("python3")
            .arg(script)
            .arg(mode)
            .stdout(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut port = String::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(3),
            BufReader::new(child.stdout.take().unwrap()).read_line(&mut port),
        )
        .await
        .unwrap()
        .unwrap();
        let url = format!(
            "http://127.0.0.1:{}/{}",
            port.trim(),
            if mode == "sse" { "sse" } else { "mcp" }
        );
        let server = if mode == "sse" {
            McpServer::Sse(McpServerSse::new(mode, url))
        } else {
            McpServer::Http(McpServerHttp::new(mode, url))
        };
        let tool = tool_from_server(server);
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), tool.call(json!({})))
            .await
            .unwrap()
            .unwrap();
        assert_wire_result(&result, &payload(mode, false));
        let failed = tool.call(json!({"fail":true})).await.unwrap();
        assert_wire_result(&failed, &payload(mode, true));
        assert!(!failed.is_success());
        drop(tool);
        child.kill().await.unwrap();
    }
}

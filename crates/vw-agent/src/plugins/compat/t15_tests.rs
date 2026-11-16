//! 便携 MCP 声明经真实文件、发现和运行准入入口验证。

use crate::plugins::PluginDiscoveryOptions;
use crate::plugins::mcp::{
    merge_enabled_plugin_mcp_servers,
};
use agent_client_protocol::McpServer;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use config_plugins::PluginsConfig;

const SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json";

fn write(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

fn plugin(workspace: &Path, name: &str) -> PathBuf {
    let root = workspace.join(".vibewindow/plugins").join(name);
    write(
        &root.join("plugin.json"),
        &json!({"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":name}),
    );
    root
}

#[test]
fn codex_plugin_compat_t15_preserves_isolation() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    plugin(&workspace, "absent");
    let root = plugin(&workspace, "active");
    write(
        &root.join("mcp.json"),
        &json!({"$schema":SCHEMA,"mcpServers":{"remote":{"type":"streamable-http","url":"https://example.test/mcp"}}}),
    );
    write(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"name":"ignored-identity","mcpServers":{"remote":{"command":"must-not-execute"},"injected":{"command":"must-not-execute"}}}),
    );
    let disabled = plugin(&workspace, "disabled");
    write(
        &disabled.join("mcp.json"),
        &json!({"$schema":SCHEMA,"mcpServers":{"invalid":{"type":"sse","url":"https://example.test/events"}}}),
    );
    fs::write(workspace.join(".vibewindow/plugins/disabled.disabled"), b"").unwrap();
    let before = fs::read(root.join("mcp.json")).unwrap();
    let options = PluginDiscoveryOptions::new(
        workspace,
        temp.path().join("config"),
        PluginsConfig::default(),
    );
    let mut servers = Vec::new();
    let result = merge_enabled_plugin_mcp_servers(&mut servers, &options);
    assert!(
        result.is_ok(),
        "absent optional portable MCP must not disable healthy siblings: {result:?}"
    );
    assert_eq!(servers.len(), 1);
    let McpServer::Http(server) = &servers[0] else {
        panic!("declared HTTP transport must remain authoritative")
    };
    assert_eq!(server.name, "remote");
    assert_eq!(fs::read(root.join("mcp.json")).unwrap(), before);
    assert!(!root.join("must-not-execute").exists());
}

#[test]
fn codex_plugin_compat_t15_keeps_manifest_format_authoritative() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let root = workspace.join(".vibewindow/plugins/legacy");
    write(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"name":"legacy","mcpServers":"./servers.json"}),
    );
    write(
        &root.join("servers.json"),
        &json!({"$schema":SCHEMA,"mcpServers":{"legacy":{"type":"sse","url":"https://example.test/mcp"}}}),
    );
    let options = PluginDiscoveryOptions::new(
        workspace,
        temp.path().join("config"),
        PluginsConfig::default(),
    );
    let mut servers = Vec::new();
    let result = merge_enabled_plugin_mcp_servers(&mut servers, &options);
    assert!(
        result.is_ok(),
        "legacy package must retain command/url inference even with schema metadata: {result:?}"
    );
    assert!(matches!(&servers[0], McpServer::Http(_)));
}

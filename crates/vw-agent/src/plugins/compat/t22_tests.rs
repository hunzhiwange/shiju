//! Actual runtime and session inspection boundaries for shared resource contracts.

use super::mcp_connections::McpSessionConnections;
use crate::plugins::{PluginDiscoveryOptions, merge_enabled_plugin_mcp_servers};
use crate::tools::mcp_resource::McpResourceTool;
use serde_json::json;
use std::sync::Arc;

use plugin_mcp::resource_test_support::config;
fn fixture(root: &std::path::Path, names: &[&str], mode: &str) -> (McpResourceTool, Arc<McpSessionConnections>, Vec<String>) {
    let (config, pool, names) = plugin_mcp::resource_test_support::fixture(root, names, mode,
        crate::plugins::compat::mcp_connections::host(), |servers, workspace, config| {
            crate::plugins::merge_enabled_plugin_mcp_servers(servers, &PluginDiscoveryOptions::new(workspace, config, Default::default())).unwrap();
        });
    (McpResourceTool::new(config, pool.clone()), pool, names)
}
#[test]
fn codex_plugin_compat_t22_registers_resource_tool_in_actual_runtime() {
    use crate::tools::{ToolRuntimeContext, execute_tool_call};
    let root = tempfile::tempdir().unwrap();
    let (_tool, pool, names) = fixture(root.path(), &["alpha"], "normal");
    pool.shutdown();
    let workspace = root.path().join("workspace");
    let manifest = workspace.join(".vibewindow/plugins/alpha/.codex-plugin/plugin.json");
    let mut declaration: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    declaration["mcpServers"]["shared"]["required"] = json!(true);
    std::fs::write(&manifest, serde_json::to_vec(&declaration).unwrap()).unwrap();
    let ctx =
        ToolRuntimeContext::new("t22-runtime", Some(workspace.to_string_lossy().into_owned()));
    let result=execute_tool_call("McpResource",&json!({"server":names[0],"action":"get_prompt","name":"first","arguments":{"question":"from runtime"}}).to_string(),&ctx).unwrap();
    plugin_mcp::resource_test_support::assert_runtime_prompt(&result, &names[0]);
    crate::plugins::close_session_mcp_connections("t22-runtime", &workspace);
}

#[tokio::test]
async fn codex_plugin_compat_t22_inspection_retains_resource_and_prompt_metadata() {
    let root = tempfile::tempdir().unwrap();
    let (_tool, pool, _) = fixture(root.path(), &["alpha"], "normal");
    pool.shutdown();
    let mut servers = vec![];
    merge_enabled_plugin_mcp_servers(
        &mut servers,
        &PluginDiscoveryOptions::new(
            root.path().join("workspace"),
            root.path().join("config"),
            Default::default(),
        ),
    )
    .unwrap();
    let result = crate::tools::mcp_inspect::inspect_mcp_servers(&config(servers)).await;
    plugin_mcp::resource_test_support::assert_inspection_metadata(
        &result.servers[0].resources, &result.servers[0].prompts,
    );
}

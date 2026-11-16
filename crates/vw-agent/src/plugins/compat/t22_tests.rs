//! Host integration contracts for MCP resource/prompt provenance and admission.
//! Keep the real plugin discovery, ConnectionHost, and subprocess/HTTP fixtures here;
//! local protocol validators belong to plugin-mcp, not a replacement test host.

use super::mcp_connections::McpSessionConnections;
use crate::plugins::{PluginDiscoveryOptions, merge_enabled_plugin_mcp_servers};
use crate::tools::{Tool, mcp_resource::McpResourceTool};
use serde_json::json;
use std::sync::Arc;

fn config(servers: Vec<agent_client_protocol::McpServer>) -> acp_config::ResolvedAcpxConfig {
    acp_config::ResolvedAcpxConfig {
        default_agent: "main".into(),
        default_permissions: acp_types::PermissionMode::ApproveAll,
        non_interactive_permissions: acp_types::NonInteractivePermissionPolicy::Deny,
        auth_policy: acp_types::AuthPolicy::Skip,
        ttl_ms: 300_000,
        timeout_ms: None,
        queue_max_depth: 16,
        format: acp_types::OutputFormat::Text,
        agents: Default::default(),
        auth: Default::default(),
        disable_exec: false,
        mcp_servers: servers,
        mcp_server_management: vec![],
        global_path: "global.json".into(),
        project_path: ".vwacprc.json".into(),
        has_global_config: false,
        has_project_config: false,
    }
}
fn fixture(
    root: &std::path::Path,
    names: &[&str],
    mode: &str,
) -> (McpResourceTool, Arc<McpSessionConnections>, Vec<String>) {
    let workspace = root.join("workspace");
    for name in names {
        let package = workspace.join(".vibewindow/plugins").join(name);
        std::fs::create_dir_all(package.join(".codex-plugin")).unwrap();
        std::fs::copy(
            format!("{}/testdata/plugin_compat/t22/server.py", env!("CARGO_MANIFEST_DIR")),
            package.join("server.py"),
        )
        .unwrap();
        std::fs::write(package.join(".codex-plugin/plugin.json"),serde_json::to_vec(&json!({
            "name":name,"mcpServers":{"shared":{"command":"python3","args":[package.join("server.py"),name,root.join(format!("{name}.jsonl")),mode]}}
        })).unwrap()).unwrap();
    }
    let mut servers = vec![];
    merge_enabled_plugin_mcp_servers(
        &mut servers,
        &PluginDiscoveryOptions::new(workspace, root.join("config"), Default::default()),
    )
    .unwrap();
    let names =
        servers.iter().map(|s| crate::tools::mcp_common::server_name(s).to_owned()).collect();
    let pool = Arc::new(McpSessionConnections::new(crate::plugins::compat::mcp_connections::host()));
    pool.synchronize(&servers).unwrap();
    (McpResourceTool::new(config(servers), pool.clone()), pool, names)
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
    assert_eq!(result.data["server"], names[0]);
    assert_eq!(result.data["mcp_result"]["messages"][0]["content"]["text"], "from runtime");
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
    assert_eq!(
        result.servers[0].resources.len(),
        2,
        "inspection must use the same lossless resource boundary"
    );
    assert_eq!(result.servers[0].resources[0]["_meta"]["source"], "alpha");
    assert_eq!(result.servers[0].prompts[0]["_meta"]["source"], "alpha");
}

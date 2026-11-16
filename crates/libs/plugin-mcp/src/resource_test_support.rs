//! Shared real package/resource fixtures and host-executed provenance assertions.
use crate::mcp_connections::McpSessionConnections;
use std::{path::Path, sync::Arc};
use serde_json::json;
pub fn server_script() -> &'static Path { Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/resources/server.py")) }
pub fn http_script() -> &'static Path { Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/resources/http_server.py")) }
pub fn config(servers: Vec<agent_client_protocol::McpServer>) -> acp_config::ResolvedAcpxConfig {
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
pub fn fixture(
    root: &std::path::Path,
    names: &[&str],
    mode: &str,
    host: Arc<dyn crate::connection_host::ConnectionHost>,
    discover: impl Fn(&mut Vec<agent_client_protocol::McpServer>, std::path::PathBuf, std::path::PathBuf),
) -> (acp_config::ResolvedAcpxConfig, Arc<McpSessionConnections>, Vec<String>) {
    let workspace = root.join("workspace");
    for name in names {
        let package = workspace.join(".vibewindow/plugins").join(name);
        std::fs::create_dir_all(package.join(".codex-plugin")).unwrap();
        std::fs::copy(
            server_script(),
            package.join("server.py"),
        )
        .unwrap();
        std::fs::write(package.join(".codex-plugin/plugin.json"),serde_json::to_vec(&json!({
            "name":name,"mcpServers":{"shared":{"command":"python3","args":[package.join("server.py"),name,root.join(format!("{name}.jsonl")),mode]}}
        })).unwrap()).unwrap();
    }
    let mut servers = vec![];
    discover(&mut servers, workspace, root.join("config"));
    let names =
        servers.iter().map(|s| mcp_client::server_name(s).to_owned()).collect();
    let pool = Arc::new(McpSessionConnections::new(host));
    pool.synchronize(&servers).unwrap();
    (config(servers), pool, names)
}

pub fn assert_runtime_prompt(result: &tool_types::ToolCallResult, name: &str) {
    assert_eq!(result.data["server"], name);
    assert_eq!(result.data["mcp_result"]["messages"][0]["content"]["text"], "from runtime");
}
pub fn assert_inspection_metadata(resources: &[serde_json::Value], prompts: &[serde_json::Value]) {
    assert_eq!(resources.len(), 2, "inspection must use the same lossless resource boundary");
    assert_eq!(resources[0]["_meta"]["source"], "alpha");
    assert_eq!(prompts[0]["_meta"]["source"], "alpha");
}

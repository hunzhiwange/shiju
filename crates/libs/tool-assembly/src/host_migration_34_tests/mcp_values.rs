use mcp_client::*;
use agent_client_protocol::{McpServer, McpServerStdio, McpServerHttp};
use serde_json::json;
use std::{collections::HashMap, path::PathBuf, time::Duration};
fn server_names(servers: &[McpServer]) -> Vec<&str> {
    servers.iter().map(server_name).collect()
}

fn resolved_config_with_servers(servers: Vec<McpServer>) -> acp_config::ResolvedAcpxConfig {
    acp_config::ResolvedAcpxConfig {
        default_agent: "main".to_string(),
        default_permissions: acp_types::PermissionMode::ApproveAll,
        non_interactive_permissions: acp_types::NonInteractivePermissionPolicy::Deny,
        auth_policy: acp_types::AuthPolicy::Skip,
        ttl_ms: 300_000,
        timeout_ms: None,
        queue_max_depth: 16,
        format: acp_types::OutputFormat::Text,
        agents: HashMap::new(),
        auth: HashMap::new(),
        disable_exec: false,
        mcp_servers: servers,
        mcp_server_management: Vec::new(),
        global_path: "global.json".to_string(),
        project_path: ".vwacprc.json".to_string(),
        has_global_config: false,
        has_project_config: false,
    }
}

#[test]
fn global_mcp_servers_append_without_overriding_acp_servers() {
    let mut config = resolved_config_with_servers(vec![McpServer::Stdio(McpServerStdio::new(
        "project-mcp",
        PathBuf::from("project-command"),
    ))]);
    let raw_servers = vec![
        json!({ "name": "project-mcp", "command": "global-ignored" }),
        json!({
            "name": "global-mcp",
            "type": "http",
            "url": "https://example.test/mcp",
            "enabled": false,
            "timeout": 2,
            "allowedTools": ["demo_echo"]
        }),
    ];

    merge_global_mcp_server_values(&mut config, &raw_servers, "vibewindow.json")
        .expect("global MCP should merge");

    assert_eq!(server_names(&config.mcp_servers), vec!["project-mcp", "global-mcp"]);
    let management = config
        .mcp_server_management
        .iter()
        .find(|entry| entry.name == "global-mcp")
        .expect("global MCP management should be preserved");
    assert!(!management.enabled);
    assert_eq!(management.timeout_ms, Some(2_000));
    assert_eq!(management.allowed_tools, vec!["demo_echo"]);
}

#[test]
fn global_mcp_servers_empty_input_preserves_existing_config() {
    let server = McpServer::Stdio(McpServerStdio::new("project-mcp", "project-command"));
    let mut config = resolved_config_with_servers(vec![server.clone()]);

    merge_global_mcp_server_values(&mut config, &[], "vibewindow.json")
        .expect("empty global MCP input should be a no-op");

    assert_eq!(config.mcp_servers, vec![server]);
    assert!(config.mcp_server_management.is_empty());
}

#[test]
fn global_mcp_servers_invalid_entry_reports_source_path() {
    let mut config = resolved_config_with_servers(Vec::new());

    let error = merge_global_mcp_server_values(&mut config, &[json!(null)], "vibewindow.json")
        .expect_err("non-object MCP server entry should be rejected");

    assert_eq!(error.to_string(), "Invalid mcpServers[0] in vibewindow.json: expected object");
    assert!(config.mcp_servers.is_empty());
    assert!(config.mcp_server_management.is_empty());
}

#[test]
fn mcp_server_overrides_update_runtime_management() {
    let mut config = resolved_config_with_servers(vec![McpServer::Stdio(McpServerStdio::new(
        "plugin-mcp",
        PathBuf::from("plugin-command"),
    ))]);
    let raw_overrides = vec![json!({
        "name": "plugin-mcp",
        "enabled": false,
        "required": true,
        "timeout": 4,
        "deniedTools": ["demo_sum"]
    })];

    apply_mcp_server_override_values(&mut config, &raw_overrides, "vibewindow.json")
        .expect("runtime override should apply");

    let management = management_for_server(&config, &config.mcp_servers[0]);
    assert!(!management.enabled);
    assert!(management.required);
    assert_eq!(management.timeout_ms, Some(4_000));
    assert_eq!(management.denied_tools, vec!["demo_sum"]);
}

#[test]
fn mcp_server_overrides_skip_unknown_servers() {
    let mut config = resolved_config_with_servers(vec![McpServer::Stdio(McpServerStdio::new(
        "known-mcp",
        "known-command",
    ))]);

    apply_mcp_server_override_values(
        &mut config,
        &[json!({ "name": "missing-mcp", "enabled": false })],
        "vibewindow.json",
    )
    .expect("unknown server override should be ignored");

    assert!(config.mcp_server_management.is_empty());
}

#[test]
fn mcp_server_overrides_update_existing_entry_and_preserve_omitted_fields() {
    let server = McpServer::Stdio(McpServerStdio::new("plugin-mcp", "plugin-command"));
    let mut config = resolved_config_with_servers(vec![server]);
    let mut existing = acp_config::McpServerManagement::default_for_server("plugin-mcp");
    existing.enabled = false;
    existing.required = true;
    existing.timeout_ms = Some(5_000);
    existing.allowed_tools = vec!["old-allowed".to_string()];
    existing.denied_tools = vec!["old-denied".to_string()];
    config.mcp_server_management.push(existing);

    apply_mcp_server_override_values(
        &mut config,
        &[json!({
            "name": "plugin-mcp",
            "timeout": null,
            "allowedTools": ["new-allowed"]
        })],
        "vibewindow.json",
    )
    .expect("override should update the existing management entry");

    assert_eq!(config.mcp_server_management.len(), 1);
    let management = &config.mcp_server_management[0];
    assert!(!management.enabled);
    assert!(management.required);
    assert_eq!(management.timeout_ms, None);
    assert_eq!(management.allowed_tools, vec!["new-allowed"]);
    assert_eq!(management.denied_tools, vec!["old-denied"]);
}

#[test]
fn mcp_server_overrides_create_management_when_entry_is_missing() {
    let server = McpServer::Stdio(McpServerStdio::new("plugin-mcp", "plugin-command"));
    let mut config = resolved_config_with_servers(vec![server]);

    apply_mcp_server_override_values(
        &mut config,
        &[json!({
            "name": "plugin-mcp",
            "required": true,
            "allowedTools": ["demo_echo"],
            "deniedTools": ["demo_delete"]
        })],
        "vibewindow.json",
    )
    .expect("override should create missing management entry");

    assert_eq!(config.mcp_server_management.len(), 1);
    let management = &config.mcp_server_management[0];
    assert_eq!(management.name, "plugin-mcp");
    assert!(management.enabled);
    assert!(management.required);
    assert_eq!(management.timeout_ms, None);
    assert_eq!(management.allowed_tools, vec!["demo_echo"]);
    assert_eq!(management.denied_tools, vec!["demo_delete"]);
}

#[test]
fn mcp_server_overrides_empty_input_preserves_management() {
    let server = McpServer::Stdio(McpServerStdio::new("plugin-mcp", "plugin-command"));
    let mut config = resolved_config_with_servers(vec![server]);
    let management = acp_config::McpServerManagement::default_for_server("plugin-mcp");
    config.mcp_server_management.push(management.clone());

    apply_mcp_server_override_values(&mut config, &[], "vibewindow.json")
        .expect("empty overrides should be a no-op");

    assert_eq!(config.mcp_server_management, vec![management]);
}

#[test]
fn mcp_management_helpers_apply_defaults_and_timeout() {
    let server = McpServer::Http(McpServerHttp::new("gateway", "https://example.com/mcp"));
    let config = resolved_config_with_servers(vec![server.clone()]);

    let mut management = management_for_server(&config, &server);
    assert_eq!(management.name, "gateway");
    assert!(management.enabled);
    assert!(!management.required);
    assert_eq!(management_timeout(&management), None);

    management.timeout_ms = Some(2_500);
    assert_eq!(management_timeout(&management), Some(Duration::from_millis(2_500)));
}

use super::*;
use agent_client_protocol::{McpServer, McpServerHttp, McpServerSse, McpServerStdio};
use serde_json::json;
use std::collections::HashMap;

fn resolved_config(servers: Vec<McpServer>) -> vw_acp::ResolvedAcpxConfig {
    vw_acp::ResolvedAcpxConfig {
        default_agent: "main".to_string(),
        default_permissions: vw_acp::PermissionMode::ApproveAll,
        non_interactive_permissions: vw_acp::NonInteractivePermissionPolicy::Deny,
        auth_policy: vw_acp::AuthPolicy::Skip,
        ttl_ms: 300_000,
        timeout_ms: None,
        queue_max_depth: 16,
        format: vw_acp::OutputFormat::Text,
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

#[tokio::test]
async fn inspect_empty_config_returns_no_servers() {
    let response = inspect_mcp_servers(&resolved_config(Vec::new())).await;

    assert!(response.servers.is_empty());
}

#[tokio::test]
async fn inspect_disabled_servers_preserves_management_and_transport_metadata() {
    let servers = vec![
        McpServer::Http(McpServerHttp::new("docs", "https://example.test/mcp")),
        McpServer::Sse(McpServerSse::new("events", "https://example.test/sse")),
        McpServer::Stdio(McpServerStdio::new("local", "unused-command")),
    ];
    let mut config = resolved_config(servers);
    config.mcp_server_management = vec![
        vw_acp::McpServerManagement {
            name: "docs".to_string(),
            enabled: false,
            required: true,
            timeout_ms: Some(250),
            allowed_tools: vec!["search".to_string()],
            denied_tools: vec!["delete".to_string()],
        },
        vw_acp::McpServerManagement {
            name: "events".to_string(),
            enabled: false,
            ..vw_acp::McpServerManagement::default_for_server("events")
        },
        vw_acp::McpServerManagement {
            name: "local".to_string(),
            enabled: false,
            ..vw_acp::McpServerManagement::default_for_server("local")
        },
    ];

    let value = serde_json::to_value(inspect_mcp_servers(&config).await).unwrap();

    assert_eq!(value["servers"].as_array().unwrap().len(), 3);
    assert_eq!(value["servers"][0]["name"], "docs");
    assert_eq!(value["servers"][0]["transport"], "http");
    assert_eq!(value["servers"][0]["connection_status"], "disabled");
    assert_eq!(value["servers"][0]["required"], true);
    assert_eq!(value["servers"][0]["timeout_ms"], 250);
    assert_eq!(value["servers"][0]["allowed_tools"], json!(["search"]));
    assert_eq!(value["servers"][0]["denied_tools"], json!(["delete"]));
    assert_eq!(value["servers"][1]["transport"], "sse");
    assert_eq!(value["servers"][2]["transport"], "stdio");
    assert!(value["servers"][0]["tool_count"].is_null());
}

#[tokio::test]
async fn inspect_workspace_rejects_invalid_project_json() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join(".vwacprc.json"), "{ invalid json").unwrap();

    let error = inspect_mcp_servers_from_workspace(temp.path()).await.unwrap_err();

    assert!(error.to_string().contains("Invalid JSON"));
}

#[test]
fn summarize_error_uses_the_outer_context() {
    let error = anyhow::anyhow!("connection refused").context("failed to inspect server");

    assert_eq!(summarize_error(&error), "failed to inspect server");
}

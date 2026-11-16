use serde_json::json;

#[tokio::test]
async fn codex_plugin_compat_t26_host_entry_revalidates_package_identity() {
    use super::t17_tests::{options, plugin};
    use agent_client_protocol::McpServer;
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let root = plugin(
        &options,
        "discovery",
        json!({"url":"https://mcp.example/mcp", "oauth":{"client_id":"public-fixture"}}),
    );
    let original = std::fs::read(root.join(".codex-plugin/plugin.json")).unwrap();
    let mut servers = Vec::new();
    super::merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    let McpServer::Http(server) = &mut servers[0] else { panic!("HTTP fixture") };
    server.url = "https://other.example/mcp".into();
    let error = plugin_mcp::mcp_auth::discover_oauth(&super::Source, server).await.unwrap_err();
    assert!(error.to_string().contains("endpoint or identity changed"));
    assert_eq!(std::fs::read(root.join(".codex-plugin/plugin.json")).unwrap(), original);
}

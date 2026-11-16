#[tokio::test]
async fn codex_plugin_compat_t64_uses_platform_command_and_unicode_paths() {
    agent_plugins::package_acceptance::platform::unicode_paths(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/plugin_compat/t61"),
        |server, output| async move {
            use crate::tools::{Tool, mcp_tool::{McpTool, discover_mcp_tool_specs}};
            let pool = std::sync::Arc::new(super::mcp_connections::McpSessionConnections::new(super::mcp_connections::host()));
            let specs = pool.with_peer(&server, None, |peer| {
                Box::pin(async move { discover_mcp_tool_specs(peer, "writer", None, &[]).await })
            }).await?;
            assert_eq!(specs.len(), 1);
            let tool = McpTool::from_session_server(server, "write", specs.into_iter().next().unwrap(), pool.clone(), None);
            let result = tool.call(serde_json::json!({"output":output})).await;
            pool.shutdown();
            result
        },
        &crate::plugins::merge_enabled_plugin_mcp_servers,
        &super::mcp_connection_sessions::close_plugin,
    ).await;
}

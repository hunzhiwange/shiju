//! Real session/context boundary for the domain notification contract.
#[tokio::test]
async fn codex_plugin_compat_t20_inspection_refreshes_only_notified_resource_and_prompt_catalogs() {
    use crate::tools::{ToolUseContext, scope_tool_use_context};
    let root = tempfile::tempdir().unwrap();
    let config = acp_config::load_resolved_config(root.path()).await.unwrap();
    let context = std::sync::Arc::new(ToolUseContext::new(
        "t20-inspect", Some(root.path().to_string_lossy().into()),
    ));
    let _fixture = plugin_mcp::catalog_test_support::assert_inspection_catalog_refresh(|server| {
        let mut config = config.clone();
        config.mcp_servers = vec![server];
        config.mcp_server_management.clear();
        let context = context.clone();
        async move {
            let mut result = scope_tool_use_context(
                context, crate::tools::mcp_inspect::inspect_mcp_servers(&config),
            ).await;
            let server = result.servers.remove(0);
            (server.resources, server.resource_templates, server.prompts)
        }
    }).await;
    crate::plugins::close_session_mcp_connections("t20-inspect", root.path());
}

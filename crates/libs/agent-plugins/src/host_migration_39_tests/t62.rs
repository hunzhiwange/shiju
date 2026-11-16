use crate::compat_t61_tests::{public_original, digest};
use crate::{PluginDiscoveryOptions, install_plugin_from_marketplace};

use super::wire::{call_wire, merge_enabled_plugin_mcp_servers, remove_installed_plugin, set_plugin_enabled};
#[tokio::test]
async fn codex_plugin_compat_t62_calls_actual_public_readonly_service() {
    use tool_mcp::mcp_tool::discover_mcp_tool_specs;
    use std::sync::Arc;
    let t = tempfile::tempdir().unwrap();
    let source = public_original(t.path(), "context7");
    let before = digest(&source);
    let config = t.path().join("config");
    let plugins = config.join("plugins");
    let installed =
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "public").unwrap();
    let options =
        PluginDiscoveryOptions::new(t.path().join("workspace"), &config, Default::default());
    let mut servers = vec![];
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert_eq!(servers.len(), 1);
    let server = servers.remove(0);
    let pool = Arc::new(plugin_mcp::mcp_connections::McpSessionConnections::new(crate::host_migration_37_tests::connection_host()));
    let specs = pool
        .with_peer(&server, None, |peer| {
            Box::pin(async move { discover_mcp_tool_specs(peer, "context7", None, &[]).await })
        })
        .await
        .unwrap();
    let id = tool_mcp::mcp_tool::mcp_tool_id("context7", "resolve-library-id").unwrap();
    let _spec =
        specs.into_iter().find(|s| s.id == id).expect("public service exposes documented resolver");
    let result = call_wire(&pool, &server, "context7", "resolve-library-id", serde_json::json!({"libraryName":"tokio","query":"Rust Tokio async runtime task spawn API"})).await.unwrap();
    assert!(
        result.model_text().to_lowercase().contains("tokio"),
        "public resolver must return requested library"
    );
    pool.shutdown();
    set_plugin_enabled(&plugins, "context7@public", false).unwrap();
    let mut disabled = vec![];
    merge_enabled_plugin_mcp_servers(&mut disabled, &options).unwrap();
    assert!(disabled.is_empty());
    remove_installed_plugin(&plugins, "context7@public").unwrap();
    assert!(!installed.installed_dir.exists());
    assert_eq!(digest(&source), before);
}

#[tokio::test]
async fn codex_plugin_compat_t62_rejects_invalid_input() {
    use agent_client_protocol::{McpServer, McpServerHttp};
    let pool = plugin_mcp::mcp_connections::McpSessionConnections::new(crate::host_migration_37_tests::connection_host());
    let server = McpServer::Http(McpServerHttp::new("unavailable", "http://127.0.0.1:0/mcp"));
    let result = pool.with_peer(&server, None, |_peer| Box::pin(async { Ok(()) })).await;
    assert!(result.is_err(), "unreachable endpoint must fail visibly");
    pool.shutdown();
}
#[test]
fn codex_plugin_compat_t62_preserves_isolation() {
    let t = tempfile::tempdir().unwrap();
    let source = public_original(t.path(), "context7");
    let before = digest(&source);
    let plugins = t.path().join("plugins");
    let a = install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "alpha").unwrap();
    let b = install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "beta").unwrap();
    set_plugin_enabled(&plugins, "context7@alpha", false).unwrap();
    remove_installed_plugin(&plugins, "context7@alpha").unwrap();
    assert!(!a.installed_dir.exists());
    assert_eq!(digest(&b.installed_dir), before);
    assert_eq!(digest(&source), before);
}

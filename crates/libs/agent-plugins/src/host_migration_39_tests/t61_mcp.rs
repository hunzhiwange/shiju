use crate::compat_t61_tests::{public_original, digest};
use crate::{PluginDiscoveryOptions, install_plugin_from_marketplace};
use std::{fs, path::Path};
use super::wire::{call_wire, merge_enabled_plugin_mcp_servers, remove_installed_plugin, set_plugin_enabled};
#[tokio::test]
async fn codex_plugin_compat_t61_runs_installed_mcp_side_effect() {
    use tool_mcp::mcp_tool::discover_mcp_tool_specs;
    use std::sync::Arc;
    let t = tempfile::tempdir().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vw-agent/testdata/plugin_compat/t61/package");
    let before = digest(&source);
    let config = t.path().join("config");
    let plugins = config.join("plugins");
    let installed =
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "market").unwrap();
    let options =
        PluginDiscoveryOptions::new(t.path().join("workspace"), &config, Default::default());
    let mut servers = vec![];
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert_eq!(servers.len(), 1);
    let server = servers.remove(0);
    let pool = Arc::new(plugin_mcp::mcp_connections::McpSessionConnections::new(crate::host_migration_37_tests::connection_host()));
    let specs = pool
        .with_peer(&server, None, |peer| {
            Box::pin(async move { discover_mcp_tool_specs(peer, "writer", None, &[]).await })
        })
        .await
        .unwrap();
    assert_eq!(specs.len(), 1);
    let output = t.path().join("output.txt");
    let result = call_wire(&pool, &server, "writer", "write", serde_json::json!({"output":output})).await.unwrap();
    assert_eq!(result.model_text(), "written");
    assert_eq!(fs::read_to_string(&output).unwrap(), "original package executed\n");
    pool.shutdown();
    assert_eq!(digest(&source), before);
    assert_eq!(digest(&installed.installed_dir), before);
    set_plugin_enabled(&plugins, "original@market", false).unwrap();
    let mut disabled = vec![];
    merge_enabled_plugin_mcp_servers(&mut disabled, &options).unwrap();
    assert!(disabled.is_empty());
    remove_installed_plugin(&plugins, "original@market").unwrap();
    assert!(output.exists());
}

#[tokio::test]
#[ignore = "live original npm dependency; set T61_EVERYTHING_DEPS to separately installed pinned dependencies"]
async fn codex_plugin_compat_t61_public_stdio_original() {
    use tool_mcp::mcp_tool::discover_mcp_tool_specs;
    use std::sync::Arc;
    let t = tempfile::tempdir().unwrap();
    let source = public_original(t.path(), "pi-agent-plugins-smoke");
    let before = digest(&source);
    let config = t.path().join("config");
    let installed = install_plugin_from_marketplace(
        source.to_str().unwrap(),
        &config.join("plugins"),
        "public",
    )
    .unwrap();
    let options =
        PluginDiscoveryOptions::new(t.path().join("workspace"), &config, Default::default());
    let mut servers = vec![];
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert_eq!(servers.len(), 1);
    let server = servers.remove(0);
    let agent_client_protocol::McpServer::Stdio(ref stdio) = server else {
        panic!("expected stdio")
    };
    let data = plugin_mcp::mcp_environment::runtime_cwd(stdio).unwrap().unwrap();
    let dependencies =
        std::env::var("T61_EVERYTHING_DEPS").expect("provide pinned external npm dependency root");
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(
            Path::new(&dependencies)
                .join("node_modules/@modelcontextprotocol/server-everything/package.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(metadata["version"], "2.1.0");
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        Path::new(&dependencies).join("node_modules"),
        data.join("node_modules"),
    )
    .unwrap();
    let pool = Arc::new(plugin_mcp::mcp_connections::McpSessionConnections::new(crate::host_migration_37_tests::connection_host()));
    let specs = pool
        .with_peer(&server, None, |peer| {
            Box::pin(async move { discover_mcp_tool_specs(peer, "everything", None, &[]).await })
        })
        .await
        .unwrap();
    let _spec = specs
        .into_iter()
        .find(|s| s.id == tool_mcp::mcp_tool::mcp_tool_id("everything", "echo").unwrap())
        .expect("original server exposes echo");
    let result = call_wire(&pool, &server, "everything", "echo", serde_json::json!({"message":"t61-original-stdio"})).await.unwrap();
    assert!(result.model_text().contains("t61-original-stdio"));
    pool.shutdown();
    assert_eq!(digest(&source), before);
    assert_eq!(digest(&installed.installed_dir), before);
}

#[tokio::test]
#[ignore = "real public Context7 network integration; no credential or mock"]
async fn codex_plugin_compat_t61_public_http_original() {
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

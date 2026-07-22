use super::PluginDiscoveryOptions;
use super::mcp::{
    load_plugin_mcp_servers_from_path, merge_enabled_plugin_mcp_servers,
    plugin_mcp_discovery_options,
};
use crate::app::agent::config::Config;
use agent_client_protocol::McpServer;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use vw_config_types::plugins::PluginsConfig;

fn write_plugin_manifest(plugin_root: &Path, name: &str, mcp_path: &str) {
    let manifest_dir = plugin_root.join(".codex-plugin");
    fs::create_dir_all(&manifest_dir).expect("manifest dir");
    fs::write(
        manifest_dir.join("plugin.json"),
        serde_json::to_vec_pretty(&json!({
            "name": name,
            "version": "0.1.0",
            "description": format!("{name} test plugin"),
            "mcpServers": mcp_path,
        }))
        .expect("manifest json"),
    )
    .expect("write manifest");
}

fn write_json(path: &Path, value: serde_json::Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("json parent");
    }
    fs::write(path, serde_json::to_vec_pretty(&value).expect("json bytes")).expect("write json");
}

fn server_name(server: &McpServer) -> &str {
    match server {
        McpServer::Http(server) => &server.name,
        McpServer::Sse(server) => &server.name,
        McpServer::Stdio(server) => &server.name,
        _ => "unknown",
    }
}

fn server_names(servers: &[McpServer]) -> Vec<&str> {
    servers.iter().map(server_name).collect()
}

fn stdio_command(server: &McpServer) -> &str {
    match server {
        McpServer::Stdio(server) => server.command.to_str().expect("utf-8 command"),
        _ => panic!("expected stdio server"),
    }
}

#[test]
fn plugin_mcp_discovery_options_use_config_parent_or_workspace_fallback() {
    let workspace_dir = PathBuf::from("/workspace/project");
    let mut config = Config::default();
    config.workspace_dir = workspace_dir.clone();
    config.config_path = PathBuf::from("/config/vibewindow.json");
    config.plugins.enabled = vec!["selected-plugin".to_string()];
    config.plugins.sources = vec!["extra-plugins".to_string()];

    let options = plugin_mcp_discovery_options(&config);

    assert_eq!(options.workspace_dir, workspace_dir);
    assert_eq!(options.config_dir, PathBuf::from("/config"));
    assert_eq!(options.config.enabled, vec!["selected-plugin"]);
    assert_eq!(options.config.sources, vec!["extra-plugins"]);

    config.config_path = PathBuf::new();
    let fallback_options = plugin_mcp_discovery_options(&config);

    assert_eq!(fallback_options.config_dir, config.workspace_dir);
}

#[test]
fn plugin_mcp_servers_parse_supported_shapes() {
    let dir = tempfile::tempdir().expect("temp dir");
    let object_path = dir.path().join("object.mcp.json");
    write_json(
        &object_path,
        json!({
            "mcpServers": [
                { "name": "object-http", "type": "http", "url": "https://example.com/mcp" },
                { "name": "object-sse", "type": "sse", "url": "https://example.com/events" }
            ]
        }),
    );
    let array_path = dir.path().join("array.mcp.json");
    write_json(
        &array_path,
        json!([
            { "name": "array-stdio", "command": "array-mcp", "args": ["serve"] }
        ]),
    );

    let object_servers =
        load_plugin_mcp_servers_from_path(&object_path).expect("object shape parses");
    let array_servers = load_plugin_mcp_servers_from_path(&array_path).expect("array shape parses");

    println!(
        "plugin mcp parse shapes: object={:?}, array={:?}",
        server_names(&object_servers),
        server_names(&array_servers)
    );
    assert_eq!(server_names(&object_servers), vec!["object-http", "object-sse"]);
    assert_eq!(server_names(&array_servers), vec!["array-stdio"]);
}

#[test]
fn plugin_mcp_config_read_error_includes_source_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing_path = dir.path().join("missing.mcp.json");

    let error = load_plugin_mcp_servers_from_path(&missing_path)
        .expect_err("missing plugin MCP config must fail");

    let message = error.to_string();
    assert!(message.contains("failed to read plugin MCP config"));
    assert!(message.contains(missing_path.to_string_lossy().as_ref()));
}

#[test]
fn plugin_mcp_config_rejects_missing_servers_and_scalar_roots() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing_servers_path = dir.path().join("missing-servers.mcp.json");
    write_json(&missing_servers_path, json!({ "servers": [] }));
    let scalar_path = dir.path().join("scalar.mcp.json");
    write_json(&scalar_path, json!(true));

    let missing_error = load_plugin_mcp_servers_from_path(&missing_servers_path)
        .expect_err("object without mcpServers must fail");
    let scalar_error = load_plugin_mcp_servers_from_path(&scalar_path)
        .expect_err("scalar plugin MCP config must fail");

    assert!(missing_error.to_string().contains("missing mcpServers"));
    assert!(scalar_error.to_string().contains("expected array or object"));
}

#[test]
fn plugin_mcp_config_reports_json_and_server_parse_errors() {
    let dir = tempfile::tempdir().expect("temp dir");
    let invalid_json_path = dir.path().join("invalid-json.mcp.json");
    fs::write(&invalid_json_path, b"{").expect("write invalid JSON");
    let invalid_server_path = dir.path().join("invalid-server.mcp.json");
    write_json(&invalid_server_path, json!([{ "name": "missing-command", "type": "stdio" }]));

    let json_error =
        load_plugin_mcp_servers_from_path(&invalid_json_path).expect_err("invalid JSON must fail");
    let server_error = load_plugin_mcp_servers_from_path(&invalid_server_path)
        .expect_err("invalid MCP server declaration must fail");

    assert!(json_error.to_string().contains("failed to parse plugin MCP config"));
    assert!(server_error.to_string().contains("command"));
    assert!(server_error.to_string().contains(invalid_server_path.to_string_lossy().as_ref()));
}

#[test]
fn disabled_plugin_mcp_servers_are_skipped() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_source = workspace_dir.join(".vibewindow/plugins");

    let active_root = plugin_source.join("active-plugin");
    write_plugin_manifest(&active_root, "active-plugin", "./.mcp.json");
    write_json(
        &active_root.join(".mcp.json"),
        json!([{ "name": "active-mcp", "command": "active-mcp" }]),
    );

    let muted_root = plugin_source.join("muted-plugin");
    write_plugin_manifest(&muted_root, "muted-plugin", "./.mcp.json");
    write_json(
        &muted_root.join(".mcp.json"),
        json!([{ "name": "muted-mcp", "command": "muted-mcp" }]),
    );
    fs::write(plugin_source.join("muted-plugin.disabled"), "").expect("disabled marker");

    let options = PluginDiscoveryOptions::new(workspace_dir, config_dir, PluginsConfig::default());
    let mut servers = Vec::new();
    merge_enabled_plugin_mcp_servers(&mut servers, &options).expect("merge plugin mcp");

    println!("plugin mcp enabled order: {:?}", server_names(&servers));
    assert_eq!(server_names(&servers), vec!["active-mcp"]);
}

#[test]
fn plugin_mcp_servers_merge_after_user_config() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/plugin-pack");
    write_plugin_manifest(&plugin_root, "plugin-pack", "./.mcp.json");
    write_json(
        &plugin_root.join(".mcp.json"),
        json!([
            { "name": "shared", "command": "plugin-shared" },
            { "name": "plugin-only", "command": "plugin-only" }
        ]),
    );
    let mut servers = vw_acp::parse_mcp_servers(
        &json!([
            { "name": "shared", "command": "user-shared" },
            { "name": "user-only", "command": "user-only" }
        ]),
        "test ACP config",
    )
    .expect("user mcp servers");

    let options = PluginDiscoveryOptions::new(workspace_dir, config_dir, PluginsConfig::default());
    merge_enabled_plugin_mcp_servers(&mut servers, &options).expect("merge plugin mcp");

    println!("plugin mcp merged order: {:?}", server_names(&servers));
    assert_eq!(server_names(&servers), vec!["shared", "user-only", "plugin-only"]);
    assert_eq!(stdio_command(&servers[0]), "user-shared");
}

#[test]
fn duplicate_plugin_mcp_server_keeps_first_declaration() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/plugin-pack");
    write_plugin_manifest(&plugin_root, "plugin-pack", "./.mcp.json");
    write_json(
        &plugin_root.join(".mcp.json"),
        json!([
            { "name": "duplicate", "command": "first-command" },
            { "name": "duplicate", "command": "second-command" }
        ]),
    );

    let options = PluginDiscoveryOptions::new(workspace_dir, config_dir, PluginsConfig::default());
    let mut servers = Vec::new();
    merge_enabled_plugin_mcp_servers(&mut servers, &options).expect("merge plugin mcp");

    assert_eq!(server_names(&servers), vec!["duplicate"]);
    assert_eq!(stdio_command(&servers[0]), "first-command");
}

#[test]
fn invalid_plugin_mcp_config_reports_plugin_context() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/broken-plugin");
    write_plugin_manifest(&plugin_root, "broken-plugin", "./.mcp.json");
    fs::write(plugin_root.join(".mcp.json"), b"{").expect("write invalid JSON");

    let options = PluginDiscoveryOptions::new(workspace_dir, config_dir, PluginsConfig::default());
    let mut servers = Vec::new();
    let error = merge_enabled_plugin_mcp_servers(&mut servers, &options)
        .expect_err("invalid plugin MCP config must fail the merge");

    let message = error.to_string();
    assert!(message.contains("failed to load MCP servers for plugin 'broken-plugin'"));
    assert!(message.contains(".mcp.json"));
    assert!(servers.is_empty());
}

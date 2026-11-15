//! MCP 工具共享逻辑测试。
//!
//! 这些测试覆盖 server 摘要、连接错误传播、资源读取输入校验以及 MCP 工具别名，
//! 确保多个 MCP 工具依赖的公共行为保持一致。

use super::*;
use crate::app::agent::config::{Config, schema::ConfigExt};
use crate::plugins::PluginDiscoveryOptions;
use agent_client_protocol::{
    EnvVariable, HttpHeader, McpServer, McpServerHttp, McpServerSse, McpServerStdio,
};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use config_plugins::PluginsConfig;

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner())
}

struct EnvVarGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var(key).ok();
        unsafe { std::env::set_var(key, value) };
        Self { key, original }
    }

    fn unset(key: &'static str) -> Self {
        let original = std::env::var(key).ok();
        unsafe { std::env::remove_var(key) };
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.original {
            unsafe { std::env::set_var(self.key, value) };
        } else {
            unsafe { std::env::remove_var(self.key) };
        }
    }
}

struct CurrentDirGuard {
    original: PathBuf,
}

impl CurrentDirGuard {
    fn set(path: &Path) -> Self {
        let original = env::current_dir().expect("current dir");
        env::set_current_dir(path).expect("set current dir");
        Self { original }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        let _ = env::set_current_dir(&self.original);
    }
}

fn write_json(path: &Path, value: serde_json::Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("json parent");
    }
    fs::write(path, serde_json::to_vec_pretty(&value).expect("json bytes")).expect("write json");
}

fn write_plugin_manifest(plugin_root: &Path, name: &str, mcp_path: &str) {
    let manifest_dir = plugin_root.join(".vibewindow-plugin");
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

#[tokio::test]
async fn with_server_peer_remote_transports_report_header_build_errors_before_connecting() {
    let http = McpServer::Http(
        McpServerHttp::new("bad-http", "https://example.com/mcp")
            .headers(vec![HttpHeader::new("invalid header", "value")]),
    );
    let http_error = with_server_peer_timeout(&http, Some(Duration::from_millis(50)), |_peer| {
        Box::pin(async move { Ok(()) })
    })
    .await
    .expect_err("invalid HTTP headers should fail before connecting");
    let http_message = format!("{http_error:#}");
    assert!(http_message.contains("failed to build HTTP MCP client for 'bad-http'"));
    assert!(http_message.contains("invalid MCP remote header name 'invalid header'"));

    let sse = McpServer::Sse(
        McpServerSse::new("bad-sse", "https://example.com/sse")
            .headers(vec![HttpHeader::new("X-Test", "line-one\nline-two")]),
    );
    let sse_error = with_server_peer_timeout(&sse, Some(Duration::from_millis(50)), |_peer| {
        Box::pin(async move { Ok(()) })
    })
    .await
    .expect_err("invalid SSE headers should fail before connecting");
    let sse_message = format!("{sse_error:#}");
    assert!(sse_message.contains("failed to build SSE MCP client for 'bad-sse'"));
    assert!(sse_message.contains("invalid MCP remote header value for 'X-Test'"));
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

#[tokio::test]
async fn with_server_peer_stdio_surfaces_spawn_errors() {
    let server = McpServer::Stdio(
        McpServerStdio::new("broken", PathBuf::from("vw-missing-mcp-command"))
            .args(vec!["--stdio".to_string()])
            .env(vec![EnvVariable::new("VW_TEST_MCP_ENV", "present")]),
    );

    let error = with_server_peer(&server, |_peer| Box::pin(async move { Ok(()) }))
        .await
        .expect_err("missing stdio command should fail");
    assert!(!error.to_string().is_empty());
}

#[tokio::test]
async fn with_server_peer_remote_transports_surface_connection_errors_after_client_build() {
    let http = McpServer::Http(McpServerHttp::new("offline-http", "http://127.0.0.1:9/mcp"));
    let http_error = with_server_peer_timeout(&http, Some(Duration::from_millis(20)), |_peer| {
        Box::pin(async move { Ok(()) })
    })
    .await
    .expect_err("offline HTTP server should fail after the client is built");
    assert!(
        format!("{http_error:#}").contains("failed to initialize MCP peer 'offline-http'"),
        "{http_error:#}"
    );

    let sse = McpServer::Sse(McpServerSse::new("offline-sse", "http://127.0.0.1:9/sse"));
    let sse_error = with_server_peer_timeout(&sse, Some(Duration::from_millis(20)), |_peer| {
        Box::pin(async move { Ok(()) })
    })
    .await
    .expect_err("offline SSE server should fail after the client is built");
    assert!(
        format!("{sse_error:#}").contains("failed to start SSE MCP transport for 'offline-sse'"),
        "{sse_error:#}"
    );
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

#[tokio::test]
async fn plugin_mcp_servers_are_visible_in_resolved_acp_config() {
    let _lock = env_lock();
    let dir = tempfile::tempdir().expect("temp dir");
    let home_dir = dir.path().join("home");
    let _home = EnvVarGuard::set("HOME", home_dir.to_str().expect("utf-8 home"));
    let _config_dir_env = EnvVarGuard::unset("VIBEWINDOW_CONFIG_DIR");
    let _workspace_env = EnvVarGuard::unset("VIBEWINDOW_WORKSPACE");

    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    write_json(
        &workspace_dir.join(".vwacprc.json"),
        json!({
            "mcpServers": [
                { "name": "user-mcp", "command": "user-mcp" }
            ]
        }),
    );

    let plugin_root = workspace_dir.join(".vibewindow/plugins/plugin-pack");
    write_plugin_manifest(&plugin_root, "plugin-pack", "./.mcp.json");
    write_json(
        &plugin_root.join(".mcp.json"),
        json!([{ "name": "plugin-mcp", "command": "plugin-mcp" }]),
    );

    let plugin_options =
        PluginDiscoveryOptions::new(workspace_dir.clone(), config_dir, PluginsConfig::default());
    let config = load_resolved_acp_config_from_with_plugin_options(&workspace_dir, &plugin_options)
        .await
        .expect("resolved config");

    println!("resolved plugin mcp order: {:?}", server_names(&config.mcp_servers));
    assert_eq!(server_names(&config.mcp_servers), vec!["user-mcp", "plugin-mcp"]);
}

#[tokio::test]
async fn resolved_acp_config_from_merges_global_servers_and_management_overrides() {
    let dir = tempfile::tempdir().expect("temp dir");
    let config_dir = dir.path().join("config");
    let workspace_dir = dir.path().join("workspace");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&workspace_dir).expect("workspace dir");
    write_json(
        &workspace_dir.join(".vwacprc.json"),
        json!({
            "mcpServers": [
                { "name": "project-mcp", "command": "project-mcp" }
            ]
        }),
    );

    let agent_config = Config {
        config_path: config_dir.join("vibewindow.json"),
        workspace_dir: config_dir.join("workspace"),
        mcp_servers: vec![json!({
            "name": "global-mcp",
            "type": "http",
            "url": "https://example.test/mcp"
        })],
        mcp_server_overrides: vec![json!({
            "name": "global-mcp",
            "enabled": false,
            "required": true,
            "timeout": 3,
            "allowedTools": ["demo_echo"]
        })],
        ..Config::default()
    };

    let mut config = acp_config::load_resolved_config(&workspace_dir).await.expect("resolved config");
    merge_global_mcp_servers_from_agent_config(&mut config, &agent_config)
        .expect("merge global mcp servers");
    apply_global_mcp_server_overrides_from_agent_config(&mut config, &agent_config)
        .expect("apply global mcp server overrides");

    assert_eq!(server_names(&config.mcp_servers), vec!["project-mcp", "global-mcp"]);
    let management = config
        .mcp_server_management
        .iter()
        .find(|entry| entry.name == "global-mcp")
        .expect("global management override");
    assert!(!management.enabled);
    assert!(management.required);
    assert_eq!(management.timeout_ms, Some(3_000));
    assert_eq!(management.allowed_tools, vec!["demo_echo"]);
}

#[tokio::test]
async fn resolved_acp_config_uses_current_directory_entrypoint() {
    let _lock = env_lock();
    let dir = tempfile::tempdir().expect("temp dir");
    let home_dir = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let workspace_dir = dir.path().join("workspace");
    let _home = EnvVarGuard::set("HOME", home_dir.to_str().expect("utf-8 home"));
    let _config_dir_env =
        EnvVarGuard::set("VIBEWINDOW_CONFIG_DIR", config_dir.to_str().expect("utf-8 config"));
    let _workspace_env = EnvVarGuard::unset("VIBEWINDOW_WORKSPACE");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&workspace_dir).expect("workspace dir");
    write_json(
        &workspace_dir.join(".vwacprc.json"),
        json!({
            "mcpServers": [
                { "name": "cwd-mcp", "command": "cwd-mcp" }
            ]
        }),
    );
    let _cwd = CurrentDirGuard::set(&workspace_dir);

    let config = load_resolved_acp_config().await.expect("resolved current dir config");

    assert_eq!(server_names(&config.mcp_servers), vec!["cwd-mcp"]);
}

#[tokio::test]
async fn resolved_acp_config_keeps_project_servers_when_runtime_config_cannot_load() {
    let _lock = env_lock();
    let dir = tempfile::tempdir().expect("temp dir");
    let home_dir = dir.path().join("home");
    let config_file = dir.path().join("config-file");
    let workspace_dir = dir.path().join("workspace");
    let _home = EnvVarGuard::set("HOME", home_dir.to_str().expect("utf-8 home"));
    let _config_dir_env =
        EnvVarGuard::set("VIBEWINDOW_CONFIG_DIR", config_file.to_str().expect("utf-8 config"));
    let _workspace_env = EnvVarGuard::unset("VIBEWINDOW_WORKSPACE");
    fs::write(&config_file, "not a directory").expect("config sentinel");
    write_json(
        &workspace_dir.join(".vwacprc.json"),
        json!({
            "mcpServers": [
                { "name": "project-only", "command": "project-only" }
            ]
        }),
    );

    let config = load_resolved_acp_config_from(&workspace_dir).await.expect("resolved config");

    assert_eq!(server_names(&config.mcp_servers), vec!["project-only"]);
}

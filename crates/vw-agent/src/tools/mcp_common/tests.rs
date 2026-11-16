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

fn server_names(servers: &[McpServer]) -> Vec<&str> { servers.iter().map(server_name).collect() }

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
    merge_global_mcp_server_values(
        &mut config,
        &agent_config.mcp_servers,
        &agent_config.config_path.to_string_lossy(),
    )
        .expect("merge global mcp servers");
    apply_mcp_server_override_values(
        &mut config,
        &agent_config.mcp_server_overrides,
        &agent_config.config_path.to_string_lossy(),
    )
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

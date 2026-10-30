//! Public HTTP credentials through plugin discovery and the actual MCP HTTP transport.
use crate::plugins::{PluginDiscoveryOptions, mcp::merge_enabled_plugin_mcp_servers};
use crate::tools::mcp_common::{server_summary, with_server_peer};
use agent_client_protocol::McpServer;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::mpsc,
};
use vw_config_types::plugins::PluginsConfig;

pub(super) fn write(path: &Path, value: Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}
pub(super) fn options(temp: &Path) -> PluginDiscoveryOptions {
    PluginDiscoveryOptions::new(
        temp.join("workspace"),
        temp.join("config"),
        PluginsConfig::default(),
    )
}
pub(super) fn plugin(options: &PluginDiscoveryOptions, name: &str, server: Value) -> PathBuf {
    let root = options.workspace_dir.join(".vibewindow/plugins").join(name);
    write(
        &root.join(".codex-plugin/plugin.json"),
        json!({"name":name,"mcpServers":{"shared":server}}),
    );
    root
}
pub(super) async fn list(server: &McpServer) -> anyhow::Result<()> {
    with_server_peer(server, |peer| {
        Box::pin(async move {
            let result = peer.list_tools(None).await?;
            anyhow::ensure!(result.tools.len() == 1, "fixture must return one actual tool");
            Ok(())
        })
    })
    .await
}
pub(super) async fn endpoint()
-> (String, mpsc::UnboundedReceiver<HashMap<String, String>>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut bytes = Vec::new();
            let mut chunk = [0u8; 4096];
            let Some(end) = (async {
                loop {
                    let n = socket.read(&mut chunk).await.ok()?;
                    if n == 0 {
                        return None;
                    }
                    bytes.extend_from_slice(&chunk[..n]);
                    if let Some(end) = bytes.windows(4).position(|x| x == b"\r\n\r\n") {
                        return Some(end + 4);
                    }
                }
            })
            .await
            else {
                continue;
            };
            let text = String::from_utf8_lossy(&bytes[..end]);
            let method = text.lines().next().unwrap().split_whitespace().next().unwrap().to_owned();
            let headers = text
                .lines()
                .skip(1)
                .filter_map(|line| line.split_once(':'))
                .map(|(k, v)| (k.to_ascii_lowercase(), v.trim().to_owned()))
                .collect::<HashMap<_, _>>();
            let length =
                headers.get("content-length").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
            while bytes.len() < end + length {
                let n = socket.read(&mut chunk).await.unwrap();
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&chunk[..n]);
            }
            let _ = tx.send(headers);
            let request = serde_json::from_slice::<Value>(&bytes[end..]).unwrap_or(Value::Null);
            let (status, body) = if method != "POST" {
                ("405 Method Not Allowed", String::new())
            } else if request.get("id").is_none() {
                ("202 Accepted", String::new())
            } else {
                let result = if request["method"] == "initialize" {
                    json!({"protocolVersion":"2025-03-26","capabilities":{"tools":{}},"serverInfo":{"name":"fixture","version":"1"}})
                } else {
                    json!({"tools":[{"name":"fixture","inputSchema":{"type":"object"}}]})
                };
                ("200 OK", json!({"jsonrpc":"2.0","id":request["id"],"result":result}).to_string())
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });
    (url, rx, task)
}
pub(super) async fn captured(
    rx: &mut mpsc::UnboundedReceiver<HashMap<String, String>>,
) -> HashMap<String, String> {
    tokio::time::timeout(Duration::from_secs(3), rx.recv()).await.unwrap().unwrap()
}

#[tokio::test]
async fn codex_plugin_compat_t17_sends_scoped_authorization_header() {
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let (url, mut rx, task) = endpoint().await;
    let root = plugin(
        &options,
        "local",
        json!({"url":url,"http_headers":{"X-Fixture":"static"},"env_http_headers":{"X-Environment":"PATH"},"bearer_token_env_var":"PATH"}),
    );
    let original = fs::read(root.join(".codex-plugin/plugin.json")).unwrap();
    let mut servers = Vec::new();
    let result = merge_enabled_plugin_mcp_servers(&mut servers, &options);
    assert!(
        result.is_ok(),
        "public HTTP credential fields must reach actual transport: {result:?}"
    );
    list(&servers[0]).await.unwrap();
    let headers = captured(&mut rx).await;
    assert!(
        headers
            .get("authorization")
            .is_some_and(|v| v == &format!("Bearer {}", std::env::var("PATH").unwrap())),
        "bearer must reach selected endpoint"
    );
    assert!(headers.get("x-environment").is_some_and(|v| v == &std::env::var("PATH").unwrap()));
    assert_eq!(headers.get("x-fixture").map(String::as_str), Some("static"));
    assert!(
        !serde_json::to_string(&servers).unwrap().contains(&std::env::var("PATH").unwrap()),
        "runtime config must not serialize environment credential values"
    );
    assert!(!server_summary(&servers[0]).to_string().contains("authorization"));
    assert_eq!(original, fs::read(root.join(".codex-plugin/plugin.json")).unwrap());
    task.abort();
}

#[test]
fn codex_plugin_compat_t17_rejects_invalid_input() {
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    plugin(
        &options,
        "local",
        json!({"url":"https://example.test/mcp","env_http_headers":{"X-Fixture":"VIBEWINDOW_T17_DEFINITELY_MISSING_67CFA"}}),
    );
    let mut servers = Vec::new();
    let error = merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap_err();
    assert!(
        format!("{error:#}").contains("environment variable is unavailable"),
        "unknown credential reference must be diagnosed without values: {error:#}"
    );
    assert!(servers.is_empty());
}

#[tokio::test]
async fn codex_plugin_compat_t17_preserves_isolation() {
    let temp = tempfile::tempdir().unwrap();
    let a = options(&temp.path().join("a"));
    let b = options(&temp.path().join("b"));
    let (url_a, mut rx_a, task_a) = endpoint().await;
    let (url_b, mut rx_b, task_b) = endpoint().await;
    plugin(&a, "local", json!({"url":url_a,"http_headers":{"X-Fixture":"first-origin"}}));
    plugin(&b, "local", json!({"url":url_b,"http_headers":{"X-Fixture":"second-origin"}}));
    let mut servers_a = Vec::new();
    let mut servers_b = Vec::new();
    let result = merge_enabled_plugin_mcp_servers(&mut servers_a, &a);
    assert!(result.is_ok(), "source-bound credentials must be admitted: {result:?}");
    merge_enabled_plugin_mcp_servers(&mut servers_b, &b).unwrap();
    list(&servers_a[0]).await.unwrap();
    list(&servers_b[0]).await.unwrap();
    assert_eq!(
        captured(&mut rx_a).await.get("x-fixture").map(String::as_str),
        Some("first-origin")
    );
    assert_eq!(
        captured(&mut rx_b).await.get("x-fixture").map(String::as_str),
        Some("second-origin")
    );
    let mut changed = servers_a[0].clone();
    if let McpServer::Http(server) = &mut changed {
        server.url = url_b;
    }
    assert!(list(&changed).await.is_err(), "changing endpoint cannot reuse credentials");
    task_a.abort();
    task_b.abort();
}

#[tokio::test]
async fn codex_plugin_compat_t17_applies_header_precedence_without_serializing_values() {
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let (url, mut rx, task) = endpoint().await;
    plugin(
        &options,
        "local",
        json!({"url":url,"http_headers":{"X-Fixture":"static","Authorization":"fixture-static"},"env_http_headers":{"x-fixture":"PATH"},"bearer_token_env_var":"PATH"}),
    );
    let mut servers = Vec::new();
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    list(&servers[0]).await.unwrap();
    let headers = captured(&mut rx).await;
    let environment = std::env::var("PATH").unwrap();
    assert!(headers.get("x-fixture").is_some_and(|v| v == &environment));
    assert!(headers.get("authorization").is_some_and(|v| v == &format!("Bearer {environment}")));
    let McpServer::Http(server) = &servers[0] else { panic!("HTTP expected") };
    let scoped = crate::plugins::plugin_mcp_http_headers(server).unwrap().unwrap();
    assert!(
        !format!("{scoped:?}").contains(&environment),
        "HTTP library diagnostics must redact values"
    );
    assert!(!serde_json::to_string(&servers).unwrap().contains("fixture-static"));
    task.abort();
}

#[test]
fn codex_plugin_compat_t17_rejects_header_injection_and_host_owned_fields() {
    for config in [
        json!({"http_headers":{"X-Fixture":"fixture-secret\r\nInjected: value"}}),
        json!({"http_headers":{"invalid name":"fixture-secret"}}),
        json!({"http_headers":{"X-Fixture":"a","x-fixture":"b"}}),
        json!({"http_headers":{"Host":"fixture-secret"}}),
        json!({"http_headers":{"Mcp-Session-Id":"fixture-secret"}}),
        json!({"bearer_token":"fixture-secret"}),
        json!({"bearer_token_env_var":""}),
        json!({"env_http_headers":{"X-Fixture":false}}),
        json!({"environment_id":"remote","http_headers":{"X-Fixture":"fixture-secret"}}),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let options = options(temp.path());
        let mut config = config;
        config["url"] = json!("https://example.test/mcp");
        plugin(&options, "local", config);
        let mut servers = Vec::new();
        let error = merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap_err();
        assert!(
            !format!("{error:#}").contains("fixture-secret"),
            "diagnostic must not contain a value"
        );
        assert!(servers.is_empty());
    }
}

#[tokio::test]
async fn codex_plugin_compat_t17_keeps_public_oauth_configuration_without_starting_login() {
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let (url, mut rx, task) = endpoint().await;
    let root = plugin(
        &options,
        "local",
        json!({"url":url,"auth":"oauth","scopes":[],"oauth_resource":"https://example.test/resource","oauth":{"clientId":"public-client","clientSecret":"fixture-only-secret","callbackUrl":"http://127.0.0.1/callback","callbackPort":4321}}),
    );
    let manifest = crate::plugins::load_plugin_manifest(&root).unwrap();
    let (outcome, _) = crate::plugins::mcp::load_plugin_mcp_declarations(&manifest).unwrap();
    let super::mcp_legacy::PluginMcpDeclaration::Legacy { config, .. } = &outcome.servers[0] else {
        panic!("legacy expected")
    };
    assert_eq!(config["scopes"], json!([]), "explicit empty scopes must remain empty");
    assert_eq!(config["oauth"]["client_id"], "public-client");
    assert_eq!(config["oauth"]["callback_port"], 4321);
    let mut servers = Vec::new();
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert!(!serde_json::to_string(&servers).unwrap().contains("fixture-only-secret"));
    let error = list(&servers[0]).await.unwrap_err();
    assert!(format!("{error:#}").contains("separate login"));
    assert!(!format!("{error:#}").contains("fixture-only-secret"));
    assert!(rx.try_recv().is_err(), "unavailable login must fail before sending HTTP");
    task.abort();
}

#[test]
fn codex_plugin_compat_t17_rejects_invalid_oauth_and_private_authentication() {
    for fields in [
        json!({"auth":"ema_auth"}),
        json!({"auth":"chatgpt"}),
        json!({"oauth":{"client_secret":"fixture-only-secret"}}),
        json!({"oauth":{"client_id":"public-client","client_secret":" "}}),
        json!({"oauth":{"callback_port":65536}}),
        json!({"oauth":{"token_endpoint_auth_method":"client_secret_basic"}}),
        json!({"oauth":{"ema_registration":{}}}),
        json!({"oauth":{"authorization_server_issuer":"https://example.test"}}),
        json!({"scopes":false}),
        json!({"oauth_resource":false}),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let options = options(temp.path());
        let mut config = fields;
        config["url"] = json!("https://example.test/mcp");
        plugin(&options, "local", config);
        let mut servers = Vec::new();
        let error = merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap_err();
        assert!(!format!("{error:#}").contains("fixture-only-secret"));
        assert!(servers.is_empty());
    }
}

#[tokio::test]
async fn codex_plugin_compat_t17_reaudits_source_and_rejects_forged_context() {
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let (url, mut rx, task) = endpoint().await;
    let root = plugin(&options, "local", json!({"url":url,"http_headers":{"X-Fixture":"before"}}));
    let mut servers = Vec::new();
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(root.join(".codex-plugin/plugin.json")).unwrap()).unwrap();
    manifest["mcpServers"]["shared"]["http_headers"]["X-Fixture"] = json!("after");
    write(&root.join(".codex-plugin/plugin.json"), manifest);
    assert!(list(&servers[0]).await.is_err(), "stale source must fail before sending credentials");
    assert!(rx.try_recv().is_err());
    write(
        &root.join(".codex-plugin/plugin.json"),
        json!({"name":"local","mcpServers":"./servers.json"}),
    );
    write(
        &root.join("servers.json"),
        json!([{"type":"http","name":"shared","url":url,"headers":[],"_meta":{"shiju/plugin-http-context":{}}}]),
    );
    let mut servers = Vec::new();
    let error = merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap_err();
    assert!(format!("{error:#}").contains("cannot supply host HTTP context"));
    write(
        &root.join("servers.json"),
        json!([{"type":"http","name":"shared","url":url,"headers":[{"name":"X-Fixture","value":"array"}]}]),
    );
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    list(&servers[0]).await.unwrap();
    assert_eq!(captured(&mut rx).await.get("x-fixture").map(String::as_str), Some("array"));
    task.abort();
}

#[tokio::test]
async fn codex_plugin_compat_t17_filters_portable_client_owned_headers() {
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let (url, mut rx, task) = endpoint().await;
    let root = options.workspace_dir.join(".vibewindow/plugins/portable");
    write(
        &root.join("plugin.json"),
        json!({"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"portable"}),
    );
    write(
        &root.join("mcp.json"),
        json!({"$schema":super::mcp_portable::SCHEMA,"mcpServers":{"shared":{"type":"streamable-http","url":url,"headers":{"Authorization":"package-must-not-own","Host":"ignored.invalid","X-Fixture":"portable"}}}}),
    );
    let mut servers = Vec::new();
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    list(&servers[0]).await.unwrap();
    let headers = captured(&mut rx).await;
    assert!(!headers.contains_key("authorization"));
    assert_ne!(headers.get("host").map(String::as_str), Some("ignored.invalid"));
    assert_eq!(headers.get("x-fixture").map(String::as_str), Some("portable"));
    task.abort();
}

#[tokio::test]
async fn codex_plugin_compat_t17_does_not_forward_credentials_on_redirect() {
    let temp = tempfile::tempdir().unwrap();
    let options = options(temp.path());
    let (destination, mut rx, destination_task) = endpoint().await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let redirect_task = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut data = [0; 8192];
            let _ = socket.read(&mut data).await;
            let response = format!(
                "HTTP/1.1 307 Temporary Redirect\r\nLocation: {destination}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });
    plugin(
        &options,
        "local",
        json!({"url":url,"http_headers":{"X-Fixture":"fixture-only-secret"}}),
    );
    let mut servers = Vec::new();
    merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    assert!(list(&servers[0]).await.is_err());
    assert!(
        tokio::time::timeout(Duration::from_millis(100), rx.recv()).await.is_err(),
        "redirect target must receive no credential-bearing request"
    );
    redirect_task.abort();
    destination_task.abort();
}

#[test]
fn codex_plugin_compat_t17_rejects_insecure_credential_endpoints() {
    for url in [
        "http://example.test/mcp",
        "https://user:fixture-only-secret@example.test/mcp",
        "https://example.test/mcp#fragment",
        "file:///tmp/mcp",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let options = options(temp.path());
        plugin(
            &options,
            "local",
            json!({"url":url,"http_headers":{"X-Fixture":"fixture-only-secret"}}),
        );
        let mut servers = Vec::new();
        let error = merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap_err();
        assert!(!format!("{error:#}").contains("fixture-only-secret"));
    }
}

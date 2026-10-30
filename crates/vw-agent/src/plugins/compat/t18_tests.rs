//! Real stdio process ownership and lifecycle acceptance.
use super::mcp_connections::McpSessionConnections;
use crate::plugins::{PluginDiscoveryOptions, merge_enabled_plugin_mcp_servers};
use agent_client_protocol::{McpServer, McpServerStdio};
use serde_json::json;
use std::{fs, path::Path, time::Duration};
use vw_config_types::plugins::PluginsConfig;

fn server(root: &Path, name: &str) -> McpServer {
    let script = root.join("server.py");
    fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t18/server.py")).unwrap();
    McpServer::Stdio(
        McpServerStdio::new(name, "python3")
            .args(vec![script.display().to_string(), root.join("starts").display().to_string()]),
    )
}

async fn pid(pool: &McpSessionConnections, server: &McpServer) -> anyhow::Result<String> {
    pool.with_peer(server, None, |peer| {
        Box::pin(async move {
            let listed = peer.list_all_tools().await?;
            Ok(listed[0].description.as_deref().unwrap().to_owned())
        })
    })
    .await
}

#[tokio::test]
async fn codex_plugin_compat_t18_reuses_connection_until_session_ends() {
    let root = tempfile::tempdir().unwrap();
    let server = server(root.path(), "local");
    let pool = McpSessionConnections::default();
    let first = pid(&pool, &server).await.unwrap();
    let second = pid(&pool, &server).await.unwrap();
    assert_eq!(first, second, "same session must reuse the actual server process");
    assert_eq!(fs::read_to_string(root.path().join("starts")).unwrap().lines().count(), 1);
    pool.shutdown();
}

#[test]
fn codex_plugin_compat_t18_rejects_invalid_input() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("workspace/.vibewindow/plugins/local");
    fs::create_dir_all(package.join(".codex-plugin")).unwrap();
    fs::write(package.join(".codex-plugin/plugin.json"), serde_json::to_vec(&json!({"name":"local","mcpServers":{"local":{"command":"must-not-run","startup_timeout_sec":0}}})).unwrap()).unwrap();
    let options = PluginDiscoveryOptions::new(
        root.path().join("workspace"),
        root.path().join("config"),
        PluginsConfig::default(),
    );
    let error = merge_enabled_plugin_mcp_servers(&mut Vec::new(), &options).unwrap_err();
    assert!(
        format!("{error:#}").contains("positive startup_timeout_sec"),
        "invalid timeout must be rejected with its lifecycle validation: {error:#}"
    );
}

#[tokio::test]
async fn codex_plugin_compat_t18_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let server = server(root.path(), "local");
    let first = McpSessionConnections::default();
    let second = McpSessionConnections::default();
    let a = pid(&first, &server).await.unwrap();
    let b = pid(&second, &server).await.unwrap();
    assert_ne!(a, b, "legacy connections stay private to each session");
    first.shutdown();
    assert!(
        pid(&first, &server).await.is_err(),
        "closed session must not silently restart a service"
    );
    assert_eq!(
        pid(&second, &server).await.unwrap(),
        b,
        "ending another session must not close this connection"
    );
    second.shutdown();
    tokio::time::sleep(Duration::from_millis(20)).await;
}

fn settings(server: &mut McpServer, startup: u64, call: u64) {
    super::mcp_connection_settings::bind(
        server,
        super::mcp_connection_settings::ConnectionSettings {
            startup_timeout_ms: Some(startup),
            tool_timeout_ms: Some(call),
            ..Default::default()
        },
    )
    .unwrap();
}

#[cfg(unix)]
async fn exited(pid: &str) {
    let id: i32 = pid.parse().unwrap();
    for _ in 0..250 {
        if unsafe { libc::kill(id, 0) } == -1 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("owned MCP process must be killed and reaped");
}

#[tokio::test]
async fn codex_plugin_compat_t18_serializes_startup_and_survives_bridge_runtime_drop() {
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let server = server(root.path(), "local");
    let pool = Arc::new(McpSessionConnections::default());
    let other = pool.clone();
    let configured = server.clone();
    let first = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(pid(&other, &configured))
            .unwrap()
    })
    .join()
    .unwrap();
    let (a, b) = tokio::join!(pid(&pool, &server), pid(&pool, &server));
    assert_eq!(a.unwrap(), first);
    assert_eq!(b.unwrap(), first);
    assert_eq!(fs::read_to_string(root.path().join("starts")).unwrap().lines().count(), 1);
    pool.shutdown();
    #[cfg(unix)]
    exited(&first).await;
}

#[tokio::test]
async fn codex_plugin_compat_t18_times_out_initialization_and_catalog_without_orphans() {
    for mode in ["startup-hang", "catalog-hang"] {
        let root = tempfile::tempdir().unwrap();
        let mut server = server(root.path(), "local");
        let McpServer::Stdio(stdio) = &mut server else { unreachable!() };
        stdio.args.push(mode.into());
        settings(&mut server, 200, 1000);
        let pool = McpSessionConnections::default();
        let error = pid(&pool, &server).await.unwrap_err();
        assert!(format!("{error:#}").contains("timed out"));
        let started = fs::read_to_string(root.path().join("starts")).unwrap();
        #[cfg(unix)]
        exited(started.trim()).await;
        pool.shutdown();
    }
}

#[tokio::test]
async fn codex_plugin_compat_t18_closes_timed_out_and_cancelled_calls() {
    use rmcp::model::CallToolRequestParam;
    use std::borrow::Cow;
    for dropped in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut server = server(root.path(), "local");
        settings(&mut server, 2000, 150);
        let pool = McpSessionConnections::default();
        let first = pid(&pool, &server).await.unwrap();
        let call = pool.with_peer(&server, None, |peer| {
            Box::pin(async move {
                peer.call_tool(CallToolRequestParam {
                    name: Cow::Borrowed("hang"),
                    arguments: None,
                })
                .await?;
                Ok(())
            })
        });
        if dropped {
            assert!(tokio::time::timeout(Duration::from_millis(100), call).await.is_err());
        } else {
            assert!(format!("{:#}", call.await.unwrap_err()).contains("timed out"));
        }
        #[cfg(unix)]
        exited(&first).await;
        assert_ne!(pid(&pool, &server).await.unwrap(), first);
        pool.shutdown();
    }
}

#[tokio::test]
async fn codex_plugin_compat_t18_cancels_startup_and_leaves_other_sessions_alive() {
    use crate::tools::{ToolUseContext, scope_tool_use_context};
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let mut server = server(root.path(), "local");
    let McpServer::Stdio(stdio) = &mut server else { unreachable!() };
    stdio.args.push("startup-hang".into());
    let pool = McpSessionConnections::default();
    let token = tokio_util::sync::CancellationToken::new();
    let context = Arc::new(ToolUseContext::new("cancel", None).with_abort_token(token.clone()));
    let cancel = async {
        tokio::time::sleep(Duration::from_millis(150)).await;
        token.cancel();
    };
    let (result, ()) = tokio::join!(scope_tool_use_context(context, pid(&pool, &server)), cancel);
    assert!(format!("{:#}", result.unwrap_err()).contains("cancelled"));
    let started = fs::read_to_string(root.path().join("starts")).unwrap();
    #[cfg(unix)]
    exited(started.trim()).await;
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t18_retires_changed_disabled_and_exited_servers() {
    let root = tempfile::tempdir().unwrap();
    let server = server(root.path(), "local");
    let pool = McpSessionConnections::default();
    pool.synchronize(std::slice::from_ref(&server)).unwrap();
    let first = pid(&pool, &server).await.unwrap();
    #[cfg(unix)]
    {
        unsafe {
            libc::kill(first.parse().unwrap(), libc::SIGKILL);
        }
        exited(&first).await;
        // The read side observes EOF asynchronously; do not count a stale peer as connected.
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_ne!(pid(&pool, &server).await.unwrap(), first);
    }
    pool.synchronize(&[]).unwrap();
    assert!(pid(&pool, &server).await.is_err());
    let mut changed = server.clone();
    let McpServer::Stdio(stdio) = &mut changed else { unreachable!() };
    stdio.env.push(agent_client_protocol::EnvVariable::new("T18_CHANGED", "true"));
    pool.synchronize(std::slice::from_ref(&changed)).unwrap();
    assert!(
        pid(&pool, &server).await.is_err(),
        "old definitions must not resurrect a retired connection"
    );
    assert!(pid(&pool, &changed).await.is_ok());
    pool.shutdown();
}

struct HttpFixture {
    child: std::process::Child,
    url: String,
}
impl Drop for HttpFixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
async fn http_fixture(root: &Path, mode: &str) -> HttpFixture {
    let script = root.join("transport_server.py");
    fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t18/http_server.py"))
        .unwrap();
    let child = std::process::Command::new("python3")
        .arg(&script)
        .arg(root)
        .arg(mode)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut fixture = HttpFixture { child, url: String::new() };
    for _ in 0..100 {
        if let Ok(port) = fs::read_to_string(root.join("port")) {
            fixture.url = format!("http://127.0.0.1:{port}/mcp");
            return fixture;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("local HTTP fixture must start");
}

#[tokio::test]
async fn codex_plugin_compat_t18_reuses_streamable_http_and_legacy_sse_sessions() {
    for mode in ["http", "sse"] {
        let root = tempfile::tempdir().unwrap();
        let fixture = http_fixture(root.path(), mode).await;
        let server = if mode == "http" {
            McpServer::Http(agent_client_protocol::McpServerHttp::new("http", &fixture.url))
        } else {
            McpServer::Sse(agent_client_protocol::McpServerSse::new("sse", &fixture.url))
        };
        let pool = McpSessionConnections::default();
        let first = pid(&pool, &server).await.unwrap();
        assert_eq!(pid(&pool, &server).await.unwrap(), first);
        assert_eq!(fs::read_to_string(root.path().join("starts")).unwrap().lines().count(), 1);
        pool.shutdown();
    }
}

#[tokio::test]
async fn codex_plugin_compat_t18_keeps_tool_bindings_but_releases_session_lease() {
    use crate::tools::{Tool, ToolSpec, mcp_tool::McpTool};
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let server = server(root.path(), "local");
    let pool = Arc::new(McpSessionConnections::default());
    let tool: Arc<dyn Tool> = Arc::new(McpTool::from_session_server(
        server.clone(),
        "pid",
        ToolSpec::new("mcp__local__pid", "pid", json!({"type":"object"})),
        pool.clone(),
        None,
    ));
    let tools = crate::session::runtime::SessionToolset::new(vec![tool.clone()])
        .with_mcp_connections(pool.clone());
    let first = tool.call(json!({})).await.unwrap();
    assert_eq!(tool.call(json!({})).await.unwrap().model_result, first.model_result);
    let clone = tools.clone();
    drop(tools);
    assert!(tool.call(json!({})).await.is_ok());
    drop(clone);
    assert!(
        tool.call(json!({})).await.is_err(),
        "retained tool must not retain a retired chat's authority"
    );
}

#[tokio::test]
async fn codex_plugin_compat_t18_plugin_disable_closes_only_its_live_service() {
    for array in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        let package = workspace.join(".vibewindow/plugins/local");
        fs::create_dir_all(package.join(".codex-plugin")).unwrap();
        let server = server(temp.path(), "local");
        let McpServer::Stdio(stdio) = server else { unreachable!() };
        let declaration = if array {
            fs::write(
                package.join(".mcp.json"),
                serde_json::to_vec(&json!([McpServer::Stdio(stdio)])).unwrap(),
            )
            .unwrap();
            json!("./.mcp.json")
        } else {
            json!({"local":{"command":"python3","args":stdio.args,"environment_id":"local","required":true,"startup_readiness":"catalog","startup_timeout_sec":2,"tool_timeout_sec":1}})
        };
        fs::write(
            package.join(".codex-plugin/plugin.json"),
            serde_json::to_vec(&json!({"name":"local","mcpServers":declaration})).unwrap(),
        )
        .unwrap();
        let options = PluginDiscoveryOptions::new(
            workspace.clone(),
            temp.path().join("config"),
            PluginsConfig::default(),
        );
        let mut servers = Vec::new();
        merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
        let pool = crate::plugins::mcp_session_connections("disable-local", &workspace);
        pool.synchronize(&servers).unwrap();
        let first = pid(&pool, &servers[0]).await.unwrap();
        crate::plugins::set_plugin_enabled(&workspace.join(".vibewindow/plugins"), "local", false)
            .unwrap();
        assert!(pid(&pool, &servers[0]).await.is_err());
        #[cfg(unix)]
        exited(&first).await;
        crate::plugins::close_session_mcp_connections("disable-local", &workspace);
    }
}

#[tokio::test]
async fn codex_plugin_compat_t18_required_failure_blocks_actual_tool_execution() {
    use crate::tools::{ToolCallError, ToolUseContext};
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let pool = crate::plugins::mcp_session_connections("required", root.path());
    let mut server =
        McpServer::Stdio(McpServerStdio::new("unavailable", root.path().join("must-not-exist")));
    super::mcp_connection_settings::bind(
        &mut server,
        super::mcp_connection_settings::ConnectionSettings { required: true, ..Default::default() },
    )
    .unwrap();
    assert!(pid(&pool, &server).await.is_err());
    let context =
        Arc::new(ToolUseContext::new("required", Some(root.path().display().to_string())));
    let result =
        crate::tools::execute_tool_from_registry(&[], "must-not-execute", json!({}), context).await;
    assert!(
        matches!(result,Err(ToolCallError::Failed(message)) if message.contains("required MCP server"))
    );
    crate::plugins::close_session_mcp_connections("required", root.path());
}

#[test]
fn codex_plugin_compat_t18_validates_lifecycle_fields_without_secret_diagnostics() {
    for setting in [
        json!({"startup_timeout_sec":-1}),
        json!({"startup_timeout_ms":0}),
        json!({"tool_timeout_sec":"sensitive-invalid"}),
        json!({"required":"sensitive-invalid"}),
        json!({"startup_readiness":"sensitive-invalid"}),
        json!({"environment_id":"sensitive-invalid"}),
    ] {
        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("workspace/.vibewindow/plugins/local");
        fs::create_dir_all(package.join(".codex-plugin")).unwrap();
        let mut config = json!({"command":"must-not-run"});
        config.as_object_mut().unwrap().extend(setting.as_object().unwrap().clone());
        fs::write(
            package.join(".codex-plugin/plugin.json"),
            serde_json::to_vec(&json!({"name":"local","mcpServers":{"local":config}})).unwrap(),
        )
        .unwrap();
        let options = PluginDiscoveryOptions::new(
            root.path().join("workspace"),
            root.path().join("config"),
            PluginsConfig::default(),
        );
        let error = merge_enabled_plugin_mcp_servers(&mut Vec::new(), &options).unwrap_err();
        assert!(!format!("{error:#}").contains("sensitive-invalid"));
    }
}

#[tokio::test]
async fn codex_plugin_compat_t18_applies_host_call_timeout_to_actual_dynamic_tool() {
    use crate::tools::{Tool, ToolSpec, mcp_tool::McpTool};
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let mut server = server(root.path(), "local");
    settings(&mut server, 2000, 10000);
    let pool = Arc::new(McpSessionConnections::default());
    let first = pid(&pool, &server).await.unwrap();
    let tool = McpTool::from_session_server(
        server,
        "hang",
        ToolSpec::new("mcp__local__hang", "hang", json!({"type":"object"})),
        pool.clone(),
        Some(Duration::from_millis(100)),
    );
    let error = tool.call(json!({})).await.unwrap_err();
    assert!(format!("{error:#}").contains("timed out"));
    #[cfg(unix)]
    exited(&first).await;
    pool.shutdown();
}

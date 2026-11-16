//! Real stdio process ownership and lifecycle acceptance.
use plugin_mcp::mcp_connections::McpSessionConnections;

use agent_client_protocol::{McpServer, McpServerStdio};
use serde_json::json;
use std::{fs, path::Path, time::Duration};

fn server(root: &Path, name: &str) -> McpServer {
    let script = root.join("server.py");
    fs::write(&script, include_bytes!("../../../../vw-agent/testdata/plugin_compat/t18/server.py")).unwrap();
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

fn settings(server: &mut McpServer, startup: u64, call: u64) {
    plugin_mcp::mcp_connection_settings::bind(
        server,
        plugin_mcp::mcp_connection_settings::ConnectionSettings {
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
async fn codex_plugin_compat_t18_reuses_connection_until_session_ends() {
    let root = tempfile::tempdir().unwrap();
    let server = server(root.path(), "local");
    let pool = McpSessionConnections::new(super::connection_host());
    let first = pid(&pool, &server).await.unwrap();
    let second = pid(&pool, &server).await.unwrap();
    assert_eq!(first, second, "same session must reuse the actual server process");
    assert_eq!(fs::read_to_string(root.path().join("starts")).unwrap().lines().count(), 1);
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t18_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let server = server(root.path(), "local");
    let first = McpSessionConnections::new(super::connection_host());
    let second = McpSessionConnections::new(super::connection_host());
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

#[tokio::test]
async fn codex_plugin_compat_t18_serializes_startup_and_survives_bridge_runtime_drop() {
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let server = server(root.path(), "local");
    let pool = Arc::new(McpSessionConnections::new(super::connection_host()));
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
        let pool = McpSessionConnections::new(super::connection_host());
        let error = pid(&pool, &server).await.unwrap_err();
        assert!(format!("{error:#}").contains("timed out"), "{error:#}");
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
        let pool = McpSessionConnections::new(super::connection_host());
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
async fn codex_plugin_compat_t18_retires_changed_disabled_and_exited_servers() {
    let root = tempfile::tempdir().unwrap();
    let server = server(root.path(), "local");
    let pool = McpSessionConnections::new(super::connection_host());
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
    fs::write(&script, include_bytes!("../../../../vw-agent/testdata/plugin_compat/t18/http_server.py"))
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
        if let Ok(port) = fs::read_to_string(root.join("port"))
            && let Ok(port) = port.trim().parse::<std::num::NonZeroU16>()
        {
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
        // File creation is not publication: require the actual server to fill its port.
        fs::write(root.path().join("port"), b"").unwrap();
        let fixture = http_fixture(root.path(), mode).await;
        let server = if mode == "http" {
            McpServer::Http(agent_client_protocol::McpServerHttp::new("http", &fixture.url))
        } else {
            McpServer::Sse(agent_client_protocol::McpServerSse::new("sse", &fixture.url))
        };
        let pool = McpSessionConnections::new(super::connection_host());
        let first = pid(&pool, &server).await.unwrap();
        assert_eq!(pid(&pool, &server).await.unwrap(), first);
        assert_eq!(fs::read_to_string(root.path().join("starts")).unwrap().lines().count(), 1);
        pool.shutdown();
    }
}

#[tokio::test]
async fn codex_plugin_compat_t18_applies_host_call_timeout_to_actual_dynamic_tool() {
    use super::tools::{Tool, ToolSpec, mcp_tool::McpTool};
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let mut server = server(root.path(), "local");
    settings(&mut server, 2000, 10000);
    let pool = Arc::new(McpSessionConnections::new(super::connection_host()));
    let first = pid(&pool, &server).await.unwrap();
    let tool = McpTool::from_session_server(
        server,
        "hang",
        ToolSpec::new("mcp__local__hang", "hang", json!({"type":"object"})),
        pool.clone(),
        Some(Duration::from_millis(100)),
    );
    let error = tool.call(json!({})).await.unwrap_err();
    assert!(format!("{error:#}").contains("timed out"), "{error:#}");
    #[cfg(unix)]
    exited(&first).await;
    pool.shutdown();
}

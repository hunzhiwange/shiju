//! Capability negotiation through the real owned stdio transport.
use super::mcp_connections::McpSessionConnections;
use agent_client_protocol::{McpServer, McpServerStdio};
use rmcp::model::CallToolRequestParam;
use serde_json::{Value, json};

fn server(root: &std::path::Path, version: &str) -> McpServer {
    let script = root.join("server.py");
    std::fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t23/server.py"))
        .unwrap();
    McpServer::Stdio(
        McpServerStdio::new("codex_apps", "python3")
            .args(vec![script.display().to_string(), version.to_owned()]),
    )
}

async fn probe(pool: &McpSessionConnections, server: &McpServer) -> anyhow::Result<Value> {
    pool.with_peer(server, Some(std::time::Duration::from_secs(5)), |peer| {
        Box::pin(async move {
            Ok(peer
                .call_tool(CallToolRequestParam { name: "probe".into(), arguments: None })
                .await?
                .structured_content
                .unwrap())
        })
    })
    .await
}

#[tokio::test]
async fn codex_plugin_compat_t23_advertises_only_implemented_capabilities() {
    let root = tempfile::tempdir().unwrap();
    let pool = McpSessionConnections::default();
    let result = probe(&pool, &server(root.path(), "2025-06-18")).await.unwrap();
    assert_eq!(result["initialize"]["capabilities"], json!({}));
    assert_eq!(
        result["initialize"]["protocolVersion"], "2025-06-18",
        "initialize must use the frozen version rather than rmcp's older LATEST"
    );
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t23_rejects_invalid_input() {
    let root = tempfile::tempdir().unwrap();
    let pool = McpSessionConnections::default();
    let result = probe(&pool, &server(root.path(), "2099-01-01")).await;
    assert!(result.is_err(), "an unsupported negotiated protocol must not reach tools/call");
    assert!(format!("{:#}", result.unwrap_err()).contains("unsupported protocol version"));
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t23_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let accepted = McpSessionConnections::default();
    let rejected = McpSessionConnections::default();
    let first_server = server(root.path(), "2025-03-26");
    let first = probe(&accepted, &first_server).await.unwrap();
    assert_eq!(first["initialize"]["capabilities"], json!({}));
    let other = probe(&rejected, &server(root.path(), "2026-07-28")).await;
    assert!(other.is_err(), "a second session must not borrow support from a peer's version");
    assert_eq!(probe(&accepted, &first_server).await.unwrap()["pid"], first["pid"]);
    for capability in ["roots", "sampling", "elicitation"] {
        assert_eq!(first["replies"][capability]["error"]["code"], -32600);
        assert_eq!(first["replies"][capability]["error"]["data"]["capability"], capability);
    }
    accepted.shutdown();
    rejected.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t23_retains_known_fallbacks_without_advertising_roots() {
    for version in ["2024-11-05", "2025-03-26", "2025-06-18"] {
        let root = tempfile::tempdir().unwrap();
        let pool = McpSessionConnections::default();
        let server = server(root.path(), version);
        let result = probe(&pool, &server).await.unwrap();
        assert_eq!(result["initialize"]["protocolVersion"], "2025-06-18");
        let negotiated = pool
            .with_peer(&server, None, |peer| {
                Box::pin(async move { Ok(peer.peer_info().unwrap().protocol_version.to_string()) })
            })
            .await
            .unwrap();
        assert_eq!(negotiated, version);
        for capability in ["roots", "sampling", "elicitation"] {
            assert_eq!(result["replies"][capability]["error"]["code"], -32600);
            assert!(result["replies"][capability].get("result").is_none());
        }
        assert!(!result.to_string().contains(root.path().to_str().unwrap()));
        pool.shutdown();
    }
}

#[tokio::test]
async fn codex_plugin_compat_t23_rejects_unknown_versions_without_reflecting_payloads() {
    for version in ["", "remote-secret-sentinel", "2025-11-25", "2026-07-28"] {
        let root = tempfile::tempdir().unwrap();
        let pool = McpSessionConnections::default();
        let error = probe(&pool, &server(root.path(), version)).await.unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("unsupported protocol version"));
        assert!(!message.contains("remote-secret-sentinel"));
        pool.shutdown();
    }
}

#[tokio::test]
async fn codex_plugin_compat_t23_sends_negotiated_http_version_on_each_request() {
    use agent_client_protocol::McpServerHttp;
    use std::time::Duration;
    for (version, mode) in [
        ("2025-06-18", "json"),
        ("2025-03-26", "json"),
        ("2025-06-18", "sse"),
        ("2025-03-26", "sse"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let script = root.path().join("http_server.py");
        let port = root.path().join("port");
        let log = root.path().join("requests.jsonl");
        std::fs::write(
            &script,
            include_bytes!("../../../testdata/plugin_compat/t23/http_server.py"),
        )
        .unwrap();
        let mut child = tokio::process::Command::new("python3")
            .arg(script)
            .args([version, port.to_str().unwrap(), log.to_str().unwrap(), mode])
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let port = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(port) = std::fs::read_to_string(&port) {
                    break port;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let pool = McpSessionConnections::default();
        let server = McpServer::Http(McpServerHttp::new(
            "codex_apps",
            format!("http://127.0.0.1:{port}/mcp"),
        ));
        let result = probe(&pool, &server).await;
        pool.shutdown();
        if result.is_err() {
            child.kill().await.unwrap();
            child.wait().await.unwrap();
        }
        assert!(result.is_ok(), "negotiated HTTP version must reach the server: {result:?}");
        assert_eq!(result.unwrap()["initialize"]["capabilities"], json!({}));
        let closed = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let requests = std::fs::read_to_string(&log).unwrap();
                if requests.contains("DELETE") {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        let records = std::fs::read_to_string(log).unwrap();
        child.kill().await.unwrap();
        child.wait().await.unwrap();
        closed.unwrap();
        let rows = records
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert!(rows.iter().any(|row| row["method"] == "GET"));
        for row in rows {
            if row["method"] == "initialize" {
                assert!(row["protocol"].is_null());
            } else {
                assert_eq!(
                    row["protocol"], version,
                    "every request retains this connection's version"
                );
            }
        }
    }
}

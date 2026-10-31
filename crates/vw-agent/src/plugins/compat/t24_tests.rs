//! Forms through the real process and session interaction boundary.
use super::mcp_connections::McpSessionConnections;
use crate::{
    plugins::McpFormSession,
    session::ui_types::{McpFormAction, McpFormEvent, McpFormRequest, McpFormResponse},
    tools::{ToolUseContext, scope_tool_use_context},
};
use agent_client_protocol::{McpServer, McpServerStdio};
use rmcp::model::CallToolRequestParam;
use serde_json::{Value, json};
use std::{path::Path, sync::Arc, time::Duration};

fn server(root: &Path, name: &str) -> McpServer {
    let script = root.join("server.py");
    std::fs::write(&script, include_bytes!("../../../testdata/plugin_compat/t24/server.py"))
        .unwrap();
    McpServer::Stdio(McpServerStdio::new(name, "python3").args(vec![script.display().to_string()]))
}

async fn call(pool: &McpSessionConnections, server: &McpServer) -> anyhow::Result<Value> {
    call_args(pool, server, None, Duration::from_secs(5)).await
}

async fn call_args(
    pool: &McpSessionConnections,
    server: &McpServer,
    arguments: Option<serde_json::Map<String, Value>>,
    timeout: Duration,
) -> anyhow::Result<Value> {
    pool.with_peer(server, Some(timeout), |peer| {
        Box::pin(async move {
            Ok(peer
                .call_tool(CallToolRequestParam { name: "form".into(), arguments })
                .await?
                .structured_content
                .unwrap())
        })
    })
    .await
}

async fn request(
    receiver: &mut tokio::sync::broadcast::Receiver<McpFormEvent>,
    call: &mut std::pin::Pin<&mut impl std::future::Future<Output = anyhow::Result<Value>>>,
) -> McpFormRequest {
    tokio::select! {
        event = receiver.recv() => match event.unwrap() {
            McpFormEvent::Requested { request } => request,
            _ => panic!("expected a pending form"),
        },
        result = call => panic!("target: real elicitation must await a user response, got {result:?}"),
        _ = tokio::time::sleep(Duration::from_secs(3)) => panic!("target: real elicitation must reach session interaction"),
    }
}

fn answer() -> McpFormResponse {
    McpFormResponse {
        action: McpFormAction::Accept,
        content: Some(json!({"name":"用户", "age":21,"approved":true})),
    }
}

#[tokio::test]
async fn codex_plugin_compat_t24_returns_validated_user_response() {
    let root = tempfile::tempdir().unwrap();
    let host = McpFormSession::register("t24-accept", root.path()).unwrap();
    let mut events = host.subscribe();
    let pool = McpSessionConnections::default();
    let server = server(root.path(), "public-form");
    let context =
        Arc::new(ToolUseContext::new("t24-accept", Some(root.path().display().to_string())));
    scope_tool_use_context(context, async {
        let operation = call(&pool, &server);
        tokio::pin!(operation);
        let req = request(&mut events, &mut operation.as_mut()).await;
        assert_eq!(req.session_id, "t24-accept");
        assert_eq!(req.server_name, "public-form");
        assert_eq!(req.server_request_id, "form-1");
        host.respond(&req, answer()).unwrap();
        let result = operation.await.unwrap();
        assert_eq!(
            result["form_reply"]["result"],
            json!({"action":"accept","content":{"name":"用户","age":21,"approved":true}})
        );
        assert!(host.pending().is_empty());
        assert!(result["initialize"]["capabilities"].get("elicitation").is_some());
    })
    .await;
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t24_rejects_invalid_input() {
    let root = tempfile::tempdir().unwrap();
    let host = McpFormSession::register("t24-invalid", root.path()).unwrap();
    let mut events = host.subscribe();
    let pool = McpSessionConnections::default();
    let server = server(root.path(), "public-form");
    let context =
        Arc::new(ToolUseContext::new("t24-invalid", Some(root.path().display().to_string())));
    scope_tool_use_context(context, async {
        let operation = call(&pool, &server);
        tokio::pin!(operation);
        let req = request(&mut events, &mut operation.as_mut()).await;
        let mut invalid = answer();
        invalid.content.as_mut().unwrap()["age"] = json!(17);
        assert!(host.respond(&req, invalid).is_err());
        assert_eq!(host.pending().len(), 1, "invalid data cannot consume its responder");
        host.respond(&req, answer()).unwrap();
        assert_eq!(operation.await.unwrap()["form_reply"]["result"]["action"], "accept");
    })
    .await;
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t24_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let host = McpFormSession::register("t24-owner", root.path()).unwrap();
    let other = McpFormSession::register("t24-other", root.path()).unwrap();
    let mut events = host.subscribe();
    let pool = McpSessionConnections::default();
    let server = server(root.path(), "public-form");
    let context =
        Arc::new(ToolUseContext::new("t24-owner", Some(root.path().display().to_string())));
    scope_tool_use_context(context, async {
        let operation = call(&pool, &server);
        tokio::pin!(operation);
        let req = request(&mut events, &mut operation.as_mut()).await;
        assert!(other.respond(&req, answer()).is_err());
        let mut forged = req.clone();
        forged.server_name = "another-server".into();
        assert!(host.respond(&forged, answer()).is_err());
        assert_eq!(host.pending().len(), 1);
        host.respond(&req, McpFormResponse { action: McpFormAction::Decline, content: None })
            .unwrap();
        assert_eq!(operation.await.unwrap()["form_reply"]["result"], json!({"action":"decline"}));
        assert!(host.respond(&req, answer()).is_err());
    })
    .await;
    pool.shutdown();
}

#[path = "t24_lifecycle_tests.rs"]
mod lifecycle;
#[path = "t24_privacy_tests.rs"]
mod privacy;
#[path = "t24_schema_tests.rs"]
mod schema;

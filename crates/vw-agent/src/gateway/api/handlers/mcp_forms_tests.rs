//! Host capabilities are random, short lived and tenant bound.
use super::*;
use axum::http::HeaderMap;
use vw_api_types::mcp_form::McpFormHostOpen;

#[tokio::test]
async fn codex_plugin_compat_t25_gateway_host_rejects_foreign_tenant_and_closes() {
    let temp = tempfile::tempdir().unwrap();
    let Json(opened) = open(
        HeaderMap::new(),
        Json(McpFormHostOpen {
            session_id: uuid::Uuid::new_v4().to_string(),
            workspace: Some(temp.path().to_string_lossy().into_owned()),
        }),
    )
    .await
    .unwrap();
    assert!(
        poll(Json(McpFormHostRef { host_id: opened.host_id.clone() })).await.unwrap().0.is_empty()
    );
    let tenant = TenantScope::new("foreign-t25", "foreign-t25").unwrap();
    let foreign = ui_store::with_tenant_scope(
        tenant,
        poll(Json(McpFormHostRef { host_id: opened.host_id.clone() })),
    )
    .await;
    assert!(foreign.is_err());
    assert!(close(Json(McpFormHostRef { host_id: opened.host_id.clone() })).await.unwrap().0);
    assert!(poll(Json(McpFormHostRef { host_id: opened.host_id })).await.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "explicit native window fixture; requires VW_MCP_FORM_WINDOW_ROOT"]
async fn codex_plugin_compat_t25_window_gateway_fixture() {
    use axum::{extract::State, routing::post};
    use serde_json::{Value, json};
    use std::{path::PathBuf, sync::Arc};
    let root =
        PathBuf::from(std::env::var("VW_MCP_FORM_WINDOW_ROOT").expect("explicit fixture root"));
    assert!(root.is_absolute() && root.is_dir());
    let script = root.join("server.py");
    std::fs::write(&script, include_bytes!("../../../../testdata/plugin_compat/t24/server.py"))
        .unwrap();
    async fn trigger(State(root): State<Arc<PathBuf>>, Json(body): Json<Value>) -> Json<Value> {
        use crate::{
            plugins::McpSessionConnections,
            tools::{ToolUseContext, scope_tool_use_context},
        };
        use agent_client_protocol::{McpServer, McpServerStdio};
        let session = body["session"].as_str().unwrap().to_owned();
        let pool = McpSessionConnections::default();
        let server = McpServer::Stdio(
            McpServerStdio::new("中文公开表单服务", "python3")
                .args(vec![root.join("server.py").display().to_string()]),
        );
        let form: Value =
            serde_json::from_str(include_str!("../../../../testdata/plugin_compat/t25/form.json"))
                .unwrap();
        let params = json!({"message":form["message"],"requestedSchema":form["schema"]});
        let args = json!({"params": params}).as_object().unwrap().clone();
        let context = Arc::new(ToolUseContext::new(&session, Some(root.display().to_string())));
        let result = scope_tool_use_context(
            context,
            pool.with_peer(&server, Some(Duration::from_secs(110)), |peer| {
                Box::pin(async move {
                    use rmcp::model::CallToolRequestParam;
                    let result = peer
                        .call_tool(CallToolRequestParam {
                            name: "form".into(),
                            arguments: Some(args),
                        })
                        .await?;
                    Ok(result.structured_content.unwrap())
                })
            }),
        )
        .await
        .unwrap();
        pool.shutdown();
        Json(result)
    }
    let app = axum::Router::new()
        .nest("/v1", router())
        .route("/trigger", post(trigger))
        .with_state(Arc::new(root.clone()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    std::fs::write(root.join("ready.json"), json!({"port":port}).to_string()).unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            while !root.join("stop").exists() {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn codex_plugin_compat_t25_gateway_host_expiry_revokes_old_capability_before_reopening() {
    let temp = tempfile::tempdir().unwrap();
    let body = McpFormHostOpen {
        session_id: uuid::Uuid::new_v4().to_string(),
        workspace: Some(temp.path().display().to_string()),
    };
    let Json(first) = open(HeaderMap::new(), Json(body.clone())).await.unwrap();
    assert!(
        open(HeaderMap::new(), Json(body.clone())).await.is_err(),
        "a second host cannot take over a live lease"
    );
    hosts().lock().unwrap().get_mut(&first.host_id).unwrap().touched =
        Instant::now() - LEASE_IDLE - Duration::from_secs(1);
    assert!(poll(Json(McpFormHostRef { host_id: first.host_id.clone() })).await.is_err());
    let Json(second) = open(HeaderMap::new(), Json(body)).await.unwrap();
    assert_ne!(first.host_id, second.host_id);
    assert!(poll(Json(McpFormHostRef { host_id: first.host_id })).await.is_err());
    assert!(close(Json(McpFormHostRef { host_id: second.host_id })).await.unwrap().0);
}

use super::{mcp_connections::McpSessionConnections, mcp_elicitation::FormOwner};
use serde_json::{Value, json};
#[tokio::test]
#[ignore = "requires an unlocked desktop running the task24 UI harness"]
async fn codex_plugin_compat_t24_desktop_roundtrip() {
    let (_, gateway) =
        crate::gateway::start(crate::gateway::ServeOptions { port: 43124, ..Default::default() })
            .await
            .unwrap();
    let pool = McpSessionConnections::for_form_owner(crate::plugins::compat::mcp_connections::host(), FormOwner {
        session: "t24-desktop".into(),
        tenant: crate::session::ui_store::current_tenant_scope(),
    });
    let server: agent_client_protocol::McpServer = serde_json::from_value(json!({
        "name":"表单验收", "command":"python3", "args":[format!("{}/testdata/plugin_compat/t24/server.py", env!("CARGO_MANIFEST_DIR"))], "env":[]
    })).unwrap();
    eprintln!("TASK24_DESKTOP_READY: enter age 24, choose a and accept, then submit");
    let result = pool
        .with_peer(&server, Some(std::time::Duration::from_secs(180)), |peer| {
            Box::pin(async move {
                let response = peer
                    .call_tool(
                        serde_json::from_value(json!({"name":"form","arguments":{}})).unwrap(),
                    )
                    .await?;
                Ok(serde_json::to_value(response)?)
            })
        })
        .await
        .unwrap();
    let response: Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    eprintln!("TASK24_SYNTHETIC_RESPONSE: {response}");
    assert_eq!(response["result"], json!({"action":"accept","content":{"age":24,"choice":"a"}}));
    pool.shutdown();
    gateway.abort();
}

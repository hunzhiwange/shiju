use super::mcp_elicitation::FormOwner;
use crate::question;
use serde_json::{Value, json};
use std::time::Duration;
#[tokio::test]
async fn codex_plugin_compat_t27_url_stdio_completes_exact_request() {
    use super::mcp_connections::McpSessionConnections;
    let pool = std::sync::Arc::new(McpSessionConnections::for_form_owner(crate::plugins::compat::mcp_connections::host(), FormOwner {
        session: "t27-stdio-url".into(),
        tenant: crate::session::ui_store::current_tenant_scope(),
    }));
    let task_pool = pool.clone();
    let server = serde_json::from_value(json!({"name":"url-fixture","command":"python3","args":[format!("{}/testdata/plugin_compat/t27/server.py",env!("CARGO_MANIFEST_DIR"))],"env":[]})).unwrap();
    let task = tokio::spawn(async move {
        task_pool
            .with_peer(&server, None, |peer| {
                Box::pin(async move {
                    let result = peer
                        .call_tool(
                            serde_json::from_value(json!({"name":"authorize","arguments":{}}))
                                .unwrap(),
                        )
                        .await?;
                    Ok(serde_json::to_value(result)?)
                })
            })
            .await
            .unwrap()
    });
    let pending = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(pending) =
                question::list().into_iter().find(|q| q.session_id == "t27-stdio-url")
            {
                break pending;
            }
            assert!(!task.is_finished(), "URL request must reach session interaction");
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert!(pending.questions[0].question.contains("url-completion-91"));
    question::reply(question::ReplyInput {
        request_id: pending.id,
        answers: vec![vec!["accept".into()]],
    });
    let result = tokio::time::timeout(Duration::from_secs(10), task).await.unwrap().unwrap();
    pool.shutdown();
    let response: Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(response["id"], 91);
    assert_eq!(response["result"]["action"], "accept");
    assert!(response["result"].get("content").is_none_or(Value::is_null));
}

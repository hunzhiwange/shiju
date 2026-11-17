use plugin_mcp::{mcp_connections::McpSessionConnections, mcp_elicitation::FormOwner};
use agent_question as question;
use serde_json::{Value, json};
use std::sync::Arc;

async fn roundtrip(session: &str, answer: Option<Vec<Vec<String>>>) -> Value {
    let owner = FormOwner {
        session: session.into(),
        tenant: Default::default(),
    };
    let pool = Arc::new(McpSessionConnections::for_form_owner(super::connection_host(), owner));
    let server: agent_client_protocol::McpServer = serde_json::from_value(json!({
        "name":"form-fixture", "command":"python3", "args":[format!("{}/../../vw-agent/testdata/plugin_compat/t24/server.py", env!("CARGO_MANIFEST_DIR"))], "env":[]
    })).unwrap();
    let task_pool = pool.clone();
    let task = tokio::spawn(async move {
        task_pool
            .with_peer(&server, None, |peer| {
                Box::pin(async move {
                    let result = peer
                        .call_tool(
                            serde_json::from_value(json!({"name":"form","arguments":{}})).unwrap(),
                        )
                        .await?;
                    Ok(serde_json::to_value(result)?)
                })
            })
            .await
            .unwrap()
    });
    let request = tokio::time::timeout(std::time::Duration::from_secs(20), async {
        loop {
            if task.is_finished() {
                break None;
            }
            if let Some(request) = question::list(Default::default()).into_iter().find(|r| r.session_id == session) {
                break Some(request);
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let Some(request) = request else {
        let result = task.await.unwrap();
        pool.shutdown();
        return serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    };
    let form =
        request.mcp_form.as_ref().expect("MCP metadata reaches the trusted question request");
    assert_eq!(form.server, "form-fixture");
    assert_eq!(form.schema["properties"]["age"]["minimum"], 18);
    assert_eq!(request.questions.len(), 3);
    assert_eq!(request.questions[1].options.len(), 2);
    match answer {
        Some(mut answers) => {
            if answers.len() == 2 {
                answers.insert(1, vec!["a".into()]);
            }
            question::reply(question::ReplyInput { request_id: request.id, answers }, Default::default());
        }
        None => question::reject(request.id, Default::default()),
    }
    let result =
        tokio::time::timeout(std::time::Duration::from_secs(20), task).await.unwrap().unwrap();
    pool.shutdown();
    serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap()
}
#[tokio::test]
async fn codex_plugin_compat_t24_returns_validated_user_response() {
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = events.clone();
    let unsubscribe = agent_bus::subscribe(question::event::REPLIED, move |payload| {
        if payload["properties"]["sessionID"] == "t24-accept" {
            captured.lock().unwrap().push(payload);
        }
    });
    let accepted =
        roundtrip("t24-accept", Some(vec![vec!["24".into()], vec!["accept".into()]])).await;
    assert_eq!(accepted["result"], json!({"action":"accept","content":{"age":24,"choice":"a"}}));
    unsubscribe();
    let events = events.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert!(events[0]["properties"]["answers"].is_null());
    drop(events);
    let declined = roundtrip("t24-decline", Some(vec![vec![], vec!["decline".into()]])).await;
    assert_eq!(declined["result"], json!({"action":"decline"}));
    let cancelled = roundtrip("t24-cancel", None).await;
    assert_eq!(cancelled["result"], json!({"action":"cancel"}));
}
#[tokio::test]
async fn codex_plugin_compat_t24_rejects_invalid_input() {
    let response =
        roundtrip("t24-invalid", Some(vec![vec!["17".into()], vec!["accept".into()]])).await;
    assert_eq!(response["error"]["code"], -32602);
    assert!(response.get("result").is_none());
    let invalid_enum = roundtrip(
        "t24-invalid-enum",
        Some(vec![vec!["24".into()], vec!["c".into()], vec!["accept".into()]]),
    )
    .await;
    assert_eq!(invalid_enum["error"]["code"], -32602);
}
#[tokio::test]
async fn codex_plugin_compat_t24_preserves_isolation() {
    let (first, second) = tokio::join!(
        roundtrip("t24-first", Some(vec![vec!["24".into()], vec!["accept".into()]])),
        roundtrip("t24-second", Some(vec![vec!["42".into()], vec!["accept".into()]])),
    );
    assert_eq!(first["result"]["content"]["age"], 24);
    assert_eq!(second["result"]["content"]["age"], 42);
    assert!(mcp_client::capabilities::client_info().capabilities.elicitation.is_none());
}

#[tokio::test(start_paused = true)]
async fn codex_plugin_compat_t24_timeout_removes_pending_form() {
    let owner = FormOwner {
        session: "t24-timeout".into(),
        tenant: Default::default(),
    };
    let request = serde_json::from_value(
        json!({"message":"Synthetic timeout", "requestedSchema":{"type":"object","properties":{}}}),
    )
    .unwrap();
    let task = tokio::spawn(plugin_mcp::mcp_elicitation::request(
        owner,
        "fixture".into(),
        request,
        Default::default(),
        Default::default(),
        None,
    ));
    tokio::task::yield_now().await;
    assert!(question::list(Default::default()).iter().any(|q| q.session_id == "t24-timeout"));
    tokio::time::advance(std::time::Duration::from_secs(121)).await;
    assert_eq!(task.await.unwrap().unwrap().action, rmcp::model::ElicitationAction::Cancel);
    assert!(!question::list(Default::default()).iter().any(|q| q.session_id == "t24-timeout"));
}

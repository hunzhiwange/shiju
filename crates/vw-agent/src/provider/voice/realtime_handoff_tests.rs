use super::*;
use crate::provider::voice::realtime::RealtimeSession;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use vw_api_types::voice::*;
use vw_config_types::voice::VoiceConfig;
fn ident(value: &str) -> VoiceId {
    value.to_owned().try_into().unwrap()
}
fn events(status: &str) -> Vec<Value> {
    vec![
        json!({"type":"response.created","response":{"id":"r1"}}),
        json!({"type":"response.output_item.added","response_id":"r1","item":{"type":"function_call","id":"tool1","call_id":"call1","name":"shiju_handoff"}}),
        json!({"type":"response.function_call_arguments.done","response_id":"r1","item_id":"tool1","call_id":"call1","name":"shiju_handoff","arguments":"{\"instruction\":\"整理项目\"}"}),
        json!({"type":"response.output_item.done","response_id":"r1","item":{"type":"function_call","id":"tool1","status":"completed","call_id":"call1","name":"shiju_handoff","arguments":"{\"instruction\":\"整理项目\"}"}}),
        json!({"type":"response.done","response":{"id":"r1","status":status}}),
    ]
}
async fn fixture(events: Vec<Value>) -> (RealtimeSession, tokio::task::JoinHandle<Vec<Value>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
        ws.send(Message::Text(
            json!({"type":"session.created","session":{"id":"upstream"}}).to_string().into(),
        ))
        .await
        .unwrap();
        let update: Value =
            serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
        assert_eq!(update["session"]["tools"], tools());
        let mut session = update["session"].clone();
        session["id"] = json!("upstream");
        ws.send(Message::Text(
            json!({"type":"session.updated","session":session}).to_string().into(),
        ))
        .await
        .unwrap();
        for event in events {
            ws.send(Message::Text(event.to_string().into())).await.unwrap();
        }
        let mut commands = Vec::new();
        while let Some(Ok(Message::Text(text))) = ws.next().await {
            commands.push(serde_json::from_str(&text).unwrap());
        }
        commands
    });
    let session = RealtimeSession::connect(
        &base,
        "synthetic",
        reqwest::Client::builder().no_proxy(),
        &VoiceConfig { realtime_enabled: true, ..Default::default() },
        ident("openai"),
        ident("voice1"),
        TurnMode::Manual,
    )
    .await
    .unwrap();
    (session, task)
}
#[tokio::test]
async fn voice_task_12_service_tool_waits_for_all_completions_and_returns_denial_once() {
    let mut wire = events("completed");
    wire.extend(wire[2..].to_vec());
    let count = wire.len();
    let (mut session, server) = fixture(wire).await;
    for _ in 0..4 {
        session.next().await.unwrap();
        assert!(session.take_handoffs().is_empty());
    }
    session.next().await.unwrap();
    let calls = session.take_handoffs();
    assert_eq!(calls.len(), 1);
    for _ in 5..count {
        session.next().await.unwrap();
        assert!(session.take_handoffs().is_empty());
    }
    session.handoff_result(&calls[0], HandoffStatus::Failed, "工作审批已拒绝").await.unwrap();
    session.stop().await;
    let commands = server.await.unwrap();
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0]["item"]["type"], "function_call_output");
    assert_eq!(commands[0]["item"]["call_id"], "call1");
    let output: Value =
        serde_json::from_str(commands[0]["item"]["output"].as_str().unwrap()).unwrap();
    assert_eq!(output["status"], "failed");
    assert_eq!(output["text"], "工作审批已拒绝");
    assert_eq!(commands[1]["type"], "response.create");
    assert_eq!(commands[1]["response"]["tools"], json!([]));
}
#[tokio::test]
async fn voice_task_12_cancelled_response_never_releases_work() {
    for status in ["cancelled", "failed", "incomplete"] {
        let wire = events(status);
        let count = wire.len();
        let (mut session, server) = fixture(wire).await;
        for _ in 0..count {
            session.next().await.unwrap();
        }
        assert!(session.take_handoffs().is_empty());
        session.stop().await;
        assert!(server.await.unwrap().is_empty());
    }
}
#[tokio::test]
async fn voice_task_12_arguments_cannot_choose_tenant_or_session() {
    let mut wire = events("completed");
    wire[2]["arguments"] = json!("{\"instruction\":\"work\",\"tenant\":\"other\"}");
    let (mut session, server) = fixture(wire).await;
    session.next().await.unwrap();
    session.next().await.unwrap();
    assert_eq!(session.next().await.unwrap_err(), VoiceErrorCode::ProtocolError);
    assert!(session.take_handoffs().is_empty());
    assert!(server.await.unwrap().is_empty());
}
#[tokio::test]
async fn voice_task_12_interrupted_tool_and_late_results_are_discarded() {
    let wire = events("completed");
    let (mut session, server) = fixture(wire).await;
    session.next().await.unwrap();
    session.next().await.unwrap();
    session
        .input(
            VoiceCounter::FIRST,
            VoiceEvent::ResponseInterrupt {
                response_id: ident("r1"),
                item_id: None,
                content_index: None,
                audio_end_ms: None,
            },
        )
        .await
        .unwrap();
    for _ in 0..3 {
        session.next().await.unwrap();
    }
    assert!(session.take_handoffs().is_empty());
    session.stop().await;
    server.await.unwrap();
}
#[test]
fn voice_task_12_arguments_and_result_bounds_hold_at_unicode_boundaries() {
    assert!(instruction(&json!({"instruction":"中".repeat(4001)}).to_string()).is_err());
    assert!(instruction("{\"instruction\":\"\"}").is_err());
    assert!(instruction("{\"instruction\":\"a\",\"instruction\":\"b\"}").is_err());
    let text = "a".repeat(9000);
    assert!(instruction(&json!({"instruction":text}).to_string()).is_err());
}

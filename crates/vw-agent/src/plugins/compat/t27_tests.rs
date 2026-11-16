async fn url_interaction(session: &str, answer: Option<&str>) -> rmcp::model::ElicitationAction {
    use super::mcp_elicitation::{FormOwner, request};
    use crate::question;
    let owner = FormOwner {
        session: session.into(),
        tenant: crate::session::ui_store::current_tenant_scope(),
    };
    let params = serde_json::json!({"mode":"url","message":"Authorize public fixture","url":"https://auth.example/authorize","elicitationId":session});
    let task = tokio::spawn(request(owner, "fixture".into(), serde_json::from_value(serde_json::json!({"message":"Authorize public fixture","requestedSchema":{"type":"object","properties":{}}})).unwrap(), tokio_util::sync::CancellationToken::new(), tokio_util::sync::CancellationToken::new(), Some(params)));
    let pending = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let Some(pending) = question::list().into_iter().find(|q| q.session_id == session) {
                break pending;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert!(pending.questions[0].question.contains(session));
    assert!(pending.questions[0].question.contains("https://auth.example/authorize"));
    match answer {
        Some(action) => question::reply(question::ReplyInput {
            request_id: pending.id,
            answers: vec![vec![action.into()]],
        }),
        None => question::reject(pending.id),
    }
    let result = task.await.unwrap().unwrap();
    assert!(result.content.is_none());
    assert!(!question::list().iter().any(|q| q.session_id == session));
    result.action
}
#[tokio::test]
async fn codex_plugin_compat_t27_url_accepts_user_confirmation() {
    assert_eq!(
        url_interaction("t27-url-accept", Some("accept")).await,
        rmcp::model::ElicitationAction::Accept
    );
}
#[tokio::test]
async fn codex_plugin_compat_t27_url_declines_user_denial() {
    assert_eq!(
        url_interaction("t27-url-decline", Some("decline")).await,
        rmcp::model::ElicitationAction::Decline
    );
}
#[tokio::test]
async fn codex_plugin_compat_t27_url_cancels_rejected_question() {
    assert_eq!(
        url_interaction("t27-url-cancel", None).await,
        rmcp::model::ElicitationAction::Cancel
    );
}

#[tokio::test]
async fn codex_plugin_compat_t27_url_cancels_closed_session() {
    let closed = tokio_util::sync::CancellationToken::new();
    closed.cancel();
    let result = super::mcp_elicitation::request(
        super::mcp_elicitation::FormOwner { session: "t27-closed".into(), tenant: crate::session::ui_store::current_tenant_scope() },
        "fixture".into(), serde_json::from_value(serde_json::json!({"message":"authorize","requestedSchema":{"type":"object","properties":{}}})).unwrap(),
        tokio_util::sync::CancellationToken::new(), closed,
        Some(serde_json::json!({"mode":"url","message":"authorize","url":"https://auth.example/a","elicitationId":"closed"})),
    ).await.unwrap();
    assert_eq!(result.action, rmcp::model::ElicitationAction::Cancel);
    assert!(!crate::question::list().iter().any(|q| q.session_id == "t27-closed"));
}
#[test]
fn codex_plugin_compat_t27_url_wire_preserves_request_identity() {
    let raw = super::mcp_resources::RawResponse::default();
    let params = serde_json::json!({"mode":"url","message":"authorize","url":"https://auth.example/a","elicitationId":"url-identity"});
    let mut message =
        serde_json::json!({"jsonrpc":"2.0","id":71,"method":"elicitation/create","params":params});
    raw.capture(&message);
    super::mcp_content::preserve_mcp_wire_result(&mut message);
    let _: rmcp::model::ServerJsonRpcMessage = serde_json::from_value(message).unwrap();
    assert!(raw.take_form(&rmcp::model::RequestId::Number(72)).is_none());
    assert_eq!(raw.take_form(&rmcp::model::RequestId::Number(71)).unwrap(), params);
    assert!(raw.take_form(&rmcp::model::RequestId::Number(71)).is_none());
}

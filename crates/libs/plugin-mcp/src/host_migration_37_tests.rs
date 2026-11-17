async fn url_interaction(session: &str, answer: Option<&str>) -> rmcp::model::ElicitationAction {
    use crate::mcp_elicitation::{FormOwner, request};
    use agent_question as question;
    let owner = FormOwner {
        session: session.into(),
        tenant: Default::default(),
    };
    let params = serde_json::json!({"mode":"url","message":"Authorize public fixture","url":"https://auth.example/authorize","elicitationId":session});
    let task = tokio::spawn(request(owner, "fixture".into(), serde_json::from_value(serde_json::json!({"message":"Authorize public fixture","requestedSchema":{"type":"object","properties":{}}})).unwrap(), tokio_util::sync::CancellationToken::new(), tokio_util::sync::CancellationToken::new(), Some(params)));
    let pending = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let Some(pending) = question::list(Default::default()).into_iter().find(|q| q.session_id == session) {
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
        }, Default::default()),
        None => question::reject(pending.id, Default::default()),
    }
    let result = task.await.unwrap().unwrap();
    assert!(result.content.is_none());
    assert!(!question::list(Default::default()).iter().any(|q| q.session_id == session));
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
    let result = crate::mcp_elicitation::request(
        crate::mcp_elicitation::FormOwner { session: "t27-closed".into(), tenant: Default::default() },
        "fixture".into(), serde_json::from_value(serde_json::json!({"message":"authorize","requestedSchema":{"type":"object","properties":{}}})).unwrap(),
        tokio_util::sync::CancellationToken::new(), closed,
        Some(serde_json::json!({"mode":"url","message":"authorize","url":"https://auth.example/a","elicitationId":"closed"})),
    ).await.unwrap();
    assert_eq!(result.action, rmcp::model::ElicitationAction::Cancel);
    assert!(!agent_question::list(Default::default()).iter().any(|q| q.session_id == "t27-closed"));
}
#[test]
fn codex_plugin_compat_t27_url_wire_preserves_request_identity() {
    let raw = crate::mcp_resources::RawResponse::default();
    let params = serde_json::json!({"mode":"url","message":"authorize","url":"https://auth.example/a","elicitationId":"url-identity"});
    let mut message =
        serde_json::json!({"jsonrpc":"2.0","id":71,"method":"elicitation/create","params":params});
    raw.capture(&message);
    crate::mcp_content::preserve_mcp_wire_result(&mut message);
    let _: rmcp::model::ServerJsonRpcMessage = serde_json::from_value(message).unwrap();
    assert!(raw.take_form(&rmcp::model::RequestId::Number(72)).is_none());
    assert_eq!(raw.take_form(&rmcp::model::RequestId::Number(71)).unwrap(), params);
    assert!(raw.take_form(&rmcp::model::RequestId::Number(71)).is_none());
}

#[test]
fn codex_plugin_compat_t19_keeps_empty_allowlist_and_deny_precedence() {
    use crate::{mcp_connection_settings as settings, mcp_legacy::parse_legacy_mcp};
    let root = tempfile::tempdir().unwrap();
    for (allow, expected) in [(serde_json::json!([]), false), (serde_json::json!(["read"]), true)] {
        let mut outcome=parse_legacy_mcp(serde_json::json!({"catalog":{"command":"unused","enabled_tools":allow,"disabled_tools":["blocked"]}}),"fixture").unwrap();
        let mut entries = settings::prepare(&mut outcome, root.path()).unwrap();
        let mut server = outcome.into_runtime_servers("fixture").unwrap().remove(0);
        settings::bind(&mut server, entries.remove("catalog").unwrap()).unwrap();
        assert_eq!(settings::tool_allowed(&server, "read").unwrap(), expected);
        assert!(!settings::tool_allowed(&server, "blocked").unwrap());
    }
}

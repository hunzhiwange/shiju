use super::mcp_oauth_flow::PendingAuthorization;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

fn pending() -> PendingAuthorization {
    PendingAuthorization::new(
        "https://auth.example/authorize",
        "public-client",
        "http://127.0.0.1:8432/callback",
        "https://mcp.example/mcp",
    )
    .unwrap()
}
fn callback(flow: &PendingAuthorization) -> String {
    let state = flow
        .authorization_url
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .into_owned();
    format!("http://127.0.0.1:8432/callback?code=fixture-code&state={state}")
}
#[test]
fn codex_plugin_compat_t27_accepts_only_matching_state_and_pkce() {
    let flow = pending();
    let challenge = flow
        .authorization_url
        .query_pairs()
        .find(|(key, _)| key == "code_challenge")
        .unwrap()
        .1
        .into_owned();
    let reply = callback(&flow);
    let (code, verifier) = flow.complete(&reply).unwrap();
    assert_eq!(code, "fixture-code");
    assert_eq!(challenge, URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())));
    assert_eq!(verifier.len(), 43);
}
#[test]
fn codex_plugin_compat_t27_rejects_invalid_input() {
    assert!(
        PendingAuthorization::new(
            "https://auth.example/authorize",
            "client",
            "http://127.0.0.1:0/callback",
            "https://mcp.example/mcp"
        )
        .is_err()
    );
    assert!(
        pending().complete("http://127.0.0.1:8432/callback?state=forged&code=fixture").is_err()
    );
    let flow = pending();
    let reply = callback(&flow) + "&state=duplicate";
    assert!(flow.complete(&reply).is_err());
    let flow = pending();
    let reply = callback(&flow).replace("8432", "8433");
    assert!(flow.complete(&reply).is_err());
}
#[test]
fn codex_plugin_compat_t27_preserves_isolation() {
    let first = pending();
    let second = pending();
    assert_ne!(first.authorization_url, second.authorization_url);
    assert!(second.complete(&callback(&first)).is_err());
}

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

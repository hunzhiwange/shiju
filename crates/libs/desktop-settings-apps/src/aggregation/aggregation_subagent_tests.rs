use super::subagent::*;
use gateway_app_settings::AgentAppsSettingsError;
use vw_api_app_aggregation::*;
use vw_api_chat_identity::ReasoningEffortDto as Effort;
fn snapshot() -> CodexAggregationSettings {
    let requested = CodexSettingsValues {
        subagent_model_id: Some("published-a".into()),
        subagent_effort: Some(Effort::High),
        auto_review_model_id: Some("review-keep".into()),
        classic_tools: false,
    };
    CodexAggregationSettings {
        revision: "r1".into(),
        effective: requested.clone(),
        requested,
        choices: vec![
            CodexSettingsChoice {
                model_id: "published-a".into(),
                label: "中文模型／日本語".into(),
                efforts: vec![Effort::Low, Effort::High],
            },
            CodexSettingsChoice {
                model_id: "published-b".into(),
                label: "另一个模型".into(),
                efforts: vec![Effort::Low],
            },
        ],
        notices: vec![AggregationNotice::NewSession],
    }
}
fn loaded() -> Form {
    let state = snapshot();
    Form { draft: state.requested.clone(), snapshot: Some(state), ..Default::default() }
}
fn context() -> Context {
    use desktop_provider_apps::{AgentAppsContext, Scope};
    Context {
        scope: Scope {
            context: AgentAppsContext {
                gateway: "http://127.0.0.1".into(),
                tenant: Default::default(),
                project: Some("/fixture".into()),
            },
            host: "host-t60".into(),
            app: vw_api_app_identity::AppId::Codex,
        },
        project: "project-t60".into(),
        access: 4,
    }
}
#[test]
fn aggregation_t60_aggregation_subagent_preserves_contract() {
    let state = snapshot();
    let mut form =
        Form { draft: state.requested.clone(), snapshot: Some(state), ..Default::default() };
    form.select_model(None);
    assert_eq!(form.draft.subagent_model_id, None);
    assert_eq!(form.draft.subagent_effort, None, "following main must clear reasoning override");
    assert_eq!(form.draft.auto_review_model_id.as_deref(), Some("review-keep"));
    assert!(!form.draft.classic_tools);
}
#[test]
fn aggregation_t60_switch_model_only_retains_supported_effort() {
    let mut form = loaded();
    form.select_model(Some("published-b".into()));
    assert_eq!(form.draft.subagent_effort, None);
    form.select_effort(Some(Effort::High));
    assert_eq!(form.draft.subagent_effort, None);
    form.select_effort(Some(Effort::Low));
    form.select_model(Some("published-a".into()));
    assert_eq!(form.draft.subagent_effort, Some(Effort::Low));
    form.select_effort(None);
    assert_eq!(form.draft.subagent_effort, None);
    form.select_model(Some("unpublished".into()));
    assert_eq!(form.draft.subagent_model_id.as_deref(), Some("published-a"));
}
#[test]
fn aggregation_t60_removed_model_keeps_intent_and_effective_state_distinct() {
    let mut form = loaded();
    let state = form.snapshot.as_mut().unwrap();
    state.choices.clear();
    state.effective.subagent_model_id = None;
    state.effective.subagent_effort = None;
    assert!(form.unavailable());
    assert!(form.selected().is_none());
    assert_eq!(form.draft.subagent_model_id.as_deref(), Some("published-a"));
    let write = form.write(&context()).unwrap();
    assert_eq!(write.expected_revision, "r1");
    assert_eq!(write.requested.auto_review_model_id.as_deref(), Some("review-keep"));
    form.select_model(None);
    assert!(!form.unavailable());
    assert_eq!(form.write(&context()).unwrap().requested.subagent_effort, None);
}
#[test]
fn aggregation_t60_errors_preserve_draft_and_require_refresh() {
        for status in [401, 403, 409, 422, 501, 502, 503, 504] {
        let mut form = loaded();
        form.pending = Some(10);
        assert!(!form.finish(9, true, Ok(snapshot())));
        assert_eq!(form.pending, Some(10));
        assert!(form.finish(
            10,
            true,
            Err(AgentAppsSettingsError::Http {
                status,
                body: serde_json::json!({"secret":"never display"})
            })
        ));
        assert!(form.snapshot.is_none());
        assert!(form.write(&context()).is_none());
        assert!(!form.saved);
        assert_eq!(form.draft.subagent_model_id.as_deref(), Some("published-a"));
        assert!(!form.error.unwrap().contains("secret"));
        form.pending = Some(11);
        assert!(form.finish(11, false, Ok(snapshot())));
        assert!(form.error.is_none());
        assert!(form.write(&context()).is_some());
    }
}
#[tokio::test]
async fn aggregation_t60_form_calls_sdk_with_scope_revision_and_clear_values() {
    use std::io::{Read, Write};
    use {gateway_endpoint::{GatewayAuth, GatewayEndpoint}, {vw_gateway_client::GatewayClient}};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let mut captured = Vec::new();
        for method in ["GET", "PUT"] {
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && std::time::Instant::now() < deadline =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(10))
                    }
                    Err(e) => panic!("HTTP fixture accept: {e}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut bytes = vec![];
            let mut chunk = [0; 4096];
            loop {
                let n = stream.read(&mut chunk).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&chunk[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                    let len = headers
                        .lines()
                        .find_map(|l| {
                            l.strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + len {
                        break;
                    }
                }
            }
            let input = String::from_utf8(bytes).unwrap();
            assert!(
                input.starts_with(&format!(
                    "{method} /v1/agent-apps/codex/aggregation/codex/settings"
                ))
            );
            assert!(input.to_lowercase().contains("authorization: bearer fixture-t60"));
            captured.push(input);
            let body = serde_json::to_string(&snapshot()).unwrap();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(),body).unwrap();
        }
        captured
    });
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_auth(GatewayAuth { skey: Some("fixture-t60".into()) }),
    )
    .unwrap();
    let c = context();
    let mut form = Form { pending: Some(1), ..Default::default() };
    let state = execute(client.clone(), c.clone(), None).await.unwrap();
    assert!(form.finish(1, false, Ok(state)));
    form.select_model(None);
    let write = form.write(&c).unwrap();
    form.pending = Some(2);
    let state = execute(client, c, Some(write)).await.unwrap();
    assert!(form.finish(2, true, Ok(state)));
    assert!(form.saved);
    let captured = server.join().unwrap();
    assert!(captured[0].contains("targetHostId=host-t60"));
    assert!(captured[0].contains("projectId=project-t60"));
    let body: serde_json::Value =
        serde_json::from_str(captured[1].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(
        body,
        serde_json::json!({"targetHostId":"host-t60","projectId":"project-t60","expectedRevision":"r1","requested":{"subagentModelId":null,"subagentEffort":null,"autoReviewModelId":"review-keep","classicTools":false}})
    );
}

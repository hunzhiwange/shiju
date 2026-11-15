use super::subagent::*;
use vw_api_app_aggregation::*;
use vw_api_chat_identity::ReasoningEffortDto;
use gateway_app_settings::AgentAppsSettingsError;
fn loaded() -> Form {
    let mut state = CodexAggregationSettings {
        revision: "r61".into(),
        requested: Default::default(),
        effective: Default::default(),
        choices: vec![CodexSettingsChoice {
            model_id: "published".into(),
            label: "审批／日本語".into(),
            efforts: vec![],
        }],
        notices: vec![
            AggregationNotice::MultiAgentV2Forced,
            AggregationNotice::EncryptedTaskCrossProvider,
        ],
    };
    state.effective.classic_tools = false;
    Form { draft: state.requested.clone(), snapshot: Some(state), ..Default::default() }
}
fn wire_snapshot() -> CodexAggregationSettings {
    let requested = CodexSettingsValues {
        subagent_model_id: Some("published-a".into()),
        subagent_effort: Some(ReasoningEffortDto::High),
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
                efforts: vec![ReasoningEffortDto::Low, ReasoningEffortDto::High],
            },
            CodexSettingsChoice {
                model_id: "published-b".into(),
                label: "另一个模型".into(),
                efforts: vec![ReasoningEffortDto::Low],
            },
        ],
        notices: vec![AggregationNotice::NewSession],
    }
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
            host: "host-t61".into(),
            app: vw_api_app_identity::AppId::Codex,
        },
        project: "project-t61".into(),
        access: 4,
    }
}
#[test]
fn aggregation_t61_review_changes_preserve_subagent_and_server_constraints() {
    let mut form = loaded();
    assert!(form.draft.classic_tools);
    form.draft.subagent_model_id = Some("published".into());
    form.draft.subagent_effort = Some(ReasoningEffortDto::High);
    form.select_review(Some("published".into()));
    form.select_review(Some("unpublished".into()));
    assert_eq!(form.draft.auto_review_model_id.as_deref(), Some("published"));
    form.select_classic(false);
    assert!(!form.draft.classic_tools);
    form.select_classic(true);
    assert!(form.draft.classic_tools);
    assert!(
        !form.snapshot.as_ref().unwrap().effective.classic_tools,
        "draft cannot bypass server v2 override"
    );
    form.select_review(None);
    assert!(form.draft.auto_review_model_id.is_none());
    assert_eq!(form.draft.subagent_model_id.as_deref(), Some("published"));
    assert_eq!(form.draft.subagent_effort, Some(ReasoningEffortDto::High));
    form.snapshot = None;
    form.select_review(Some("published".into()));
    form.select_classic(false);
    assert!(form.draft.auto_review_model_id.is_none());
    assert!(form.draft.classic_tools);
}
#[test]
fn aggregation_t61_quota_failure_preserves_intent_requires_refresh() {
    let mut form = loaded();
    form.select_review(Some("published".into()));
    form.pending = Some(1);
    assert!(form.finish(
        1,
        true,
        Err(AgentAppsSettingsError::Http {
            status: 429,
            body: serde_json::json!({"token":"hidden"})
        })
    ));
    assert_eq!(form.draft.auto_review_model_id.as_deref(), Some("published"));
    assert!(form.snapshot.is_none());
    assert!(!form.saved);
    assert!(form.error.unwrap().contains("额度"));
    assert!(!form.error.unwrap().contains("hidden"));
}
#[tokio::test]
async fn aggregation_t61_form_calls_sdk_with_scope_revision_and_clear_values() {
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
            assert!(input.to_lowercase().contains("authorization: bearer fixture-t61"));
            captured.push(input);
            let body = serde_json::to_string(&wire_snapshot()).unwrap();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(),body).unwrap();
        }
        captured
    });
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_auth(GatewayAuth { skey: Some("fixture-t61".into()) }),
    )
    .unwrap();
    let c = context();
    let mut form = Form { pending: Some(1), ..Default::default() };
    let state = execute(client.clone(), c.clone(), None).await.unwrap();
    assert!(form.finish(1, false, Ok(state)));
    form.select_review(None);
    form.select_classic(false);
    let write = form.write(&c).unwrap();
    form.pending = Some(2);
    let state = execute(client, c, Some(write)).await.unwrap();
    assert!(form.finish(2, true, Ok(state)));
    assert!(form.saved);
    let captured = server.join().unwrap();
    assert!(captured[0].contains("targetHostId=host-t61"));
    assert!(captured[0].contains("projectId=project-t61"));
    let body: serde_json::Value =
        serde_json::from_str(captured[1].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(
        body,
        serde_json::json!({"targetHostId":"host-t61","projectId":"project-t61","expectedRevision":"r1","requested":{"subagentModelId":"published-a","subagentEffort":"high","autoReviewModelId":null,"classicTools":false}})
    );
}

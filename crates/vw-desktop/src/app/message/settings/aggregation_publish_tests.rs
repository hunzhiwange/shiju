use super::*;
fn preview() -> AggregationPreview {
    serde_json::from_value(serde_json::json!({
        "revision":"r1","previewToken":"opaque-preview","defaultProviderId":"one",
        "members":[{"providerId":"one","name":"默认中文供应商😀","models":[]},
        {"providerId":"two","name":"成员","models":[{"id":"stable-model","label":"中文模型","supports1m":true}]}],
        "issues":[],"notices":["model_reselect","restart_desktop"]
    })).unwrap()
}
#[test]
fn aggregation_t58_aggregation_publish_preserves_contract() {
    let mut session = Session::default();
    assert!(!session.can_confirm());
    session.preview = Some(preview());
    assert!(session.can_confirm(), "server preview must enable explicit confirmation");
}
fn server(
    responses: Vec<(u16, serde_json::Value)>,
) -> (GatewayClient, std::thread::JoinHandle<Vec<(String, serde_json::Value)>>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let mut requests = vec![];
        for (status, response) in responses {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && std::time::Instant::now() < deadline =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(5))
                    }
                    Err(e) => panic!("accept: {e}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
            let mut bytes = vec![];
            while !bytes.ends_with(b"\r\n\r\n") {
                let mut b = [0];
                stream.read_exact(&mut b).unwrap();
                bytes.push(b[0]);
            }
            let head = String::from_utf8(bytes).unwrap();
            let len = head
                .lines()
                .find_map(|l| {
                    l.to_lowercase()
                        .strip_prefix("content-length:")
                        .map(|n| n.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let mut body = vec![0; len];
            stream.read_exact(&mut body).unwrap();
            requests.push((head, serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null)));
            let body = response.to_string();
            write!(stream, "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
        requests
    });
    let endpoint = vw_gateway_client::GatewayEndpoint::new("127.0.0.1", port)
        .with_auth(vw_gateway_client::GatewayAuth { skey: Some("gateway-key".into()) });
    (GatewayClient::new(endpoint).unwrap(), handle)
}

fn selection(app: vw_gateway_client::vw_api_types::agent_apps::AppId) -> Selection {
    use crate::app::state::{AgentAppsContext, app_providers::list::Scope};
    Selection {
        scope: Scope {
            context: AgentAppsContext {
                gateway: "http://fixture".into(),
                tenant: Default::default(),
                project: None,
            },
            host: "host".into(),
            app,
        },
        project: "project".into(),
        catalog_generation: 1,
    }
}
fn progress(status: TaskStatus, cancelled: bool) -> ProviderOperationProgress {
    use vw_gateway_client::vw_api_types::app_providers::operations::ProviderOperationPhase;
    ProviderOperationProgress {
        operation_id: "operation".into(),
        request_id: "request".into(),
        phase: if matches!(
            status,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        ) {
            ProviderOperationPhase::Terminal
        } else {
            ProviderOperationPhase::Preparing
        },
        status,
        completed: 0,
        total: 3,
        cancellation_requested: cancelled,
        editor_outcome: None,
        aggregation_outcome: None,
    }
}
#[test]
fn aggregation_t58_confirmation_rejects_missing_blocked_and_duplicate_intents() {
    let mut session = Session { preview: Some(preview()), ..Session::default() };
    session.preview.as_mut().unwrap().issues.push(PreviewIssue {
        code: PreviewIssueCode::FailoverEnabled,
        blocking: true,
        provider_id: None,
        model_id: None,
    });
    assert!(!session.begin(Action::Confirm));
    session.preview.as_mut().unwrap().issues.clear();
    session.preview.as_mut().unwrap().preview_token = None;
    assert!(!session.begin(Action::Confirm));
    session.preview.as_mut().unwrap().preview_token = Some("token".into());
    session.preview.as_mut().unwrap().revision.clear();
    assert!(!session.begin(Action::Confirm));
    session.preview.as_mut().unwrap().revision = "r1".into();
    assert!(session.begin(Action::Confirm));
    assert!(!session.begin(Action::Confirm));
    assert!(!session.begin(Action::Preview));
    session.finish(Ok(Reply::Operation(progress(TaskStatus::Running, false))));
    assert!(!session.can_confirm());
    assert!(session.begin(Action::Cancel));
    session.finish(Ok(Reply::Operation(progress(TaskStatus::Running, true))));
    assert!(session.running(), "cancel request is not a terminal cancellation");
    assert!(!session.begin(Action::Cancel));
    session.finish(Ok(Reply::Operation(progress(TaskStatus::Cancelled, true))));
    assert!(!session.running());
}
#[tokio::test]
async fn aggregation_t58_sdk_preview_confirm_poll_cancel_preserve_scope() {
    use vw_gateway_client::vw_api_types::agent_apps::AppId;
    for app in [AppId::Claude, AppId::Codex, AppId::ClaudeDesktop] {
        let mut published = progress(TaskStatus::Completed, false);
        let state: AggregationState = serde_json::from_value(serde_json::json!({"revision":"r2","mode":"aggregation","attached":true,"defaultProviderId":"one","directProviderId":null,"members":[],"notices":[],"recovery":"none"})).unwrap();
        published.aggregation_outcome = Some(AggregationOutcome::Published { state });
        let (client, handle) = server(vec![
            (200, serde_json::to_value(preview()).unwrap()),
            (202, serde_json::to_value(progress(TaskStatus::Queued, false)).unwrap()),
            (200, serde_json::to_value(progress(TaskStatus::Running, false)).unwrap()),
            (200, serde_json::to_value(progress(TaskStatus::Running, true)).unwrap()),
            (200, serde_json::to_value(&published).unwrap()),
        ]);
        let mut session = Session::default();
        for action in [Action::Preview, Action::Confirm, Action::Poll, Action::Cancel, Action::Poll]
        {
            let intent = session.clone();
            assert!(session.begin(action));
            let result = execute(
                client.clone(),
                selection(app),
                "management-grant".into(),
                intent,
                action,
                Some("one".into()),
            )
            .await;
            assert!(result.is_ok(), "{result:?}");
            session.finish(result);
        }
        assert_eq!(session.operation.as_ref(), Some(&published));
        let requests = handle.join().unwrap();
        assert_eq!(requests.len(), 5);
        let app = serde_json::to_value(app).unwrap();
        assert!(requests[0].0.starts_with(&format!(
            "POST /v1/agent-apps/{}/aggregation/preview ",
            app.as_str().unwrap()
        )));
        assert_eq!(requests[0].1["catalogPolicy"], "preserve");
        assert_eq!(requests[0].1["defaultProviderId"], "one");
        assert!(requests[1].0.contains("/aggregation/publish "));
        assert_eq!(requests[1].1["expectedRevision"], "r1");
        assert_eq!(requests[1].1["previewToken"], "opaque-preview");
        assert_eq!(requests[1].1["confirmed"], true);
        assert!(requests[1].1["requestId"].as_str().unwrap().starts_with("aggregation-"));
        for (head, body) in requests.iter().take(2) {
            assert!(head.to_lowercase().contains("authorization: bearer management-grant"));
            assert_eq!(body["targetHostId"], "host");
            assert_eq!(body["projectId"], "project");
        }
        assert!(requests[2].0.starts_with("GET /v1/app-providers/operations/operation?"));
        assert!(requests[3].0.starts_with("DELETE /v1/app-providers/operations/operation?"));
        for (head, _) in requests.iter().skip(2) {
            assert!(head.contains("targetHostId=host"));
            assert!(head.contains("projectId=project"));
        }
    }
}
#[tokio::test]
async fn aggregation_t58_failed_submission_invalidates_preview_without_replay() {
    use vw_gateway_client::vw_api_types::agent_apps::AppId;
    for status in [403, 409, 422, 502, 503, 504] {
        let (client, handle) = server(vec![(
            status,
            serde_json::json!({"error":{"code":"target_failed","message":"sensitive fixture payload"}}),
        )]);
        let mut session = Session { preview: Some(preview()), ..Session::default() };
        let result = execute(
            client,
            selection(AppId::ClaudeDesktop),
            "management-grant".into(),
            session.clone(),
            Action::Confirm,
            None,
        )
        .await;
        assert!(
            matches!(result, Err(AggregationApiError::Http { status: observed, .. }) if observed == status)
        );
        session.finish(result);
        assert!(!session.can_confirm());
        assert!(session.preview.is_none());
        assert!(!session.error.unwrap().contains("sensitive"));
        assert_eq!(handle.join().unwrap().len(), 1);
    }
}
#[tokio::test]
async fn aggregation_t58_unconfirmed_execute_sends_no_publication() {
    use vw_gateway_client::vw_api_types::agent_apps::AppId;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let client = GatewayClient::new(vw_gateway_client::GatewayEndpoint::new(
        "127.0.0.1",
        listener.local_addr().unwrap().port(),
    ))
    .unwrap();
    let result = execute(
        client,
        selection(AppId::Claude),
        "grant".into(),
        Session::default(),
        Action::Confirm,
        None,
    )
    .await;
    assert!(result.is_err());
    assert!(listener.accept().is_err());
}

#[test]
fn aggregation_t58_stale_or_dismissed_results_do_not_restore_confirmation() {
    use vw_gateway_client::vw_api_types::agent_apps::AppId;
    let selected = selection(AppId::ClaudeDesktop);
    let mut form = super::super::Form::new(selected.clone(), 10);
    form.publish = Some(Session { busy: true, ..Session::default() });
    let mut other = selected.clone();
    other.project = "another".into();
    assert!(!form.finish_publication(&other, 10, Ok(Reply::Preview(preview()))));
    assert!(!form.finish_publication(&selected, 9, Ok(Reply::Preview(preview()))));
    assert!(form.finish_publication(&selected, 10, Ok(Reply::Preview(preview()))));
    assert!(!form.finish_publication(&selected, 10, Ok(Reply::Preview(preview()))));
    form.publish = None;
    assert!(!form.finish_publication(&selected, 10, Ok(Reply::Preview(preview()))));
}

#[test]
fn aggregation_t58_debug_redacts_preview_tokens_and_error_payloads() {
    let s = Session { preview: Some(preview()), ..Session::default() };
    assert!(!format!("{s:?}").contains("opaque-preview"));
    let event = Event::Finished {
        selection: selection(vw_gateway_client::vw_api_types::agent_apps::AppId::Claude),
        generation: 1,
        result: Ok(Reply::Preview(preview())),
    };
    assert!(!format!("{event:?}").contains("opaque-preview"));
    let event = Event::Finished {
        selection: selection(vw_gateway_client::vw_api_types::agent_apps::AppId::Claude),
        generation: 1,
        result: Err(AggregationApiError::Decode("sensitive".into())),
    };
    assert!(!format!("{event:?}").contains("sensitive"));
}

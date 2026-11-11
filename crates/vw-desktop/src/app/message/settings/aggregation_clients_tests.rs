use super::*;
fn state() -> CodexClientsState {
    CodexClientsState {
        revision: "clients-r1".into(),
        daemon: true,
        others: true,
        auth: true,
        unverified: true,
    }
}
fn ready() -> Session {
    Session { snapshot: Some(state()), ..Default::default() }
}
fn selection() -> Selection {
    use crate::app::state::{AgentAppsContext, app_providers::list::Scope};
    Selection {
        scope: Scope {
            context: AgentAppsContext {
                gateway: "http://fixture".into(),
                tenant: Default::default(),
                project: None,
            },
            host: "host-t62".into(),
            app: AppId::Codex,
        },
        project: "project-t62".into(),
        catalog_generation: 1,
    }
}
fn operation(status: TaskStatus, outcome: Option<AggregationOutcome>) -> ProviderOperationProgress {
    serde_json::from_value(serde_json::json!({"operationId":"op-t62","requestId":"request-t62","phase":"terminal","status":status,"completed":0,"total":0,"cancellationRequested":false,"aggregationOutcome":outcome})).unwrap()
}
#[test]
fn aggregation_t62_ack_does_not_restart_or_clear_unknown_and_stale() {
    let mut s = ready();
    assert!(!s.begin(Action::Restart), "opening confirmation is mandatory");
    assert!(s.begin(Action::Ack));
    assert!(!s.begin(Action::Ack));
    s.finish(Action::Ack, Ok(Reply::State(state())));
    assert!(s.acknowledged);
    assert_eq!(s.snapshot.as_ref().unwrap(), &state());
    assert!(s.operation.is_none());
    assert!(s.result_label().is_none());
    assert!(!s.begin(Action::Ack));
    assert!(s.begin(Action::Read));
    assert!(!s.acknowledged);
    s.finish(
        Action::Read,
        Ok(Reply::State(CodexClientsState { revision: "clients-r2".into(), ..state() })),
    );
    assert!(s.begin(Action::Ack));
    s.finish(
        Action::Ack,
        Ok(Reply::State(CodexClientsState { revision: "clients-r3".into(), ..state() })),
    );
    assert!(!s.acknowledged, "changed revision must require a new acknowledgement");
    assert!(s.begin(Action::Ack));
}
#[test]
fn aggregation_t62_restart_acceptance_cancellation_and_terminal_are_distinct() {
    let mut s = ready();
    s.confirming = true;
    assert!(s.begin(Action::Restart));
    s.finish(Action::Restart, Ok(Reply::Operation(operation(TaskStatus::Running, None))));
    assert!(s.running());
    assert!(!s.begin(Action::Read));
    assert!(!s.begin(Action::Restart));
    assert!(s.result_label().unwrap().contains("尚未完成"));
    assert!(s.begin(Action::Cancel));
    let mut p = operation(TaskStatus::Running, None);
    p.cancellation_requested = true;
    s.finish(Action::Cancel, Ok(Reply::Operation(p)));
    assert!(s.running());
    assert!(!s.begin(Action::Cancel));
    assert!(s.begin(Action::Poll));
    s.finish(
        Action::Poll,
        Ok(Reply::Operation(operation(
            TaskStatus::Completed,
            Some(AggregationOutcome::Restarted { result: CodexRestartResult::NotRunning }),
        ))),
    );
    assert!(s.result_label().unwrap().contains("未执行重启"));
    for (status, outcome, ok) in [
        (
            TaskStatus::Completed,
            Some(AggregationOutcome::Restarted { result: CodexRestartResult::Restarted }),
            true,
        ),
        (
            TaskStatus::Failed,
            Some(AggregationOutcome::Restarted { result: CodexRestartResult::Restarted }),
            false,
        ),
        (TaskStatus::Completed, None, false),
        (
            TaskStatus::Completed,
            Some(AggregationOutcome::Failed {
                status: 502,
                code: AggregationErrorCode::TargetFailed,
                recovery: RecoveryState::None,
            }),
            false,
        ),
    ] {
        let mut s = ready();
        s.finish(Action::Poll, Ok(Reply::Operation(operation(status, outcome))));
        assert_eq!(s.result_label().is_some_and(|l| l.starts_with("托管守护进程已重启")), ok);
    }
}
#[test]
fn aggregation_t62_errors_and_stale_responses_cannot_reenable_restart() {
    let selection = selection();
    let mut f = super::super::Form::new(selection.clone(), 9);
    f.clients = Some(Session { pending: true, ..ready() });
    assert!(!f.finish_clients(&selection, 8, Action::Ack, Ok(Reply::State(state()))));
    let mut other = selection.clone();
    other.catalog_generation += 1;
    assert!(!f.finish_clients(&other, 9, Action::Ack, Ok(Reply::State(state()))));
    for status in [401, 403, 409, 422, 501, 502, 503, 504] {
        let mut s = ready();
        s.confirming = true;
        assert!(s.begin(Action::Restart));
        s.finish(
            Action::Restart,
            Err(AgentAppsSettingsError::Http {
                status,
                body: serde_json::json!({"secret":"never-display"}),
            }),
        );
        assert!(!s.can_restart());
        assert!(s.snapshot.is_none());
        assert!(!s.error.unwrap().contains("never-display"));
    }
    assert!(f.finish_clients(&selection, 9, Action::Ack, Ok(Reply::State(state()))));
    assert!(!f.finish_clients(&selection, 9, Action::Ack, Ok(Reply::State(state()))));
    for s in [
        CodexClientsState { daemon: false, ..state() },
        CodexClientsState { revision: String::new(), ..state() },
    ] {
        assert!(!Session { snapshot: Some(s), ..Default::default() }.can_restart());
    }
}
// Local HTTP substitutes only the gateway boundary; the real SDK serializes every request.
fn server(
    responses: Vec<(u16, serde_json::Value)>,
) -> (GatewayClient, std::thread::JoinHandle<Vec<(String, serde_json::Value)>>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let mut inputs = vec![];
        for (status, response) in responses {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
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
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut head = vec![];
            while !head.ends_with(b"\r\n\r\n") {
                let mut b = [0];
                stream.read_exact(&mut b).unwrap();
                head.push(b[0]);
            }
            let head = String::from_utf8(head).unwrap();
            let len = head
                .lines()
                .find_map(|s| {
                    s.to_lowercase()
                        .strip_prefix("content-length:")
                        .map(|n| n.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let mut body = vec![0; len];
            stream.read_exact(&mut body).unwrap();
            inputs.push((head, serde_json::from_slice(&body).unwrap_or_default()));
            let body = response.to_string();
            write!(stream,"HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
        inputs
    });
    (
        GatewayClient::new(vw_gateway_client::GatewayEndpoint::new("127.0.0.1", port)).unwrap(),
        handle,
    )
}
#[tokio::test]
async fn aggregation_t62_sdk_ack_restart_and_progress_use_exact_scope() {
    let running = operation(TaskStatus::Running, None);
    let done = operation(
        TaskStatus::Completed,
        Some(AggregationOutcome::Restarted { result: CodexRestartResult::Restarted }),
    );
    let (client, handle) = server(vec![
        (200, serde_json::to_value(state()).unwrap()),
        (200, serde_json::to_value(state()).unwrap()),
        (202, serde_json::to_value(&running).unwrap()),
        (200, serde_json::to_value(&running).unwrap()),
        (200, serde_json::to_value(done).unwrap()),
    ]);
    let mut s = Session::default();
    for action in [Action::Read, Action::Ack, Action::Restart, Action::Cancel, Action::Poll] {
        if matches!(action, Action::Restart) {
            s.confirming = true;
        }
        let reply = execute(client.clone(), selection(), "fixture-t62".into(), s.clone(), action)
            .await
            .unwrap();
        assert!(s.begin(action));
        s.finish(action, Ok(reply));
    }
    assert!(s.result_label().unwrap().starts_with("托管守护进程已重启"));
    let requests = handle.join().unwrap();
    assert_eq!(requests.len(), 5);
    for (head, _) in &requests {
        assert!(head.to_lowercase().contains("authorization: bearer fixture-t62"));
    }
    assert!(requests[0].0.starts_with("GET /v1/agent-apps/codex/aggregation/codex/clients?"));
    assert!(requests[0].0.contains("targetHostId=host-t62"));
    assert!(requests[0].0.contains("projectId=project-t62"));
    assert!(requests[1].0.starts_with("POST /v1/agent-apps/codex/aggregation/codex/ack "));
    assert_eq!(
        requests[1].1,
        serde_json::json!({"targetHostId":"host-t62","projectId":"project-t62","expectedRevision":"clients-r1"})
    );
    assert!(requests[2].0.starts_with("POST /v1/agent-apps/codex/aggregation/codex/restart "));
    let body = &requests[2].1;
    assert_eq!(body.as_object().unwrap().len(), 5);
    assert_eq!(body["confirmed"], true);
    assert_eq!(body["expectedRevision"], "clients-r1");
    assert_eq!(body["targetHostId"], "host-t62");
    assert_eq!(body["projectId"], "project-t62");
    assert!(body["requestId"].as_str().unwrap().starts_with("codex-restart-"));
    assert!(requests[3].0.starts_with("DELETE /v1/app-providers/operations/op-t62?"));
    assert!(requests[4].0.starts_with("GET /v1/app-providers/operations/op-t62?"));
    for (head, _) in &requests[3..] {
        assert!(head.contains("targetHostId=host-t62"));
        assert!(head.contains("projectId=project-t62"));
    }
}
#[tokio::test]
async fn aggregation_t62_sdk_rejects_unconfirmed_and_foreign_intent_without_request() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let client = GatewayClient::new(vw_gateway_client::GatewayEndpoint::new(
        "127.0.0.1",
        listener.local_addr().unwrap().port(),
    ))
    .unwrap();
    for s in [Session::default(), ready()] {
        assert!(
            tokio::time::timeout(
                std::time::Duration::from_secs(1),
                execute(client.clone(), selection(), "fixture".into(), s, Action::Restart)
            )
            .await
            .expect("invalid intent must not perform I/O")
            .is_err()
        );
    }
    let mut c = selection();
    c.scope.app = AppId::Claude;
    assert!(execute(client, c, "fixture".into(), Session::default(), Action::Read).await.is_err());
    assert!(
        matches!(listener.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock),
        "no request may reach host for invalid intent"
    );
}

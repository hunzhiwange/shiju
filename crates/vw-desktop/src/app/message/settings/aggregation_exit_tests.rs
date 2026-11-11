use super::*;
#[test]
fn aggregation_t59_aggregation_exit_preserves_contract() {
    let mut session = Session::default();
    let operation = serde_json::from_value(serde_json::json!({
        "operationId":"exit-op", "requestId":"request", "status":"completed",
        "phase":"terminal", "completed":1,"total":1,"cancellationRequested":false,
        "aggregationOutcome":{"kind":"exited","state":{
            "revision":"r2","mode":"direct","attached":false,"defaultProviderId":"one",
            "directProviderId":"one","members":[],"notices":[],"recovery":"failed"
        }}
    }))
    .unwrap();
    session.finish(Ok(Reply::Operation(operation)));
    assert!(session.error.is_some(), "failed restoration must never be presented as success");
}

use super::tests::{progress, selection, server};
use vw_gateway_client::vw_api_types::agent_apps::AppId;
fn state() -> AggregationState {
    serde_json::from_value(serde_json::json!({
        "revision":"saved-r1","mode":"aggregation","attached":true,"defaultProviderId":"one",
        "directProviderId":"saved-direct","members":[{"providerId":"one","name":"中文成员😀","models":[]}],
        "notices":["model_reselect","restart_desktop"],"recovery":"none"
    })).unwrap()
}
fn exit_session() -> Session {
    Session { exit_state: Some(state()), ..Session::default() }
}
#[tokio::test]
async fn aggregation_t59_exit_sdk_preserves_scope_revision_and_returned_members() {
    for app in [AppId::Claude, AppId::Codex, AppId::ClaudeDesktop] {
        let mut restored = state();
        restored.mode = AggregationMode::Direct;
        restored.attached = false;
        restored.recovery = RecoveryState::Restored;
        let mut done = progress(TaskStatus::Completed, false);
        done.aggregation_outcome = Some(AggregationOutcome::Exited { state: restored.clone() });
        let (client, handle) = server(vec![
            (202, serde_json::to_value(progress(TaskStatus::Queued, false)).unwrap()),
            (200, serde_json::to_value(progress(TaskStatus::Running, false)).unwrap()),
            (200, serde_json::to_value(progress(TaskStatus::Running, true)).unwrap()),
            (200, serde_json::to_value(&done).unwrap()),
        ]);
        let mut session = exit_session();
        for action in [Action::Confirm, Action::Poll, Action::Cancel, Action::Poll] {
            let intent = session.clone();
            assert!(session.begin(action));
            let result = execute(
                client.clone(),
                selection(app),
                "management-grant".into(),
                intent,
                action,
                None,
            )
            .await;
            assert!(result.is_ok(), "{result:?}");
            session.finish(result);
        }
        assert_eq!(session.operation.as_ref(), Some(&done));
        assert!(session.error.is_none());
        assert_eq!(session.exit_state.as_ref().unwrap().members, restored.members);
        assert!(!session.can_confirm());
        let requests = handle.join().unwrap();
        assert_eq!(requests.len(), 4);
        let app = serde_json::to_value(app).unwrap();
        assert!(requests[0].0.starts_with(&format!(
            "POST /v1/agent-apps/{}/aggregation/exit ",
            app.as_str().unwrap()
        )));
        let body = &requests[0].1;
        assert_eq!(body["targetHostId"], "host");
        assert_eq!(body["projectId"], "project");
        assert_eq!(body["expectedRevision"], "saved-r1");
        assert_eq!(body["confirmed"], true);
        assert!(body["requestId"].as_str().unwrap().starts_with("aggregation-exit-"));
        assert_eq!(body.as_object().unwrap().len(), 5);
        assert!(requests[1].0.starts_with("GET /v1/app-providers/operations/operation?"));
        assert!(requests[2].0.starts_with("DELETE /v1/app-providers/operations/operation?"));
        for (head, _) in &requests {
            assert!(head.to_lowercase().contains("authorization: bearer management-grant"));
        }
        for (head, _) in requests.iter().skip(1) {
            assert!(head.contains("targetHostId=host") && head.contains("projectId=project"));
        }
    }
}
#[tokio::test]
async fn aggregation_t59_exit_errors_require_refresh_and_never_replay() {
    for status in [403, 409, 422, 501, 502, 503, 504] {
        let (client, handle) =
            server(vec![(status, serde_json::json!({"error":{"message":"sensitive payload"}}))]);
        let mut session = exit_session();
        let result = execute(
            client,
            selection(AppId::ClaudeDesktop),
            "grant".into(),
            session.clone(),
            Action::Confirm,
            None,
        )
        .await;
        session.finish(result);
        assert!(session.error.is_some());
        assert!(!session.error.unwrap().contains("sensitive"));
        assert!(!session.begin(Action::Confirm));
        assert_eq!(session.exit_state.as_ref().unwrap().members.len(), 1);
        assert_eq!(handle.join().unwrap().len(), 1);
    }
}
#[test]
fn aggregation_t59_exit_rejects_duplicate_stale_and_inconsistent_results() {
    let mut session = exit_session();
    assert!(session.begin(Action::Confirm));
    assert!(!session.begin(Action::Confirm));
    session.finish(Ok(Reply::Operation(progress(TaskStatus::Running, true))));
    assert!(session.running());
    assert!(!session.begin(Action::Cancel));
    for recovery in [RecoveryState::Pending, RecoveryState::Failed] {
        let mut operation = progress(TaskStatus::Completed, false);
        let mut state = state();
        state.recovery = recovery;
        operation.aggregation_outcome = Some(AggregationOutcome::Exited { state });
        session.finish(Ok(Reply::Operation(operation)));
        assert!(session.error.is_some());
    }
    let mut operation = progress(TaskStatus::Completed, false);
    operation.aggregation_outcome = Some(AggregationOutcome::Published { state: state() });
    session.finish(Ok(Reply::Operation(operation)));
    assert!(session.error.is_some());
    let selected = selection(AppId::Codex);
    let mut form = super::super::Form::new(selected.clone(), 5);
    form.publish = Some(Session { busy: true, ..exit_session() });
    let mut other = selected.clone();
    other.project = "another".into();
    assert!(!form.finish_publication(
        &other,
        5,
        Ok(Reply::Operation(progress(TaskStatus::Running, false)))
    ));
    assert!(!form.finish_publication(
        &selected,
        4,
        Ok(Reply::Operation(progress(TaskStatus::Running, false)))
    ));
    assert!(form.finish_publication(
        &selected,
        5,
        Ok(Reply::Operation(progress(TaskStatus::Running, false)))
    ));
    assert!(!form.finish_publication(
        &selected,
        5,
        Ok(Reply::Operation(progress(TaskStatus::Running, false)))
    ));
    form.invalidate_selection(&other);
    assert!(form.publish.is_none());
}
#[tokio::test]
async fn aggregation_t59_exit_without_revision_sends_no_write() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let client = GatewayClient::new(vw_gateway_client::GatewayEndpoint::new(
        "127.0.0.1",
        listener.local_addr().unwrap().port(),
    ))
    .unwrap();
    let mut session = exit_session();
    session.exit_state.as_mut().unwrap().revision.clear();
    assert!(!session.begin(Action::Confirm));
    assert!(
        execute(client, selection(AppId::Claude), "grant".into(), session, Action::Confirm, None)
            .await
            .is_err()
    );
    assert!(listener.accept().is_err());
}

use super::*;
use super::super::{ChatSessionMeta, HostMessage, TuiState};
use cli_gateway_transport::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

fn pending() -> (ActiveRunner, V3GatewayApp) {
    let runtime = GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 1)).unwrap(),
        GatewaySessionSeed::new(std::path::PathBuf::from("/workspace")),
    );
    let mut app = V3GatewayApp::new(runtime, TuiState::default());
    let mut runner = ActiveRunner::new();
    runner.session_picker_generation = 1;
    runner.session_picker_in_flight = Some(1);
    app.widget_mut().show_selection_overlay(
        super::super::sessions::session_picker_loading_surface(1), "正在加载", vec![],
    );
    (runner, app)
}

fn complete(runner: &mut ActiveRunner, app: &mut V3GatewayApp, generation: u64,
    directory: &str, result: Result<Vec<ChatSessionMeta>, String>) {
    runner.host_sender.send(HostMessage::SessionCandidatesFinished {
        generation, directory: directory.into(), session_id: None, result,
    }).unwrap();
    runner.drain_host_messages(app);
}

#[test]
fn workflow_req_05_empty_result_remains_a_successful_list() {
    let (mut runner, mut app) = pending();
    complete(&mut runner, &mut app, 1, "/workspace", Ok(vec![]));
    assert_eq!(app.widget().selection_surface(), Some(super::super::SESSION_RESUME_SURFACE));
    assert!(runner.session_picker_in_flight.is_none());
}

#[test]
fn workflow_req_05_failure_exposes_a_real_retry_command() {
    let (mut runner, mut app) = pending();
    complete(&mut runner, &mut app, 1, "/workspace", Err("网络失败🙂".into()));
    assert_eq!(app.widget().selection_surface(), Some("session_resume_retry"));
    assert!(matches!(crate::commands::route_selection("session_resume_retry", "retry"),
        Ok(crate::commands::CommandRoute::OpenSessionPicker)));
}

#[test]
fn workflow_req_05_closed_picker_is_not_reopened() {
    let (mut runner, mut app) = pending();
    app.widget_mut().clear_overlay();
    complete(&mut runner, &mut app, 1, "/workspace", Ok(vec![]));
    assert_eq!(app.widget().selection_surface(), None);
    assert!(runner.session_picker_in_flight.is_none());
}

#[test]
fn workflow_req_05_previous_generation_cannot_replace_new_overlay() {
    let (mut runner, mut app) = pending();
    runner.session_picker_generation = 2;
    complete(&mut runner, &mut app, 1, "/workspace", Err("旧请求".into()));
    assert_eq!(app.widget().selection_surface(), Some("session_resume_loading:1"));
}

#[test]
fn workflow_req_05_foreign_directory_is_rejected() {
    let (mut runner, mut app) = pending();
    complete(&mut runner, &mut app, 1, "/其它目录🙂", Ok(vec![]));
    assert_eq!(app.widget().selection_surface(), Some("session_resume_loading:1"));
}

#[test]
fn workflow_req_05_foreign_session_is_rejected() {
    let (mut runner, mut app) = pending();
    runner.host_sender.send(HostMessage::SessionCandidatesFinished {
        generation: 1, directory: "/workspace".into(), session_id: Some("other".into()),
        result: Ok(vec![]),
    }).unwrap();
    runner.drain_host_messages(&mut app);
    assert_eq!(app.widget().selection_surface(), Some("session_resume_loading:1"));
}

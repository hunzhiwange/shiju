use super::app::{ActiveRunner, V3GatewayApp};
use cli_gateway_transport::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use cli_interaction_state::TuiState;
use std::sync::Arc;
use std::time::{Duration, Instant};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use vw_tui::{V3Action, V3Attachment, V3AttachmentKind};

fn app() -> V3GatewayApp {
    let runtime = GatewayUiRuntime::new(
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 9)).unwrap(),
        GatewaySessionSeed::new(std::env::temp_dir()),
    );
    let mut app = V3GatewayApp::new(runtime, TuiState::default());
    app.widget_mut().replace_draft("草稿 🦀");
    app.widget_mut().add_attachment(V3Attachment {
        kind: V3AttachmentKind::Image,
        label: "image.png".into(),
        path: None,
        media_type: Some("image/png".into()),
        text: None,
        bytes: Some(vec![1, 2]),
    });
    app
}

fn finish(runner: &mut ActiveRunner, app: &mut V3GatewayApp) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while runner.editor_in_flight() {
        assert!(Instant::now() < deadline, "editor did not complete");
        runner.drain(app);
        std::thread::yield_now();
    }
}

fn check(result: Result<Option<String>, String>, expected: &str) {
    let mut app = app();
    let attachments = app.widget().attachments().to_vec();
    let mut runner = ActiveRunner::new_with_editor(Arc::new(move |draft| {
        assert_eq!(draft, "草稿 🦀");
        result.clone()
    }));
    runner.execute_action(&mut app, V3Action::OpenExternalEditor);
    finish(&mut runner, &mut app);
    assert_eq!(app.widget().draft(), expected);
    assert_eq!(app.widget().attachments(), attachments);
}

#[test]
fn terminal_req_07_success_preserves_attachments() {
    check(Ok(Some("修改 ✨".into())), "修改 ✨");
}
#[test]
fn terminal_req_07_cancel_preserves_draft() {
    check(Ok(None), "草稿 🦀");
}
#[test]
fn terminal_req_07_failure_preserves_draft() {
    check(Err("launch failed".into()), "草稿 🦀");
}
#[test]
fn terminal_req_07_empty_success_replaces_draft() {
    check(Ok(Some(String::new())), "");
}
#[test]
fn terminal_req_07_worker_panic_completes() {
    let mut app = app();
    let mut runner = ActiveRunner::new_with_editor(Arc::new(|_| panic!("editor failure")));
    runner.execute_action(&mut app, V3Action::OpenExternalEditor);
    finish(&mut runner, &mut app);
    assert_eq!(app.widget().draft(), "草稿 🦀");
    assert_eq!(app.widget().attachments().len(), 1);
}
#[test]
fn terminal_req_07_duplicate_launch_is_ignored_and_can_restart() {
    let mut app = app();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = calls.clone();
    let mut runner = ActiveRunner::new_with_editor(Arc::new(move |_| {
        observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(None)
    }));
    runner.execute_action(&mut app, V3Action::OpenExternalEditor);
    runner.execute_action(&mut app, V3Action::OpenExternalEditor);
    finish(&mut runner, &mut app);
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    runner.execute_action(&mut app, V3Action::OpenExternalEditor);
    finish(&mut runner, &mut app);
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
}

//! 验证提示运行时的工具生命周期事件与幂等取消行为。
//! 伪运行器记录类型化工具事件，并用重复取消验证终止句柄的幂等性。

use std::time::Duration;

use serde_json::json;

use super::prompt::start_prompt_with_runner;
use super::{PromptEvent, PromptTermination, types::PromptCancelHandle};
use crate::session::processor::{Request, StreamEvent, ToolCallStatus, ToolEvent, ToolEventKind};
use crate::session::ui_types::{ChatMessage, TokenUsage};

fn request() -> Request {
    Request {
        stream: 1,
        session: "session-1".to_string(),
        query: "hello".to_string(),
        root: None,
        model: None,
        reliability: crate::app::agent::config::ReliabilityConfig::default(),
        options: json!({}),
        approval: None,
        channel_name: None,
        non_cli_approval_context: None,
        assistant_message_id: None,
        history: Vec::<ChatMessage>::new(),
        persist_app_session_artifacts: false,
        app_session_scope: None,
        abort_token: None,
        session_tools: super::SessionToolset::default(),
    }
}

#[test]
fn prompt_runtime_emits_typed_tool_lifecycle() {
    let call_id = "call-1";
    let turn = start_prompt_with_runner(request(), move |_request, mut emit| {
        emit(StreamEvent::Delta("hello".to_string()));
        for (kind, status, payload) in [
            (
                ToolEventKind::Start,
                ToolCallStatus::Running,
                json!({"status": "running", "input": "{}"}),
            ),
            (
                ToolEventKind::Update,
                ToolCallStatus::Running,
                json!({"status": "running", "output": "half"}),
            ),
            (
                ToolEventKind::Complete,
                ToolCallStatus::Completed,
                json!({"status": "completed", "output": "done"}),
            ),
        ] {
            emit(StreamEvent::Tool(ToolEvent {
                call_id: call_id.to_string(),
                name: "shell".to_string(),
                kind,
                status,
                occurred_ms: 1,
                payload,
            }));
        }
        emit(StreamEvent::Done(TokenUsage::default()));
    });

    let mut events = Vec::new();
    while let Ok(event) = turn.events.recv_timeout(Duration::from_secs(1)) {
        let terminal = matches!(event, PromptEvent::Terminated(_));
        events.push(event);
        if terminal {
            break;
        }
    }
    assert!(matches!(&events[0], PromptEvent::Text(text) if text == "hello"));
    assert!(matches!(
        &events[1],
        PromptEvent::Tool(event)
            if event.call_id == call_id && event.kind == ToolEventKind::Start
    ));
    assert!(matches!(
        &events[2],
        PromptEvent::Tool(event)
            if event.call_id == call_id && event.kind == ToolEventKind::Update
    ));
    assert!(matches!(
        &events[3],
        PromptEvent::Tool(event)
            if event.call_id == call_id
                && event.kind == ToolEventKind::Complete
                && event.status == ToolCallStatus::Completed
    ));
    assert!(matches!(
        events.last(),
        Some(PromptEvent::Terminated(PromptTermination::Completed(_)))
    ));
}

#[test]
// 重复取消只改变一次终止状态，不能重复发送完成事件或遗留运行句柄。
fn prompt_runtime_cancel_is_idempotent() {
    let turn = start_prompt_with_runner(request(), |request, mut emit| {
        let abort = request.abort_token.expect("runtime cancellation token");
        while !abort.is_cancelled() {
            std::thread::yield_now();
        }
        emit(StreamEvent::Delta("must not escape after cancellation".to_string()));
    });
    let cancel: PromptCancelHandle = turn.cancel.clone();

    assert!(cancel.cancel());
    assert!(!cancel.cancel());
    assert!(cancel.is_cancelled());

    let event = turn.events.recv_timeout(Duration::from_secs(1)).expect("cancel terminal");
    assert!(matches!(event, PromptEvent::Terminated(PromptTermination::Cancelled)));
    assert!(turn.events.recv_timeout(Duration::from_millis(50)).is_err());
}

#[cfg(unix)]
fn shell_turn_fixture(
    session: &str,
) -> (tempfile::TempDir, Request, crate::tools::traits::ToolCallResult) {
    use crate::tools::{ToolUseContext, traits::Tool};
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    let security = Arc::new(crate::security::SecurityPolicy {
        autonomy: crate::security::AutonomyLevel::Full,
        workspace_only: false,
        workspace_dir: dir.path().to_path_buf(),
        allowed_commands: vec!["printf".into(), "sleep".into()],
        forbidden_paths: vec![],
        ..Default::default()
    });
    let tools: Vec<Box<dyn Tool>> = crate::tools::default_tools_with_runtime(
        security,
        Arc::new(crate::runtime::NativeRuntime::new()),
    );
    let context = Arc::new(ToolUseContext::new(session, Some(dir.path().to_string_lossy().into())));
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut result = runtime
        .block_on(crate::tools::execute_tool_from_registry(
            &tools,
            "bash",
            json!({"command":"printf ready; sleep 60", "yield_time_ms":250}),
            context.clone(),
        ))
        .unwrap()
        .result;
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while result.data["state"] == "running"
        && result.data["output"] != "ready"
        && std::time::Instant::now() < deadline
    {
        result = runtime
            .block_on(crate::tools::execute_tool_from_registry(
                &tools,
                "bash_wait",
                json!({"process_id":result.data["process_id"], "cursor":0, "yield_time_ms":250}),
                context.clone(),
            ))
            .unwrap()
            .result;
    }
    if result.data["state"] != "running" || result.data["output"] != "ready" {
        runtime
            .block_on(crate::tools::execute_tool_from_registry(
                &tools,
                "bash_cancel",
                json!({"process_id":result.data["process_id"]}),
                context,
            ))
            .unwrap();
        panic!("real process readiness handshake failed");
    }
    let mut req = request();
    req.session = session.into();
    req.root = Some(dir.path().to_string_lossy().into());
    (dir, req, result)
}

#[cfg(unix)]
#[test]
fn shell_session_stop_cancels_owned_processes() {
    use crate::tools::shell::processes::{ShellProcessScope, ShellProcessState, ShellProcesses};
    let (dir, req, result) = shell_turn_fixture("stop-owned");
    let (_foreign_dir, foreign_req, foreign) = shell_turn_fixture("stop-foreign");
    let tenant = serde_json::to_string(&crate::session::ui_store::current_tenant_scope()).unwrap();
    let scope = ShellProcessScope::new(tenant.clone(), dir.path(), req.session.clone()).unwrap();
    let foreign_scope = ShellProcessScope::new(
        tenant,
        std::path::Path::new(foreign_req.root.as_ref().unwrap()),
        foreign_req.session.clone(),
    )
    .unwrap();
    let turn = start_prompt_with_runner(req, |_, mut emit| {
        emit(StreamEvent::Done(TokenUsage::default()));
    });
    assert!(matches!(
        turn.events.recv_timeout(Duration::from_secs(5)).unwrap(),
        PromptEvent::Terminated(PromptTermination::Completed(_))
    ));
    turn.cancel.cancel();
    let host = ShellProcesses::shared().unwrap();
    let state = host.snapshot(&scope, result.data["process_id"].as_str().unwrap()).unwrap().state;
    let foreign_state =
        host.snapshot(&foreign_scope, foreign.data["process_id"].as_str().unwrap()).unwrap().state;
    // Always clean up even when the assertion below fails in the red phase.
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(host.cancel(&scope, result.data["process_id"].as_str().unwrap())).unwrap();
    runtime
        .block_on(host.cancel(&foreign_scope, foreign.data["process_id"].as_str().unwrap()))
        .unwrap();
    assert_eq!(state, ShellProcessState::Cancelled);
    assert_eq!(foreign_state, ShellProcessState::Running);
    eprintln!("explicit stop: owned child reaped; foreign session preserved");
}

#[cfg(unix)]
#[test]
fn shell_session_normal_return_preserves_process() {
    use crate::tools::shell::processes::{ShellProcessScope, ShellProcessState, ShellProcesses};
    let (dir, req, result) = shell_turn_fixture("normal-return");
    let scope = ShellProcessScope::new(
        serde_json::to_string(&crate::session::ui_store::current_tenant_scope()).unwrap(),
        dir.path(),
        req.session.clone(),
    )
    .unwrap();
    let turn = start_prompt_with_runner(req, |_, mut emit| {
        emit(StreamEvent::Done(TokenUsage::default()));
    });
    assert!(matches!(
        turn.events.recv_timeout(Duration::from_secs(5)).unwrap(),
        PromptEvent::Terminated(PromptTermination::Completed(_))
    ));
    drop(turn);
    let host = ShellProcesses::shared().unwrap();
    let state = host.snapshot(&scope, result.data["process_id"].as_str().unwrap()).unwrap().state;
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(host.cancel(&scope, result.data["process_id"].as_str().unwrap())).unwrap();
    assert_eq!(state, ShellProcessState::Running);
    eprintln!("normal return and receiver drop preserve real running child");
}

#[cfg(unix)]
#[test]
fn shell_session_external_stop_reaps_returned_process() {
    use crate::tools::shell::processes::{ShellProcessScope, ShellProcessState, ShellProcesses};
    let (dir, mut req, result) = shell_turn_fixture("external-stop");
    let scope = ShellProcessScope::new(
        serde_json::to_string(&crate::session::ui_store::current_tenant_scope()).unwrap(),
        dir.path(),
        req.session.clone(),
    )
    .unwrap();
    let external = tokio_util::sync::CancellationToken::new();
    req.abort_token = Some(external.clone());
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let turn = start_prompt_with_runner(req, move |request, _emit| {
        ready_tx.send(()).unwrap();
        let token = request.abort_token.unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !token.is_cancelled() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    external.cancel();
    assert!(matches!(
        turn.events.recv_timeout(Duration::from_secs(10)).unwrap(),
        PromptEvent::Terminated(PromptTermination::Cancelled)
    ));
    assert_eq!(
        ShellProcesses::shared()
            .unwrap()
            .snapshot(&scope, result.data["process_id"].as_str().unwrap())
            .unwrap()
            .state,
        ShellProcessState::Cancelled
    );
}

#[cfg(unix)]
#[test]
fn shell_session_transport_interrupt_preserves_process() {
    use crate::tools::shell::processes::{ShellProcessScope, ShellProcessState, ShellProcesses};
    let (dir, req, result) = shell_turn_fixture("transport-interrupt");
    let scope = ShellProcessScope::new(
        serde_json::to_string(&crate::session::ui_store::current_tenant_scope()).unwrap(),
        dir.path(),
        req.session.clone(),
    )
    .unwrap();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let turn = start_prompt_with_runner(req, move |request, _emit| {
        ready_tx.send(()).unwrap();
        let token = request.abort_token.unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !token.is_cancelled() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        done_tx.send(()).unwrap();
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    turn.cancel.interrupt();
    done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let host = ShellProcesses::shared().unwrap();
    let state = host.snapshot(&scope, result.data["process_id"].as_str().unwrap()).unwrap().state;
    turn.cancel.cancel();
    assert_eq!(state, ShellProcessState::Running);
    assert_eq!(
        host.snapshot(&scope, result.data["process_id"].as_str().unwrap()).unwrap().state,
        ShellProcessState::Cancelled
    );
}

#[cfg(unix)]
#[test]
fn shell_session_completed_handle_does_not_cancel_later_prompt_process() {
    use crate::tools::shell::processes::{ShellProcessScope, ShellProcessState, ShellProcesses};
    let (dir, req, result) = shell_turn_fixture("later-prompt");
    let scope = ShellProcessScope::new(
        serde_json::to_string(&crate::session::ui_store::current_tenant_scope()).unwrap(),
        dir.path(),
        req.session.clone(),
    )
    .unwrap();
    let turn = start_prompt_with_runner(req, |_, mut emit| {
        emit(StreamEvent::Done(TokenUsage::default()));
    });
    assert!(matches!(
        turn.events.recv_timeout(Duration::from_secs(5)).unwrap(),
        PromptEvent::Terminated(PromptTermination::Completed(_))
    ));
    let host = ShellProcesses::shared().unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let later = runtime
        .block_on(host.start(scope.clone(), || {
            let mut command = tokio::process::Command::new("/bin/sh");
            command.arg("-c").arg("exec sleep 60");
            Ok(command)
        }))
        .unwrap();
    turn.cancel.cancel();
    let first_state =
        host.snapshot(&scope, result.data["process_id"].as_str().unwrap()).unwrap().state;
    let later_state = host.snapshot(&scope, &later.process_id).unwrap().state;
    runtime.block_on(host.cancel(&scope, &later.process_id)).unwrap();
    assert_eq!(first_state, ShellProcessState::Cancelled);
    assert_eq!(later_state, ShellProcessState::Running);
}

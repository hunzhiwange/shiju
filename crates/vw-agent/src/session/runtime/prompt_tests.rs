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

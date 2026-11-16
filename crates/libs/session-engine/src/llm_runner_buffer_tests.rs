//! Buffering and provider-event collection belong to the library runner.
use super::*;
use crate::processor_events::StreamEvent as CollectorStreamEvent;
use crate::llm_runner as runner;
use crate::message::AssistantError;
use session_port::ui_types as models;
use std::sync::mpsc;
use std::time::Duration;
mod llm {
    pub use llm_runtime::StreamEvent;
    pub use crate::processor_events::ToolCall;
}
#[path = "llm_runner_test_support.rs"]
mod llm_runner_test_support;
use llm_runner_test_support::{collect_events, observe_first_delta_before_stream_continues, usage};

#[test]
fn collect_stream_returns_text_tool_calls_usage_and_full_messages() {
    let tool_call = llm::ToolCall {
        kind: Default::default(),
        namespace: None,
        id: "call-1".to_string(),
        name: "file_read".to_string(),
        arguments: r#"{"path":"README.md"}"#.to_string(),
    };
    let full_messages = vec![serde_json::json!({ "role": "system", "content": "base" })];

    let (result, deltas) = collect_events([
        llm::StreamEvent::Delta("hello ".to_string()),
        llm::StreamEvent::Delta("world".to_string()),
        llm::StreamEvent::ToolCalls(vec![tool_call.clone()]),
        llm::StreamEvent::FullMessages(full_messages.clone()),
        llm::StreamEvent::Done {
            usage: usage(11, 7),
            finish_reason: Some("tool_calls".to_string()),
        },
    ]);

    let step = result.expect("stream should produce a step");
    assert_eq!(step.text, "hello world");
    assert_eq!(step.tool_calls[0].id, tool_call.id);
    assert_eq!(step.full_messages, full_messages);
    assert_eq!(step.finish_reason.as_deref(), Some("tool_calls"));
    assert_eq!(step.usage.input_tokens, 11);
    assert_eq!(step.usage.output_tokens, 7);
    assert_eq!(deltas.join(""), "hello world");
}

#[test]
fn remote_tool_updates_are_forwarded_without_entering_answer_text() {
    let (tx, rx) = mpsc::channel();
    tx.send(llm::StreamEvent::RemoteToolUpdate {
        call_id: "remote-1".to_string(),
        name: "bash".to_string(),
        is_initial: true,
        payload: serde_json::json!({"status": "running", "input": "pwd"}),
    })
    .expect("remote tool event should be queued");
    tx.send(llm::StreamEvent::Delta("最终答复".to_string())).expect("answer should be queued");
    tx.send(llm::StreamEvent::Done {
        usage: models::TokenUsage::default(),
        finish_reason: Some("stop".to_string()),
    })
    .expect("done should be queued");
    drop(tx);

    let (abort_tx, _abort_rx) = tokio::sync::watch::channel(false);
    let mut forwarded = Vec::new();
    let step = super::collect_llm_step_from_stream(
        rx,
        &mut |event| {
            forwarded.push(event);
            true
        },
        &abort_tx,
        Duration::from_millis(5),
        None,
    )
    .expect("stream should produce a step");

    assert_eq!(step.text, "最终答复");
    assert!(matches!(
        &forwarded[0],
        super::StreamEvent::Tool(event)
            if event.call_id == "remote-1"
                && event.name == "bash"
                && event.kind == super::ToolEventKind::Start
                && event.status == super::ToolCallStatus::Running
    ));
    assert!(matches!(&forwarded[1], super::StreamEvent::Delta(text) if text == "最终答复"));
}

#[test]
fn llm_stream_emits_first_text_delta_immediately() {
    let first_delta = observe_first_delta_before_stream_continues(
        llm::StreamEvent::Delta("hello".to_string()),
        [llm::StreamEvent::Done {
            usage: models::TokenUsage::default(),
            finish_reason: Some("stop".to_string()),
        }],
    );

    assert_eq!(first_delta, "hello");
}

#[test]
fn llm_stream_emits_first_reasoning_delta_immediately() {
    let first_delta = observe_first_delta_before_stream_continues(
        llm::StreamEvent::ReasoningDelta("plan".to_string()),
        [
            llm::StreamEvent::Delta("answer".to_string()),
            llm::StreamEvent::Done {
                usage: models::TokenUsage::default(),
                finish_reason: Some("stop".to_string()),
            },
        ],
    );

    assert_eq!(first_delta, "<think>plan");
}

#[test]
fn collect_stream_wraps_reasoning_before_text() {
    let (result, deltas) = collect_events([
        llm::StreamEvent::ReasoningDelta("plan".to_string()),
        llm::StreamEvent::Delta("answer".to_string()),
        llm::StreamEvent::Done {
            usage: models::TokenUsage::default(),
            finish_reason: Some("stop".to_string()),
        },
    ]);

    let step = result.expect("reasoning plus text should be valid");
    assert_eq!(step.reasoning_content, "plan");
    assert_eq!(step.text, "answer");
    assert_eq!(deltas, vec!["<think>plan", "</think>\n\n", "answer"]);
}

#[test]
fn collect_stream_rejects_reasoning_without_assistant_content() {
    let (result, deltas) = collect_events([
        llm::StreamEvent::ReasoningDelta("only thoughts".to_string()),
        llm::StreamEvent::Done {
            usage: models::TokenUsage::default(),
            finish_reason: Some("stop".to_string()),
        },
    ]);

    assert_eq!(result.unwrap_err(), "模型未返回内容");
    assert_eq!(deltas, vec!["<think>only thoughts", "</think>\n\n"]);
}

#[test]
fn collect_stream_emits_late_reasoning_after_text() {
    let (result, deltas) = collect_events([
        llm::StreamEvent::Delta("answer".to_string()),
        llm::StreamEvent::ReasoningDelta("late plan".to_string()),
        llm::StreamEvent::Done {
            usage: models::TokenUsage::default(),
            finish_reason: Some("stop".to_string()),
        },
    ]);

    let step = result.expect("text should make the step valid");
    assert_eq!(step.text, "answer");
    assert_eq!(step.reasoning_content, "late plan");
    assert_eq!(deltas, vec!["answer", "\n\n<think>late plan</think>\n\n"]);
}

#[test]
fn collect_stream_emits_late_reasoning_without_repeating_early_reasoning() {
    let (result, deltas) = collect_events([
        llm::StreamEvent::ReasoningDelta("先检查。".to_string()),
        llm::StreamEvent::Delta("answer".to_string()),
        llm::StreamEvent::ReasoningDelta("late plan".to_string()),
        llm::StreamEvent::Done {
            usage: models::TokenUsage::default(),
            finish_reason: Some("stop".to_string()),
        },
    ]);

    let step = result.expect("text should make the step valid");
    assert_eq!(step.reasoning_content, "先检查。late plan");
    assert_eq!(
        deltas,
        vec!["<think>先检查。", "</think>\n\n", "answer", "\n\n<think>late plan</think>\n\n"]
    );
}

#[test]
fn collect_stream_rejects_done_without_text_or_tools() {
    let (result, deltas) = collect_events([llm::StreamEvent::Done {
        usage: models::TokenUsage::default(),
        finish_reason: Some("stop".to_string()),
    }]);

    assert_eq!(result.unwrap_err(), "模型未返回内容");
    assert!(deltas.is_empty());
}

#[test]
fn collect_stream_allows_tool_only_steps() {
    let (result, deltas) = collect_events([
        llm::StreamEvent::ToolCalls(vec![llm::ToolCall {
            kind: Default::default(),
            namespace: None,
            id: "call-1".to_string(),
            name: "TodoWrite".to_string(),
            arguments: "{}".to_string(),
        }]),
        llm::StreamEvent::Done {
            usage: models::TokenUsage::default(),
            finish_reason: Some("tool_calls".to_string()),
        },
    ]);

    let step = result.expect("tool-only response should be valid");
    assert!(step.text.is_empty());
    assert_eq!(step.tool_calls.len(), 1);
    assert!(deltas.is_empty());
}

#[test]
fn collect_stream_flushes_pending_content_on_error() {
    let (result, deltas) = collect_events([
        llm::StreamEvent::ReasoningDelta("before error".to_string()),
        llm::StreamEvent::Error(AssistantError::Unknown { message: "boom".to_string() }),
    ]);

    let message = result.unwrap_err();
    assert!(message.contains("boom"));
    assert_eq!(deltas, vec!["<think>before error", "</think>\n\n"]);
}

#[test]
fn collect_stream_flushes_text_by_character_threshold() {
    let (result, deltas) = collect_events([
        llm::StreamEvent::Delta("x".repeat(1024)),
        llm::StreamEvent::Delta("tail".to_string()),
        llm::StreamEvent::Done {
            usage: models::TokenUsage::default(),
            finish_reason: Some("stop".to_string()),
        },
    ]);

    let step = result.expect("long text should produce a valid step");
    assert_eq!(step.text.len(), 1028); // 1024 + 4
    // First flush at 1024 chars, remaining "tail" flushed at end
    assert_eq!(deltas.len(), 2);
    assert_eq!(deltas[0].len(), 1024);
    assert_eq!(deltas[1], "tail");
}

#[test]
fn collect_stream_flushes_reasoning_by_character_threshold() {
    let (result, deltas) = collect_events([
        llm::StreamEvent::ReasoningDelta("r".repeat(1024)),
        llm::StreamEvent::Done {
            usage: models::TokenUsage::default(),
            finish_reason: Some("stop".to_string()),
        },
    ]);

    assert_eq!(result.unwrap_err(), "模型未返回内容");
    // Mid-stream flush includes the <think> prefix
    assert_eq!(deltas.len(), 2);
    assert_eq!(deltas[0], format!("<think>{}", "r".repeat(1024)));
    assert_eq!(deltas[1], "</think>\n\n");
}

#[test]
fn collect_stream_overwrites_tool_calls_on_repeated_events() {
    let first = llm::ToolCall {
        kind: Default::default(),
        namespace: None,
        id: "call-1".to_string(),
        name: "read_file".to_string(),
        arguments: "{}".to_string(),
    };
    let second = llm::ToolCall {
        kind: Default::default(),
        namespace: None,
        id: "call-2".to_string(),
        name: "write_file".to_string(),
        arguments: r#"{"path":"/tmp/x"}"#.to_string(),
    };

    let (result, _deltas) = collect_events([
        llm::StreamEvent::ToolCalls(vec![first]),
        llm::StreamEvent::ToolCalls(vec![second.clone()]),
        llm::StreamEvent::Done {
            usage: models::TokenUsage::default(),
            finish_reason: Some("tool_calls".to_string()),
        },
    ]);

    let step = result.expect("repeated tool calls should still produce a step");
    assert_eq!(step.tool_calls.len(), 1);
    assert_eq!(step.tool_calls[0].id, "call-2");
    assert_eq!(step.tool_calls[0].name, "write_file");
}

#[test]
fn collect_stream_flushes_pending_text_and_reasoning_on_error() {
    let (result, deltas) = collect_events([
        llm::StreamEvent::ReasoningDelta("think ".to_string()),
        llm::StreamEvent::Delta("speak ".to_string()),
        llm::StreamEvent::Error(AssistantError::Unknown { message: "fail".to_string() }),
    ]);

    assert!(result.is_err());
    // pending reasoning flushed (as <think> block), then </think>, then pending text
    assert_eq!(deltas, vec!["<think>think ", "</think>\n\n", "speak "]);
}

#[test]
fn collect_stream_rejects_whitespace_only_text() {
    let (result, _deltas) = collect_events([
        llm::StreamEvent::Delta("   ".to_string()),
        llm::StreamEvent::Done {
            usage: models::TokenUsage::default(),
            finish_reason: Some("stop".to_string()),
        },
    ]);

    assert_eq!(result.unwrap_err(), "模型未返回内容");
}

#[test]
fn collect_stream_returns_buffered_text_when_sender_disconnects_without_done() {
    let (result, deltas) = collect_events([llm::StreamEvent::Delta("partial".to_string())]);

    let step = result.expect("buffered text should remain a valid step after disconnect");
    assert_eq!(step.text, "partial");
    assert_eq!(step.usage, models::TokenUsage::default());
    assert_eq!(step.finish_reason, None);
    assert_eq!(deltas, vec!["partial"]);
}

#[test]
fn collect_stream_keeps_latest_full_messages_event() {
    let latest = vec![serde_json::json!({ "role": "assistant", "content": "latest" })];
    let (result, _deltas) = collect_events([
        llm::StreamEvent::FullMessages(vec![serde_json::json!({
            "role": "assistant",
            "content": "stale"
        })]),
        llm::StreamEvent::FullMessages(latest.clone()),
        llm::StreamEvent::Delta("answer".to_string()),
        llm::StreamEvent::Done {
            usage: models::TokenUsage::default(),
            finish_reason: Some("stop".to_string()),
        },
    ]);

    let step = result.expect("text and full messages should produce a step");
    assert_eq!(step.full_messages, latest);
}

#[test]
fn collect_stream_rejects_whitespace_only_reasoning_fallback() {
    let (result, deltas) = collect_events([
        llm::StreamEvent::ReasoningDelta("   ".to_string()),
        llm::StreamEvent::Done {
            usage: models::TokenUsage::default(),
            finish_reason: Some("stop".to_string()),
        },
    ]);

    assert_eq!(result.unwrap_err(), "模型未返回内容");
    assert_eq!(deltas, vec!["<think>   ", "</think>\n\n"]);
}

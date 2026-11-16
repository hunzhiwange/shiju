//! Consumer rejection, abort signalling and timeout flush boundaries.
use super::*;
use crate::llm_runner as runner;
use crate::processor_events::StreamEvent as CollectorStreamEvent;
use crate::message::AssistantError;
use llm_runtime as llm;
use session_port::ui_types as models;
use std::sync::mpsc;
use std::time::Duration;

#[path = "llm_runner_test_support.rs"]
mod llm_runner_test_support;
use llm_runner_test_support::collect_events_with_delta_consumer;

#[test]
fn collect_stream_aborts_when_consumer_stops() {
    let (tx, rx) = mpsc::channel();
    tx.send(llm::StreamEvent::Delta("stop".to_string())).expect("delta should be queued");
    let _keep_sender_alive = tx;
    let (abort_tx, mut abort_rx) = tokio::sync::watch::channel(false);

    let result = super::collect_llm_step_from_stream(
        rx,
        &mut |_event| false,
        &abort_tx,
        Duration::from_millis(5),
        None,
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert!(*abort_rx.borrow_and_update());
}

#[test]
fn emit_stream_event_ok_when_consumer_returns_true() {
    let (abort_tx, mut abort_rx) = tokio::sync::watch::channel(false);
    let mut captured = String::new();
    let result = super::emit_stream_event(
        &mut |event| {
            if let super::StreamEvent::Delta(text) = event {
                captured = text;
            }
            true
        },
        &abort_tx,
        super::StreamEvent::Delta("proceed".to_string()),
    );

    assert!(result.is_ok());
    assert_eq!(captured, "proceed");
    assert!(!*abort_rx.borrow_and_update());
}

#[test]
fn emit_stream_event_stops_when_consumer_returns_false() {
    let (abort_tx, mut abort_rx) = tokio::sync::watch::channel(false);
    let result = super::emit_stream_event(
        &mut |_event| false,
        &abort_tx,
        super::StreamEvent::Delta("stop".to_string()),
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert!(*abort_rx.borrow_and_update());
}

#[test]
fn non_retryable_llm_error_matches_consumer_stopped_direct_string() {
    assert!(!super::llm_step_retry_allowed(super::STREAM_CONSUMER_STOPPED_ERR, false));
}

#[test]
fn collect_stream_timeout_flushes_pending_text_and_requests_abort() {
    let (tx, rx) = mpsc::channel();
    tx.send(llm::StreamEvent::Delta("partial".to_string())).expect("text delta should be queued");
    let _keep_sender_alive = tx;

    let (abort_tx, mut abort_rx) = tokio::sync::watch::channel(false);
    let mut deltas = Vec::new();
    let result = super::collect_llm_step_from_stream(
        rx,
        &mut |event| {
            if let super::StreamEvent::Delta(text) = event {
                deltas.push(text);
            }
            true
        },
        &abort_tx,
        Duration::from_millis(1),
        None,
    );

    assert_eq!(result.unwrap_err(), "模型响应超时");
    assert_eq!(deltas, vec!["partial"]);
    assert!(*abort_rx.borrow_and_update());
}

#[test]
fn collect_stream_aborts_when_consumer_rejects_reasoning_close_tag() {
    let (tx, rx) = mpsc::channel();
    tx.send(llm::StreamEvent::ReasoningDelta("plan".to_string()))
        .expect("reasoning delta should be queued");
    tx.send(llm::StreamEvent::Done {
        usage: models::TokenUsage::default(),
        finish_reason: Some("stop".to_string()),
    })
    .expect("done event should be queued");
    drop(tx);

    let (abort_tx, mut abort_rx) = tokio::sync::watch::channel(false);
    let mut deltas = Vec::new();
    let result = super::collect_llm_step_from_stream(
        rx,
        &mut |event| {
            let super::StreamEvent::Delta(text) = event else {
                return true;
            };
            deltas.push(text);
            deltas.len() == 1
        },
        &abort_tx,
        Duration::from_millis(5),
        None,
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert_eq!(deltas, vec!["<think>plan", "</think>\n\n"]);
    assert!(*abort_rx.borrow_and_update());
}

#[test]
fn collect_stream_aborts_when_consumer_rejects_pending_text_on_timeout() {
    let (result, deltas, aborted) = collect_events_with_delta_consumer(
        [llm::StreamEvent::Delta("partial".to_string())],
        true,
        Duration::from_millis(1),
        |_text, _index| false,
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert_eq!(deltas, vec!["partial"]);
    assert!(aborted);
}

#[test]
fn collect_stream_aborts_when_consumer_rejects_pending_reasoning_on_timeout() {
    let (result, deltas, aborted) = collect_events_with_delta_consumer(
        [llm::StreamEvent::ReasoningDelta("slow".to_string())],
        true,
        Duration::from_millis(1),
        |_text, _index| false,
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert_eq!(deltas, vec!["<think>slow"]);
    assert!(aborted);
}

#[test]
fn collect_stream_aborts_when_consumer_rejects_reasoning_close_on_timeout() {
    let (result, deltas, aborted) = collect_events_with_delta_consumer(
        [llm::StreamEvent::ReasoningDelta("slow".to_string())],
        true,
        Duration::from_millis(1),
        |_text, index| index == 0,
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert_eq!(deltas, vec!["<think>slow", "</think>\n\n"]);
    assert!(aborted);
}

#[test]
fn collect_stream_aborts_when_consumer_rejects_reasoning_before_text_close() {
    let (result, deltas, aborted) = collect_events_with_delta_consumer(
        [
            llm::StreamEvent::ReasoningDelta("plan".to_string()),
            llm::StreamEvent::Delta("answer".to_string()),
        ],
        false,
        Duration::from_millis(5),
        |_text, index| index == 0,
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert_eq!(deltas, vec!["<think>plan", "</think>\n\n"]);
    assert!(aborted);
}

#[test]
fn collect_stream_aborts_when_consumer_rejects_reasoning_before_text_flush() {
    let (result, deltas, aborted) = collect_events_with_delta_consumer(
        [
            llm::StreamEvent::ReasoningDelta("plan".to_string()),
            llm::StreamEvent::Delta("answer".to_string()),
        ],
        false,
        Duration::from_millis(5),
        |_text, _index| false,
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert_eq!(deltas, vec!["<think>plan"]);
    assert!(aborted);
}

#[test]
fn collect_stream_aborts_when_consumer_rejects_text_threshold_flush() {
    let (result, deltas, aborted) = collect_events_with_delta_consumer(
        [llm::StreamEvent::Delta("x".repeat(1024))],
        false,
        Duration::from_millis(5),
        |_text, _index| false,
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas[0].len(), 1024);
    assert!(aborted);
}

#[test]
fn collect_stream_aborts_when_consumer_rejects_reasoning_threshold_flush() {
    let (result, deltas, aborted) = collect_events_with_delta_consumer(
        [llm::StreamEvent::ReasoningDelta("r".repeat(1024))],
        false,
        Duration::from_millis(5),
        |_text, _index| false,
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert_eq!(deltas, vec![format!("<think>{}", "r".repeat(1024))]);
    assert!(aborted);
}

#[test]
fn collect_stream_aborts_when_consumer_rejects_pending_text_on_error() {
    let (result, deltas, aborted) = collect_events_with_delta_consumer(
        [
            llm::StreamEvent::Delta("speak".to_string()),
            llm::StreamEvent::Error(AssistantError::Unknown { message: "fail".to_string() }),
        ],
        false,
        Duration::from_millis(5),
        |_text, _index| false,
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert_eq!(deltas, vec!["speak"]);
    assert!(aborted);
}

#[test]
fn collect_stream_aborts_when_consumer_rejects_pending_reasoning_on_error() {
    let (result, deltas, aborted) = collect_events_with_delta_consumer(
        [
            llm::StreamEvent::ReasoningDelta("think".to_string()),
            llm::StreamEvent::Error(AssistantError::Unknown { message: "fail".to_string() }),
        ],
        false,
        Duration::from_millis(5),
        |_text, _index| false,
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert_eq!(deltas, vec!["<think>think"]);
    assert!(aborted);
}

#[test]
fn collect_stream_aborts_when_consumer_rejects_reasoning_close_on_error() {
    let (result, deltas, aborted) = collect_events_with_delta_consumer(
        [
            llm::StreamEvent::ReasoningDelta("think".to_string()),
            llm::StreamEvent::Error(AssistantError::Unknown { message: "fail".to_string() }),
        ],
        false,
        Duration::from_millis(5),
        |_text, index| index == 0,
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert_eq!(deltas, vec!["<think>think", "</think>\n\n"]);
    assert!(aborted);
}

#[test]
fn collect_stream_aborts_when_consumer_rejects_final_pending_text() {
    let (result, deltas, aborted) = collect_events_with_delta_consumer(
        [llm::StreamEvent::Delta("tail".to_string())],
        false,
        Duration::from_millis(5),
        |_text, _index| false,
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert_eq!(deltas, vec!["tail"]);
    assert!(aborted);
}

#[test]
fn collect_stream_aborts_when_consumer_rejects_final_pending_reasoning() {
    let (result, deltas, aborted) = collect_events_with_delta_consumer(
        [
            llm::StreamEvent::ReasoningDelta("plan".to_string()),
            llm::StreamEvent::Done {
                usage: models::TokenUsage::default(),
                finish_reason: Some("stop".to_string()),
            },
        ],
        false,
        Duration::from_millis(5),
        |_text, _index| false,
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert_eq!(deltas, vec!["<think>plan"]);
    assert!(aborted);
}

#[test]
fn collect_stream_aborts_when_consumer_rejects_late_reasoning_summary() {
    let (result, deltas, aborted) = collect_events_with_delta_consumer(
        [
            llm::StreamEvent::Delta("answer".to_string()),
            llm::StreamEvent::ReasoningDelta("late plan".to_string()),
            llm::StreamEvent::Done {
                usage: models::TokenUsage::default(),
                finish_reason: Some("stop".to_string()),
            },
        ],
        false,
        Duration::from_millis(5),
        |_text, index| index == 0,
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert_eq!(deltas, vec!["answer", "\n\n<think>late plan</think>\n\n"]);
    assert!(aborted);
}

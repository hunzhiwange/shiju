//! Scripted stream fixtures shared by library and host contract tests.
use super::runner;
use llm_runtime as llm;
use session_port::ui_types as models;
use std::sync::mpsc;
use std::time::Duration;

#[allow(dead_code)] // Used by library buffering contracts.
pub(super) fn collect_events(
    events: impl IntoIterator<Item = llm::StreamEvent>,
) -> (Result<runner::LlmStep, String>, Vec<String>) {
    let (tx, rx) = mpsc::channel();
    for event in events {
        tx.send(event).expect("stream event should be queued");
    }
    drop(tx);

    let (abort_tx, _abort_rx) = tokio::sync::watch::channel(false);
    let mut deltas = Vec::new();
    let result = runner::collect_llm_step_from_stream(
        rx,
        &mut |event| {
            if let super::CollectorStreamEvent::Delta(text) = event {
                deltas.push(text);
            }
            true
        },
        &abort_tx,
        Duration::from_millis(5),
        None,
    );
    (result, deltas)
}

#[allow(dead_code)] // Used by host consumer/timeout contracts.
pub(super) fn collect_events_with_delta_consumer(
    events: impl IntoIterator<Item = llm::StreamEvent>,
    keep_sender_alive: bool,
    idle_timeout: Duration,
    mut accept_delta: impl FnMut(&str, usize) -> bool,
) -> (Result<runner::LlmStep, String>, Vec<String>, bool) {
    let (tx, rx) = mpsc::channel();
    for event in events {
        tx.send(event).expect("stream event should be queued");
    }
    let keep_sender_alive = keep_sender_alive.then_some(tx);

    let (abort_tx, mut abort_rx) = tokio::sync::watch::channel(false);
    let mut deltas = Vec::new();
    let result = runner::collect_llm_step_from_stream(
        rx,
        &mut |event| {
            let super::CollectorStreamEvent::Delta(text) = event else {
                return true;
            };
            let index = deltas.len();
            let keep_going = accept_delta(&text, index);
            deltas.push(text);
            keep_going
        },
        &abort_tx,
        idle_timeout,
        None,
    );
    drop(keep_sender_alive);

    (result, deltas, *abort_rx.borrow_and_update())
}

#[allow(dead_code)] // Used by library first-delta contracts.
pub(super) fn observe_first_delta_before_stream_continues(
    first_event: llm::StreamEvent,
    remaining_events: impl IntoIterator<Item = llm::StreamEvent>,
) -> String {
    let (stream_tx, stream_rx) = mpsc::channel();
    let (observed_tx, observed_rx) = mpsc::channel();
    let collector = std::thread::spawn(move || {
        let (abort_tx, _abort_rx) = tokio::sync::watch::channel(false);
        runner::collect_llm_step_from_stream(
            stream_rx,
            &mut |event| {
                if let super::CollectorStreamEvent::Delta(delta) = event {
                    let _ = observed_tx.send(delta);
                }
                true
            },
            &abort_tx,
            Duration::from_secs(5),
            None,
        )
    });

    stream_tx.send(first_event).expect("first stream event should be sent");
    let first_delta = observed_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("first delta should be observed before another stream event arrives");
    for event in remaining_events {
        stream_tx.send(event).expect("remaining stream event should be sent");
    }
    drop(stream_tx);

    collector.join().expect("stream collector should not panic").expect("stream should complete");
    first_delta
}

pub(super) fn usage(input_tokens: i64, output_tokens: i64) -> models::TokenUsage {
    models::TokenUsage { input_tokens, output_tokens, cached_tokens: 3, reasoning_tokens: 4 }
}


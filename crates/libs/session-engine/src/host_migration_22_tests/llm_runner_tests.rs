use crate::{llm_runner as runner, llm_stream as llm};
use std::{sync::mpsc, time::Duration};
use session_port::ui_types as models;

#[test]
fn acp_request_detection_accepts_test_flag_and_agent_name() {
    assert!(runner::is_acp_request(&serde_json::json!({ "acp_test": true })));
    assert!(runner::is_acp_request(&serde_json::json!({ "acp_agent": "codex" })));
    assert!(!runner::is_acp_request(&serde_json::json!({ "acp_agent": "  " })));
    assert!(!runner::is_acp_request(&serde_json::json!({})));
}

#[test]
fn collect_stream_times_out_and_closes_open_reasoning() {
    let (tx, rx) = mpsc::channel();
    tx.send(llm::StreamEvent::ReasoningDelta("slow".to_string()))
        .expect("reasoning delta should be queued");
    let _keep_sender_alive = tx;

    let (abort_tx, mut abort_rx) = tokio::sync::watch::channel(false);
    let mut deltas = Vec::new();
    let result = runner::collect_llm_step_from_stream(
        rx,
        &mut |event| {
            if let crate::processor_events::StreamEvent::Delta(text) = event {
                deltas.push(text);
            }
            true
        },
        &abort_tx,
        Duration::from_millis(1),
        None,
    );

    assert_eq!(result.unwrap_err(), "模型响应超时");
    assert_eq!(deltas, vec!["<think>slow", "</think>\n\n"]);
    assert!(*abort_rx.borrow_and_update());
}

#[test]
fn collect_stream_allows_silent_acp_prompt_past_model_idle_timeout() {
    let (tx, rx) = mpsc::channel();
    tx.send(llm::StreamEvent::AcpStarted).unwrap();
    let (abort_tx, abort_rx) = tokio::sync::watch::channel(false);
    let worker = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(60));
        assert!(!*abort_rx.borrow());
        tx.send(llm::StreamEvent::Delta("tool finished".into())).unwrap();
        tx.send(llm::StreamEvent::Done {
            finish_reason: Some("stop".into()),
            usage: models::TokenUsage::default(),
        })
        .unwrap();
    });
    let result = runner::collect_llm_step_from_stream(
        rx,
        &mut |_| true,
        &abort_tx,
        Duration::from_millis(10),
        None,
    );

    worker.join().unwrap();
    assert_eq!(result.unwrap().text, "tool finished");
}

#[test]
fn collect_stream_reports_empty_when_sender_disconnects_without_done() {
    let (tx, rx) = mpsc::channel();
    drop(tx);

    let (abort_tx, _abort_rx) = tokio::sync::watch::channel(false);
    let mut deltas = Vec::new();
    let result = runner::collect_llm_step_from_stream(
        rx,
        &mut |event| {
            if let crate::processor_events::StreamEvent::Delta(text) = event {
                deltas.push(text);
            }
            true
        },
        &abort_tx,
        Duration::from_millis(5),
        None,
    );

    assert_eq!(result.unwrap_err(), "模型未返回内容");
    assert!(deltas.is_empty());
}

#[test]
fn llm_error_message_extracts_message_field_from_json() {
    assert_eq!(
        runner::llm_error_message(r#"{"message":"rate limit exceeded"}"#),
        "rate limit exceeded"
    );
}

#[test]
fn llm_error_message_falls_back_to_raw_string_when_not_json() {
    assert_eq!(runner::llm_error_message("connection refused"), "connection refused");
}

#[test]
fn llm_error_message_falls_back_to_raw_string_when_json_without_message() {
    assert_eq!(runner::llm_error_message(r#"{"code":503}"#), r#"{"code":503}"#,);
}

#[test]
fn llm_error_message_falls_back_when_json_message_is_not_text() {
    let error = r#"{"message":{"detail":"rate limited"},"code":429}"#;

    assert_eq!(runner::llm_error_message(error), error);
    assert!(!runner::llm_step_retry_allowed(error, false));
}

#[test]
fn acp_request_detection_rejects_unexpected_option_types() {
    assert!(!runner::is_acp_request(&serde_json::json!({ "acp_test": "true" })));
    assert!(!runner::is_acp_request(&serde_json::json!({ "acp_agent": 7 })));
    assert!(runner::is_acp_request(&serde_json::json!({
        "acp_test": false,
        "acp_agent": "  codex  "
    })));
}

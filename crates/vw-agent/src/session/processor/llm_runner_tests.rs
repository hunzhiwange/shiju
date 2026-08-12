#[test]
fn llm_runner_tests_module_is_wired() {
    let marker = String::from("llm_runner_tests");
    assert_eq!(marker.as_str(), "llm_runner_tests");
}

use crate::app::agent::session::llm;
use crate::app::agent::session::message::AssistantError;
use crate::app::agent::tools::ToolSpec;
use crate::session::ui_types as models;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::Duration;

static TEST_LLM_STREAMS: std::sync::LazyLock<std::sync::Mutex<VecDeque<Vec<llm::StreamEvent>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(VecDeque::new()));

static TEST_LLM_STREAM_TOOL_IDS: std::sync::LazyLock<std::sync::Mutex<Vec<Vec<String>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

static TEST_LLM_STREAM_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

pub(crate) fn test_llm_stream_guard() -> std::sync::MutexGuard<'static, ()> {
    let guard = TEST_LLM_STREAM_LOCK.lock().expect("lock scripted LLM stream tests");
    TEST_LLM_STREAMS.lock().expect("lock scripted LLM streams").clear();
    TEST_LLM_STREAM_TOOL_IDS.lock().expect("lock scripted LLM tool ids").clear();
    guard
}

pub(crate) fn enqueue_test_llm_stream(events: Vec<llm::StreamEvent>) {
    TEST_LLM_STREAMS.lock().expect("lock scripted LLM streams").push_back(events);
}

pub(crate) fn take_test_llm_stream_tool_ids() -> Vec<Vec<String>> {
    std::mem::take(&mut *TEST_LLM_STREAM_TOOL_IDS.lock().expect("lock scripted LLM tool ids"))
}

pub(super) fn spawn_test_llm_stream_thread(
    tools: std::collections::HashMap<String, ToolSpec>,
    mut abort_rx: tokio::sync::watch::Receiver<bool>,
    tx: std::sync::mpsc::Sender<llm::StreamEvent>,
) {
    let mut tool_ids = tools.keys().cloned().collect::<Vec<_>>();
    tool_ids.sort();
    TEST_LLM_STREAM_TOOL_IDS.lock().expect("lock scripted LLM tool ids").push(tool_ids);
    let events =
        TEST_LLM_STREAMS.lock().expect("lock scripted LLM streams").pop_front().unwrap_or_else(
            || {
                vec![llm::StreamEvent::Error(
                    crate::app::agent::session::message::AssistantError::Unknown {
                        message: "missing scripted LLM stream".to_string(),
                    },
                )]
            },
        );

    std::thread::spawn(move || {
        for event in events {
            if *abort_rx.borrow_and_update() {
                break;
            }
            let _ = tx.send(event);
        }
    });
}

fn collect_events(
    events: impl IntoIterator<Item = llm::StreamEvent>,
) -> (Result<super::LlmStep, String>, Vec<String>) {
    let (tx, rx) = mpsc::channel();
    for event in events {
        tx.send(event).expect("stream event should be queued");
    }
    drop(tx);

    let (abort_tx, _abort_rx) = tokio::sync::watch::channel(false);
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
        Duration::from_millis(5),
    );
    (result, deltas)
}

fn collect_events_with_delta_consumer(
    events: impl IntoIterator<Item = llm::StreamEvent>,
    keep_sender_alive: bool,
    idle_timeout: Duration,
    mut accept_delta: impl FnMut(&str, usize) -> bool,
) -> (Result<super::LlmStep, String>, Vec<String>, bool) {
    let (tx, rx) = mpsc::channel();
    for event in events {
        tx.send(event).expect("stream event should be queued");
    }
    let keep_sender_alive = keep_sender_alive.then_some(tx);

    let (abort_tx, mut abort_rx) = tokio::sync::watch::channel(false);
    let mut deltas = Vec::new();
    let result = super::collect_llm_step_from_stream(
        rx,
        &mut |event| {
            let super::StreamEvent::Delta(text) = event else {
                return true;
            };
            let index = deltas.len();
            let keep_going = accept_delta(&text, index);
            deltas.push(text);
            keep_going
        },
        &abort_tx,
        idle_timeout,
    );
    drop(keep_sender_alive);

    (result, deltas, *abort_rx.borrow_and_update())
}

fn usage(input_tokens: i64, output_tokens: i64) -> models::TokenUsage {
    models::TokenUsage { input_tokens, output_tokens, cached_tokens: 3, reasoning_tokens: 4 }
}

#[test]
fn acp_request_detection_accepts_test_flag_and_agent_name() {
    assert!(super::is_acp_request(&serde_json::json!({ "acp_test": true })));
    assert!(super::is_acp_request(&serde_json::json!({ "acp_agent": "codex" })));
    assert!(!super::is_acp_request(&serde_json::json!({ "acp_agent": "  " })));
    assert!(!super::is_acp_request(&serde_json::json!({})));
}

#[test]
fn acp_retryable_error_matches_timeout_and_disconnect_messages() {
    assert!(super::is_acp_retryable_error("模型响应超时"));
    assert!(super::is_acp_retryable_error("request TIMED OUT"));
    assert!(super::is_acp_retryable_error("acp agent disconnected during request"));
    assert!(super::is_acp_retryable_error("queue owner disconnected before prompt completion"));
    assert!(super::is_acp_retryable_error("API error: None - error decoding response body"));
    assert!(!super::is_acp_retryable_error("invalid api key"));
}

#[test]
fn non_retryable_llm_error_matches_model_resolution_failures() {
    assert!(super::is_non_retryable_llm_error(
        r#"{"name":"Unknown","message":"未找到模型：llama3"}"#
    ));
    assert!(super::is_non_retryable_llm_error("模型格式错误：/llama3，请使用 provider/model"));
    assert!(super::is_non_retryable_llm_error("模型ID存在歧义：llama3，请使用 provider/model"));
    assert!(super::is_non_retryable_llm_error("model not found"));
    assert!(!super::is_non_retryable_llm_error("模型响应超时"));
}

#[test]
fn llm_step_retry_allowed_blocks_model_config_errors_only() {
    assert!(!super::llm_step_retry_allowed(
        r#"{"name":"Unknown","message":"未找到模型：llama3"}"#,
        false,
    ));
    assert!(super::llm_step_retry_allowed("模型响应超时", false));
    assert!(super::llm_step_retry_allowed("模型响应超时", true));
    assert!(!super::llm_step_retry_allowed("invalid api key", true));
}

#[test]
fn llm_step_idle_timeout_uses_long_window_for_one_shot_acp_requests() {
    assert_eq!(
        super::llm_step_idle_timeout(&serde_json::json!({})),
        super::DEFAULT_LLM_STEP_IDLE_TIMEOUT
    );
    assert_eq!(
        super::llm_step_idle_timeout(&serde_json::json!({
            "acp_close_client_after_request": true
        })),
        super::ONE_SHOT_ACP_STEP_IDLE_TIMEOUT
    );
    // 一次性 ACP 会话由远端代理跑分钟级工具命令，空窗上限必须远大于普通模型 token 间隔。
    assert!(super::ONE_SHOT_ACP_STEP_IDLE_TIMEOUT >= super::DEFAULT_LLM_STEP_IDLE_TIMEOUT);
}

#[test]
fn collect_stream_returns_text_tool_calls_usage_and_full_messages() {
    let tool_call = llm::ToolCall {
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
fn collect_stream_times_out_and_closes_open_reasoning() {
    let (tx, rx) = mpsc::channel();
    tx.send(llm::StreamEvent::ReasoningDelta("slow".to_string()))
        .expect("reasoning delta should be queued");
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
    );

    assert_eq!(result.unwrap_err(), "模型响应超时");
    assert_eq!(deltas, vec!["<think>slow", "</think>\n\n"]);
    assert!(*abort_rx.borrow_and_update());
}

#[test]
fn collect_stream_reports_empty_when_sender_disconnects_without_done() {
    let (tx, rx) = mpsc::channel();
    drop(tx);

    let (abort_tx, _abort_rx) = tokio::sync::watch::channel(false);
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
        Duration::from_millis(5),
    );

    assert_eq!(result.unwrap_err(), "模型未返回内容");
    assert!(deltas.is_empty());
}

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
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert!(*abort_rx.borrow_and_update());
}

// ── llm_error_message ──────────────────────────────────────────────────────

#[test]
fn llm_error_message_extracts_message_field_from_json() {
    assert_eq!(
        super::llm_error_message(r#"{"message":"rate limit exceeded"}"#),
        "rate limit exceeded"
    );
}

#[test]
fn llm_error_message_falls_back_to_raw_string_when_not_json() {
    assert_eq!(super::llm_error_message("connection refused"), "connection refused");
}

#[test]
fn llm_error_message_falls_back_to_raw_string_when_json_without_message() {
    assert_eq!(super::llm_error_message(r#"{"code":503}"#), r#"{"code":503}"#,);
}

// ── emit_stream_event ──────────────────────────────────────────────────────

#[test]
fn llm_error_message_falls_back_when_json_message_is_not_text() {
    let error = r#"{"message":{"detail":"rate limited"},"code":429}"#;

    assert_eq!(super::llm_error_message(error), error);
    assert!(!super::is_non_retryable_llm_error(error));
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

// ── is_non_retryable_llm_error with STREAM_CONSUMER_STOPPED_ERR ─────────────

#[test]
fn non_retryable_llm_error_matches_consumer_stopped_direct_string() {
    assert!(super::is_non_retryable_llm_error(super::STREAM_CONSUMER_STOPPED_ERR));
}

// ── collect_llm_step_from_stream additional edge cases ─────────────────────

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
        id: "call-1".to_string(),
        name: "read_file".to_string(),
        arguments: "{}".to_string(),
    };
    let second = llm::ToolCall {
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
fn acp_request_detection_rejects_unexpected_option_types() {
    assert!(!super::is_acp_request(&serde_json::json!({ "acp_test": "true" })));
    assert!(!super::is_acp_request(&serde_json::json!({ "acp_agent": 7 })));
    assert!(super::is_acp_request(&serde_json::json!({
        "acp_test": false,
        "acp_agent": "  codex  "
    })));
}

#[test]
fn acp_retryable_error_matches_timeout_word_and_eof_failure() {
    assert!(super::is_acp_retryable_error("request timeout"));
    assert!(super::is_acp_retryable_error("EOF while parsing a value at line 1"));
}

#[test]
fn non_retryable_llm_error_matches_english_model_resolution_failures() {
    assert!(super::is_non_retryable_llm_error("unknown model: custom/model"));
    assert!(super::is_non_retryable_llm_error("unsupported model family"));
}

#[test]
fn llm_step_idle_timeout_requires_literal_true_flag() {
    for options in [
        serde_json::json!({ "acp_close_client_after_request": false }),
        serde_json::json!({ "acp_close_client_after_request": "true" }),
        serde_json::json!({ "acp_close_client_after_request": 1 }),
    ] {
        assert_eq!(super::llm_step_idle_timeout(&options), super::DEFAULT_LLM_STEP_IDLE_TIMEOUT);
    }
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
    );

    assert_eq!(result.unwrap_err(), super::STREAM_CONSUMER_STOPPED_ERR);
    assert_eq!(deltas, vec!["<think>plan", "</think>\n\n"]);
    assert!(*abort_rx.borrow_and_update());
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

#[test]
fn run_llm_step_uses_scripted_stream_and_filters_allowed_tools() {
    let _guard = test_llm_stream_guard();
    enqueue_test_llm_stream(vec![
        llm::StreamEvent::Delta("scripted answer".to_string()),
        llm::StreamEvent::Done { usage: usage(5, 8), finish_reason: Some("stop".to_string()) },
    ]);
    let available_tools = vec![
        ToolSpec::new("file_read", "Read a file", serde_json::json!({})),
        ToolSpec::new("shell", "Run a command", serde_json::json!({})),
    ];
    let allowed_tools = HashSet::from(["file_read".to_string()]);
    let mut deltas = Vec::new();

    let step = super::run_llm_step(
        "session-1",
        &[serde_json::json!({ "role": "user", "content": "hello" })],
        &["system".to_string()],
        Some("provider/model".to_string()),
        &serde_json::json!({}),
        &available_tools,
        &allowed_tools,
        &mut |event| {
            if let super::StreamEvent::Delta(text) = event {
                deltas.push(text);
            }
            true
        },
    )
    .expect("scripted LLM stream should produce a step");

    assert_eq!(step.text, "scripted answer");
    assert_eq!(step.usage.output_tokens, 8);
    assert_eq!(deltas, vec!["scripted answer"]);
    assert_eq!(take_test_llm_stream_tool_ids(), vec![vec!["file_read".to_string()]]);
}

#[test]
fn run_llm_step_with_retry_retries_retryable_error_then_succeeds() {
    let _guard = test_llm_stream_guard();
    enqueue_test_llm_stream(vec![llm::StreamEvent::Error(AssistantError::Unknown {
        message: "temporary timeout".to_string(),
    })]);
    enqueue_test_llm_stream(vec![
        llm::StreamEvent::Delta("second attempt".to_string()),
        llm::StreamEvent::Done { usage: usage(3, 4), finish_reason: Some("stop".to_string()) },
    ]);
    let mut deltas = Vec::new();

    let step = super::run_llm_step_with_retry(
        "session-retry",
        &[],
        &[],
        None,
        &serde_json::json!({}),
        &[],
        &HashSet::new(),
        &mut |event| {
            if let super::StreamEvent::Delta(text) = event {
                deltas.push(text);
            }
            true
        },
        2,
    )
    .expect("second scripted attempt should succeed");

    assert_eq!(step.text, "second attempt");
    assert_eq!(step.usage.input_tokens, 3);
    assert_eq!(deltas, vec!["second attempt"]);
    assert_eq!(take_test_llm_stream_tool_ids().len(), 2);
}

#[test]
fn run_llm_step_with_retry_stops_on_non_retryable_error() {
    let _guard = test_llm_stream_guard();
    enqueue_test_llm_stream(vec![llm::StreamEvent::Error(AssistantError::Unknown {
        message: "model not found".to_string(),
    })]);

    let error = super::run_llm_step_with_retry(
        "session-non-retry",
        &[],
        &[],
        None,
        &serde_json::json!({}),
        &[],
        &HashSet::new(),
        &mut |_event| true,
        3,
    )
    .unwrap_err();

    assert!(error.contains("model not found"));
    assert_eq!(take_test_llm_stream_tool_ids().len(), 1);
}

#[test]
fn run_llm_step_with_retry_treats_zero_attempts_as_one() {
    let _guard = test_llm_stream_guard();
    enqueue_test_llm_stream(vec![llm::StreamEvent::Error(AssistantError::Unknown {
        message: "temporary timeout".to_string(),
    })]);

    let error = super::run_llm_step_with_retry(
        "session-one-attempt",
        &[],
        &[],
        None,
        &serde_json::json!({}),
        &[],
        &HashSet::new(),
        &mut |_event| true,
        0,
    )
    .unwrap_err();

    assert!(error.contains("temporary timeout"));
    assert_eq!(take_test_llm_stream_tool_ids().len(), 1);
}

#[test]
fn run_llm_step_reports_missing_scripted_stream() {
    let _guard = test_llm_stream_guard();

    let error = super::run_llm_step(
        "session-missing-script",
        &[],
        &[],
        None,
        &serde_json::json!({}),
        &[],
        &HashSet::new(),
        &mut |_event| true,
    )
    .unwrap_err();

    assert!(error.contains("missing scripted LLM stream"));
    assert_eq!(take_test_llm_stream_tool_ids(), vec![Vec::<String>::new()]);
}

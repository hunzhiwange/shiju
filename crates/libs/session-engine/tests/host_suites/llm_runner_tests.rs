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

use session_engine::llm_runner as runner;
use super::StreamEvent as CollectorStreamEvent;
#[path = "../../src/llm_runner_test_support.rs"]
mod llm_runner_test_support;
use llm_runner_test_support::usage;

pub(crate) struct RecordedLlmRequest {
    pub(crate) tool_specs: Vec<ToolSpec>,
    pub(crate) system: Vec<String>,
    pub(crate) model: Option<String>,
    pub(crate) directory: String,
    pub(crate) tenant: vw_api_transport::TenantScope,
    pub(crate) messages: Vec<serde_json::Value>,
    pub(crate) options: serde_json::Value,
}
type RecordedMessages = std::collections::HashMap<String, Vec<RecordedLlmRequest>>;
static TEST_LLM_MESSAGES: std::sync::LazyLock<std::sync::Mutex<RecordedMessages>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Default::default()));

pub(super) fn record_test_llm_messages(
    session: String,
    messages: Vec<serde_json::Value>,
    options: serde_json::Value,
    system: Vec<String>,
    model: Option<String>,
    tool_specs: Vec<ToolSpec>,
) {
    TEST_LLM_MESSAGES.lock().unwrap().entry(session).or_default().push(RecordedLlmRequest {
        tool_specs,
        system,
        model,
        directory: crate::project::instance::directory(),
        tenant: crate::session::ui_store::current_tenant_scope(),
        messages,
        options,
    });
}

pub(crate) fn take_test_llm_messages(session: &str) -> Vec<RecordedLlmRequest> {
    TEST_LLM_MESSAGES.lock().unwrap().remove(session).unwrap_or_default()
}

static TEST_LLM_STREAMS: std::sync::LazyLock<std::sync::Mutex<VecDeque<Vec<llm::StreamEvent>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(VecDeque::new()));

static TEST_LLM_STREAM_TOOL_IDS: std::sync::LazyLock<std::sync::Mutex<Vec<Vec<String>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

static TEST_LLM_STREAM_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

pub(crate) fn test_llm_stream_guard() -> std::sync::MutexGuard<'static, ()> {
    let guard = TEST_LLM_STREAM_LOCK.lock().expect("lock scripted LLM stream tests");
    TEST_LLM_STREAMS.lock().expect("lock scripted LLM streams").clear();
    TEST_LLM_MESSAGES.lock().unwrap().clear();
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
    session: &str,
    tools: std::collections::HashMap<String, ToolSpec>,
    mut abort_rx: tokio::sync::watch::Receiver<bool>,
    tx: std::sync::mpsc::Sender<llm::StreamEvent>,
) -> std::thread::JoinHandle<()> {
    if let Some(stream) = CHANNEL_CANCEL_STREAMS.lock().unwrap().remove(session) {
        return std::thread::spawn(move || {
            stream.started.store(true, std::sync::atomic::Ordering::Release);
            loop {
                let cancelled = *abort_rx.borrow_and_update();
                if stream.release.load(std::sync::atomic::Ordering::Acquire) {
                    if !cancelled {
                        let _ = tx.send(llm::StreamEvent::Delta("completed naturally".into()));
                        let _ = tx.send(llm::StreamEvent::Done {
                            finish_reason: Some("stop".into()),
                            usage: Default::default(),
                        });
                    }
                    break;
                }
                if cancelled && !stream.hold_exit {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            stream.exited.store(true, std::sync::atomic::Ordering::Release);
        });
    }
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
    })
}

#[test]
fn acp_request_detection_accepts_test_flag_and_agent_name() {
    assert!(super::is_acp_request(&serde_json::json!({ "acp_test": true })));
    assert!(super::is_acp_request(&serde_json::json!({ "acp_agent": "codex" })));
    assert!(!super::is_acp_request(&serde_json::json!({ "acp_agent": "  " })));
    assert!(!super::is_acp_request(&serde_json::json!({})));
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
    let result = super::collect_llm_step_from_stream(
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
        None,
    );

    assert_eq!(result.unwrap_err(), "模型未返回内容");
    assert!(deltas.is_empty());
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


#[test]
fn llm_error_message_falls_back_when_json_message_is_not_text() {
    let error = r#"{"message":{"detail":"rate limited"},"code":429}"#;

    assert_eq!(super::llm_error_message(error), error);
    assert!(!super::llm_step_retry_allowed(error, false));
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
        None,
    )
    .expect("scripted LLM stream should produce a step");

    assert_eq!(step.text, "scripted answer");
    assert_eq!(step.usage.output_tokens, 8);
    assert_eq!(deltas, vec!["scripted answer"]);
    assert_eq!(take_test_llm_stream_tool_ids(), vec![vec!["file_read".to_string()]]);
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
        None,
    )
    .unwrap_err();

    assert!(error.contains("missing scripted LLM stream"));
    assert_eq!(take_test_llm_stream_tool_ids(), vec![Vec::<String>::new()]);
}

#[derive(Default)]
pub(crate) struct ChannelCancelStream {
    pub(crate) started: std::sync::atomic::AtomicBool,
    pub(crate) exited: std::sync::atomic::AtomicBool,
    pub(crate) release: std::sync::atomic::AtomicBool,
    pub(crate) hold_exit: bool,
}
static CHANNEL_CANCEL_STREAMS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<ChannelCancelStream>>>,
> = std::sync::LazyLock::new(Default::default);
pub(crate) fn register_channel_cancel_stream(
    session: &str,
    stream: std::sync::Arc<ChannelCancelStream>,
) {
    CHANNEL_CANCEL_STREAMS.lock().unwrap().insert(session.into(), stream);
}

#[test]
fn shell_outer_timeout_model_idle_does_not_cover_background_runtime() {
    use crate::tools::shell::processes::{ShellProcessScope, ShellProcessState, ShellProcesses};
    let dir = tempfile::tempdir().unwrap();
    let scope =
        ShellProcessScope::new("idle-tenant".into(), dir.path(), "idle-session".into()).unwrap();
    let host = ShellProcesses::new::<crate::tools::context::AgentContextHost>().unwrap();
    let caller = tokio::runtime::Runtime::new().unwrap();
    let started = caller
        .block_on(host.start(scope.clone(), || {
            let mut command = tokio::process::Command::new("sh");
            command.args(["-c", "printf idle-ready; exec sleep 60"]);
            Ok(command)
        }))
        .unwrap();
    let ready = caller
        .block_on(async {
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    let value = host
                        .wait(&scope, &started.process_id, 0, Duration::from_millis(100))
                        .await
                        .unwrap();
                    if value.output.contains("idle-ready") {
                        break value;
                    }
                }
            })
            .await
        })
        .unwrap();
    assert_eq!(ready.state, ShellProcessState::Running);
    let (tx, rx) = mpsc::channel();
    let (abort_tx, abort_rx) = tokio::sync::watch::channel(false);
    let result = super::collect_llm_step_from_stream(
        rx,
        &mut |_| true,
        &abort_tx,
        Duration::from_millis(10),
        None,
    );
    drop(tx);
    let after = host.snapshot(&scope, &started.process_id).unwrap();
    host.shutdown().unwrap();
    assert_eq!(result.unwrap_err(), "模型响应超时");
    assert!(*abort_rx.borrow(), "only the model stream is aborted");
    assert_eq!(after.state, ShellProcessState::Running);
    assert_eq!(
        host.snapshot(&scope, &started.process_id).unwrap().state,
        ShellProcessState::Cancelled
    );
    eprintln!(
        "model idle expired; process={} remained Running; shutdown reaped it",
        after.process_id
    );
}

fn recovery_api_error(message: &str, is_retryable: bool) -> AssistantError {
    AssistantError::APIError {
        message: message.into(),
        status_code: None,
        is_retryable,
        response_headers: None,
        response_body: None,
        metadata: None,
    }
}

#[test]
fn compaction_range_error_requires_oversized_nonempty_input_and_safe_replay() {
    let error = serde_json::to_string(&recovery_api_error(
        "HTTP 400: Range of input length should be [1, 1000000]",
        false,
    ))
    .unwrap();
    let large = vec![serde_json::json!({"role":"user","content":"x".repeat(1_000_001)})];
    assert!(super::is_request_too_long_error(&error, &large));
    for input in [
        vec![],
        vec![serde_json::json!({"role":"user","content":""})],
        vec![serde_json::json!({"role":"user","content":"short"})],
    ] {
        assert!(!super::is_request_too_long_error(&error, &input));
    }
    let mut with_empty = large.clone();
    with_empty.push(serde_json::json!({"role":"user","content":""}));
    assert!(!super::is_request_too_long_error(&error, &with_empty));
    let mut unsafe_error: AssistantError = serde_json::from_str(&error).unwrap();
    crate::session::llm::prevent_error_replay(&mut unsafe_error);
    assert!(!super::is_request_too_long_error(
        &serde_json::to_string(&unsafe_error).unwrap(),
        &large
    ));
    assert!(!super::is_request_too_long_error("HTTP 400: invalid parameter", &large));
}

#[test]
fn compaction_range_error_after_partial_output_never_replays() {
    let _guard = test_llm_stream_guard();
    enqueue_test_llm_stream(vec![
        llm::StreamEvent::Delta("partial".into()),
        llm::StreamEvent::Error(recovery_api_error(
            "Range of input length should be [1, 1000]",
            false,
        )),
    ]);
    let input = vec![serde_json::json!({"role":"user","content":"x".repeat(2000)})];
    let error = super::run_llm_step_with_retry(
        "partial-range",
        &input,
        &[],
        None,
        &serde_json::json!({}),
        &[],
        &HashSet::new(),
        &mut |_| true,
        2,
        0,
        None,
    )
    .unwrap_err();
    assert!(!super::is_request_too_long_error(&error, &input));
    assert_eq!(take_test_llm_messages("partial-range").len(), 1);
}

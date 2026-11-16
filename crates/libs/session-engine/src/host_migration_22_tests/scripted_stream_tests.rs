use crate::llm_stream as llm;
use message_types::AssistantError;
use tool_types::ToolSpec;
use std::{collections::{HashSet, VecDeque}, sync::{LazyLock, Mutex}};
static STREAMS: LazyLock<Mutex<VecDeque<Vec<llm::StreamEvent>>>> = LazyLock::new(Default::default);
static TOOL_IDS: LazyLock<Mutex<Vec<Vec<String>>>> = LazyLock::new(Default::default);
static REQUESTS: LazyLock<Mutex<Vec<String>>> = LazyLock::new(Default::default);
static LOCK: Mutex<()> = Mutex::new(());
fn test_llm_stream_guard() -> std::sync::MutexGuard<'static, ()> {
    let guard = LOCK.lock().unwrap();
    STREAMS.lock().unwrap().clear(); TOOL_IDS.lock().unwrap().clear(); REQUESTS.lock().unwrap().clear(); guard
}
fn enqueue_test_llm_stream(events: Vec<llm::StreamEvent>) { STREAMS.lock().unwrap().push_back(events); }
fn take_test_llm_stream_tool_ids() -> Vec<Vec<String>> { std::mem::take(&mut *TOOL_IDS.lock().unwrap()) }
fn take_test_llm_messages(session: &str) -> Vec<String> { REQUESTS.lock().unwrap().iter().filter(|id| id.as_str() == session).cloned().collect() }
fn usage(input: i64, output: i64) -> session_port::ui_types::TokenUsage {
    session_port::ui_types::TokenUsage { input_tokens: input, output_tokens: output, ..Default::default() }
}
struct ScriptedStreamHost;
impl crate::llm_runner::LlmStreamHost for ScriptedStreamHost {
    fn spawn(session: String, _: Vec<serde_json::Value>, _: Vec<String>, _: Option<String>, _: serde_json::Value, tools: std::collections::HashMap<String, ToolSpec>, mut abort_rx: tokio::sync::watch::Receiver<bool>, tx: std::sync::mpsc::Sender<llm::StreamEvent>) -> std::thread::JoinHandle<()> {
        REQUESTS.lock().unwrap().push(session);
        let mut ids = tools.keys().cloned().collect::<Vec<_>>(); ids.sort(); TOOL_IDS.lock().unwrap().push(ids);
        let events = STREAMS.lock().unwrap().pop_front().unwrap_or_else(|| vec![llm::StreamEvent::Error(AssistantError::Unknown { message: "missing scripted LLM stream".into() })]);
        std::thread::spawn(move || { for event in events { if *abort_rx.borrow_and_update() { break; } let _ = tx.send(event); } })
    }
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

    let step = crate::llm_runner::run_llm_step::<ScriptedStreamHost>(
        "session-1",
        &[serde_json::json!({ "role": "user", "content": "hello" })],
        &["system".to_string()],
        Some("provider/model".to_string()),
        &serde_json::json!({}),
        &available_tools,
        &allowed_tools,
        &mut |event| {
            if let crate::processor_events::StreamEvent::Delta(text) = event {
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

    let error = crate::llm_runner::run_llm_step::<ScriptedStreamHost>(
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

#[test]
fn compaction_range_error_requires_oversized_nonempty_input_and_safe_replay() {
    let error = serde_json::to_string(&recovery_api_error(
        "HTTP 400: Range of input length should be [1, 1000000]",
        false,
    ))
    .unwrap();
    let large = vec![serde_json::json!({"role":"user","content":"x".repeat(1_000_001)})];
    assert!(crate::llm_runner::is_request_too_long_error(&error, &large));
    for input in [
        vec![],
        vec![serde_json::json!({"role":"user","content":""})],
        vec![serde_json::json!({"role":"user","content":"short"})],
    ] {
        assert!(!crate::llm_runner::is_request_too_long_error(&error, &input));
    }
    let mut with_empty = large.clone();
    with_empty.push(serde_json::json!({"role":"user","content":""}));
    assert!(!crate::llm_runner::is_request_too_long_error(&error, &with_empty));
    let mut unsafe_error: AssistantError = serde_json::from_str(&error).unwrap();
    llm_runtime::replay_safety::prevent_error_replay(&mut unsafe_error);
    assert!(!crate::llm_runner::is_request_too_long_error(
        &serde_json::to_string(&unsafe_error).unwrap(),
        &large
    ));
    assert!(!crate::llm_runner::is_request_too_long_error("HTTP 400: invalid parameter", &large));
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
    let error = crate::llm_runner::run_llm_step_with_retry::<ScriptedStreamHost>(
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
    assert!(!crate::llm_runner::is_request_too_long_error(&error, &input));
    assert_eq!(take_test_llm_messages("partial-range").len(), 1);
}

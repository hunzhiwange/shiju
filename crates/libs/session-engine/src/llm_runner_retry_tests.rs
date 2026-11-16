//! Scripted model recovery contracts; no tenant, provider, network or environment state.
use super as runner;
use super::StreamEvent as CollectorStreamEvent;
use crate::llm_stream as llm;
use message_types::AssistantError;
use std::collections::{HashSet, VecDeque};
use std::sync::{LazyLock, Mutex};
use tool_types::ToolSpec;

#[path = "llm_runner_test_support.rs"]
mod support;
use support::usage;

static STREAMS: LazyLock<Mutex<VecDeque<Vec<llm::StreamEvent>>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));
static TOOL_IDS: LazyLock<Mutex<Vec<Vec<String>>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
static STREAM_LOCK: Mutex<()> = Mutex::new(());

fn test_llm_stream_guard() -> std::sync::MutexGuard<'static, ()> {
    let guard = STREAM_LOCK.lock().expect("lock scripted recovery tests");
    STREAMS.lock().unwrap().clear();
    TOOL_IDS.lock().unwrap().clear();
    guard
}

fn enqueue_test_llm_stream(events: Vec<llm::StreamEvent>) {
    STREAMS.lock().unwrap().push_back(events);
}

fn take_test_llm_stream_tool_ids() -> Vec<Vec<String>> {
    std::mem::take(&mut *TOOL_IDS.lock().unwrap())
}

struct ScriptedStreamHost;
impl super::LlmStreamHost for ScriptedStreamHost {
    fn spawn(
        _session_id: String,
        _messages: Vec<serde_json::Value>,
        _system: Vec<String>,
        _model: Option<String>,
        _options: serde_json::Value,
        tools: std::collections::HashMap<String, ToolSpec>,
        mut abort_rx: tokio::sync::watch::Receiver<bool>,
        tx: std::sync::mpsc::Sender<llm::StreamEvent>,
    ) -> std::thread::JoinHandle<()> {
        let mut tool_ids = tools.keys().cloned().collect::<Vec<_>>();
        tool_ids.sort();
        TOOL_IDS.lock().unwrap().push(tool_ids);
        let events = STREAMS.lock().unwrap().pop_front().expect("scripted recovery stream");
        std::thread::spawn(move || {
            for event in events {
                if *abort_rx.borrow_and_update() {
                    break;
                }
                let _ = tx.send(event);
            }
        })
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

fn model_recovery_states(
    events: Vec<super::StreamEvent>,
) -> Vec<session_port::ModelRecovery> {
    events
        .into_iter()
        .filter_map(|event| match event {
            super::StreamEvent::ModelRecovery(state) => Some(state),
            _ => None,
        })
        .collect()
}

#[test]
fn run_llm_step_with_retry_retries_retryable_error_then_succeeds() {
    let _guard = test_llm_stream_guard();
    enqueue_test_llm_stream(vec![llm::StreamEvent::Error(recovery_api_error(
        "temporary timeout",
        true,
    ))]);
    enqueue_test_llm_stream(vec![
        llm::StreamEvent::Delta("second attempt".to_string()),
        llm::StreamEvent::Done { usage: usage(3, 4), finish_reason: Some("stop".to_string()) },
    ]);
    let mut deltas = Vec::new();

    let step = super::run_llm_step_with_retry::<ScriptedStreamHost>(
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
        1,
        0,
        None,
    )
    .expect("second scripted attempt should succeed");

    assert_eq!(step.text, "second attempt");
    assert_eq!(step.usage.input_tokens, 3);
    assert_eq!(deltas, vec!["second attempt"]);
    assert_eq!(take_test_llm_stream_tool_ids().len(), 2);
}

#[test]
fn configured_provider_retries_control_total_attempts() {
    let _guard = test_llm_stream_guard();
    for _ in 0..3 {
        enqueue_test_llm_stream(vec![llm::StreamEvent::Error(recovery_api_error(
            "temporary timeout",
            true,
        ))]);
    }

    let error = super::run_llm_step_with_retry::<ScriptedStreamHost>(
        "session-configured-retries",
        &[],
        &[],
        None,
        &serde_json::json!({}),
        &[],
        &HashSet::new(),
        &mut |_event| true,
        2,
        0,
        None,
    )
    .unwrap_err();

    assert!(error.contains("temporary timeout"));
    assert_eq!(take_test_llm_stream_tool_ids().len(), 3);
}

#[test]
fn run_llm_step_with_retry_stops_on_non_retryable_error() {
    let _guard = test_llm_stream_guard();
    enqueue_test_llm_stream(vec![llm::StreamEvent::Error(AssistantError::Unknown {
        message: "model not found".to_string(),
    })]);

    let error = super::run_llm_step_with_retry::<ScriptedStreamHost>(
        "session-non-retry",
        &[],
        &[],
        None,
        &serde_json::json!({}),
        &[],
        &HashSet::new(),
        &mut |_event| true,
        2,
        0,
        None,
    )
    .unwrap_err();

    assert!(error.contains("model not found"));
    assert_eq!(take_test_llm_stream_tool_ids().len(), 1);
}

#[test]
fn run_llm_step_with_retry_preserves_length_error_without_repeating_partial_output() {
    let _guard = test_llm_stream_guard();
    let error = AssistantError::APIError {
        message: "max_tokens".into(),
        status_code: None,
        is_retryable: false,
        response_headers: None,
        response_body: Some("max_tokens".into()),
        metadata: Some(std::collections::HashMap::from([
            ("stream_failure_kind".into(), "incomplete".into()),
            ("source".into(), "aisdk".into()),
        ])),
    };
    for _ in 0..4 {
        enqueue_test_llm_stream(vec![
            llm::StreamEvent::ReasoningDelta("partial reasoning".into()),
            llm::StreamEvent::Error(error.clone()),
        ]);
    }
    let mut deltas = Vec::new();
    let result = super::run_llm_step_with_retry::<ScriptedStreamHost>(
        "session-length-error",
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
        3,
        0,
        None,
    );
    let actual = result.unwrap_err();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&actual).unwrap(),
        serde_json::to_value(error).unwrap()
    );
    assert_eq!(take_test_llm_stream_tool_ids().len(), 1);
    assert_eq!(deltas.join(""), "<think>partial reasoning</think>\n\n");
}

#[test]
fn run_llm_step_with_retry_treats_zero_retries_as_one_attempt() {
    let _guard = test_llm_stream_guard();
    enqueue_test_llm_stream(vec![llm::StreamEvent::Error(recovery_api_error(
        "temporary timeout",
        true,
    ))]);

    let error = super::run_llm_step_with_retry::<ScriptedStreamHost>(
        "session-one-attempt",
        &[],
        &[],
        None,
        &serde_json::json!({}),
        &[],
        &HashSet::new(),
        &mut |_event| true,
        0,
        0,
        None,
    )
    .unwrap_err();

    assert!(error.contains("temporary timeout"));
    assert_eq!(take_test_llm_stream_tool_ids().len(), 1);
}

#[test]
fn model_recovery_structured_transient_overrides_model_not_found_text() {
    let _guard = test_llm_stream_guard();
    enqueue_test_llm_stream(vec![llm::StreamEvent::Error(recovery_api_error(
        "model not found during temporary routing failure",
        true,
    ))]);
    enqueue_test_llm_stream(vec![
        llm::StreamEvent::Delta("recovered".into()),
        llm::StreamEvent::Done { usage: Default::default(), finish_reason: None },
    ]);
    let result = super::run_llm_step_with_retry::<ScriptedStreamHost>(
        "recovery",
        &[],
        &[],
        None,
        &serde_json::json!({}),
        &[],
        &HashSet::new(),
        &mut |_| true,
        1,
        0,
        None,
    );
    assert!(result.is_ok());
    assert_eq!(take_test_llm_stream_tool_ids().len(), 2);
}

#[test]
fn model_recovery_unknown_error_does_not_retry() {
    let _guard = test_llm_stream_guard();
    for _ in 0..3 {
        enqueue_test_llm_stream(vec![llm::StreamEvent::Error(AssistantError::Unknown {
            message: "unclassified failure".into(),
        })]);
    }
    let result = super::run_llm_step_with_retry::<ScriptedStreamHost>(
        "recovery",
        &[],
        &[],
        None,
        &serde_json::json!({}),
        &[],
        &HashSet::new(),
        &mut |_| true,
        2,
        0,
        None,
    );
    assert!(result.is_err());
    assert_eq!(take_test_llm_stream_tool_ids().len(), 1);
}

#[test]
fn model_recovery_partial_text_is_not_replayed() {
    let _guard = test_llm_stream_guard();
    for _ in 0..3 {
        enqueue_test_llm_stream(vec![
            llm::StreamEvent::Delta("partial".into()),
            llm::StreamEvent::Error(recovery_api_error("temporarily unavailable", true)),
        ]);
    }
    let mut text = String::new();
    let result = super::run_llm_step_with_retry::<ScriptedStreamHost>(
        "recovery",
        &[],
        &[],
        None,
        &serde_json::json!({}),
        &[],
        &HashSet::new(),
        &mut |event| {
            if let super::StreamEvent::Delta(value) = event {
                text.push_str(&value);
            }
            true
        },
        2,
        0,
        None,
    );
    assert!(result.is_err());
    assert_eq!(take_test_llm_stream_tool_ids().len(), 1);
    assert_eq!(text, "partial");
}

#[test]
fn model_recovery_reports_wait_and_success_without_terminal_error() {
    use session_port::RecoveryPhase;
    let _guard = test_llm_stream_guard();
    enqueue_test_llm_stream(vec![llm::StreamEvent::Error(recovery_api_error(
        "temporary failure",
        true,
    ))]);
    enqueue_test_llm_stream(vec![
        llm::StreamEvent::Delta("ok".into()),
        llm::StreamEvent::Done { usage: Default::default(), finish_reason: None },
    ]);
    let mut events = Vec::new();
    assert!(
        super::run_llm_step_with_retry::<ScriptedStreamHost>(
            "recovery",
            &[],
            &[],
            None,
            &serde_json::json!({}),
            &[],
            &HashSet::new(),
            &mut |event| {
                events.push(event);
                true
            },
            1,
            0,
            None
        )
        .is_ok()
    );
    assert!(!events.iter().any(|event| matches!(event, super::StreamEvent::Error(_))));
    let states = model_recovery_states(events);
    assert_eq!(
        states.iter().map(|state| state.phase).collect::<Vec<_>>(),
        [RecoveryPhase::Waiting, RecoveryPhase::Recovered]
    );
    assert_eq!((states[0].attempt, states[1].attempt, states[0].max_attempts), (1, 2, 2));
    assert_eq!(take_test_llm_stream_tool_ids().len(), 2);
}

#[test]
fn model_recovery_exhaustion_and_zero_retries_are_bounded() {
    use session_port::RecoveryPhase;
    let _guard = test_llm_stream_guard();
    for retries in [0, 2] {
        for _ in 0..=retries {
            enqueue_test_llm_stream(vec![llm::StreamEvent::Error(recovery_api_error(
                "temporary failure",
                true,
            ))]);
        }
        let mut events = Vec::new();
        assert!(
            super::run_llm_step_with_retry::<ScriptedStreamHost>(
                "recovery",
                &[],
                &[],
                None,
                &serde_json::json!({}),
                &[],
                &HashSet::new(),
                &mut |event| {
                    events.push(event);
                    true
                },
                retries,
                0,
                None
            )
            .is_err()
        );
        assert_eq!(take_test_llm_stream_tool_ids().len(), retries as usize + 1);
        let states = model_recovery_states(events);
        assert_eq!(states.len(), retries as usize + 1);
        let last = states.last().unwrap();
        assert_eq!(last.phase, RecoveryPhase::Stopped);
        assert_eq!(last.attempt, u64::from(retries) + 1);
        assert!(last.next_action.contains("预算已耗尽"));
    }
}

#[test]
fn model_recovery_permanent_errors_make_one_request() {
    let _guard = test_llm_stream_guard();
    for error in [
        recovery_api_error("model not found", false),
        recovery_api_error("disabled model", false),
        recovery_api_error("invalid configuration", false),
        AssistantError::ProviderAuthError {
            provider_id: "fixture".into(),
            message: "authentication failed".into(),
        },
    ] {
        enqueue_test_llm_stream(vec![llm::StreamEvent::Error(error)]);
        assert!(
            super::run_llm_step_with_retry::<ScriptedStreamHost>(
                "recovery",
                &[],
                &[],
                None,
                &serde_json::json!({}),
                &[],
                &HashSet::new(),
                &mut |_| true,
                3,
                0,
                None
            )
            .is_err()
        );
        assert_eq!(take_test_llm_stream_tool_ids().len(), 1);
    }
}

#[test]
fn model_recovery_backoff_cancellation_and_closed_consumer_stop_requests() {
    use session_port::RecoveryPhase;
    let _guard = test_llm_stream_guard();
    for close_consumer in [false, true] {
        enqueue_test_llm_stream(vec![llm::StreamEvent::Error(recovery_api_error(
            "temporary failure",
            true,
        ))]);
        let token = tokio_util::sync::CancellationToken::new();
        let mut events = Vec::new();
        let error = super::run_llm_step_with_retry::<ScriptedStreamHost>("recovery", &[], &[], None, &serde_json::json!({}), &[], &HashSet::new(), &mut |event| {
            let waiting = matches!(&event, super::StreamEvent::ModelRecovery(state) if state.phase == RecoveryPhase::Waiting);
            events.push(event);
            if waiting { token.cancel(); }
            !(waiting && close_consumer)
        }, 2, 5_000, Some(&token)).unwrap_err();
        assert_eq!(take_test_llm_stream_tool_ids().len(), 1);
        assert_eq!(error, if close_consumer { "stream consumer stopped" } else { "执行已取消" });
        let states = model_recovery_states(events);
        assert_eq!(states[0].wait_ms, 5_000);
        if !close_consumer {
            assert_eq!(states.last().unwrap().phase, RecoveryPhase::Cancelled);
            assert_eq!(states.last().unwrap().attempt, 1);
        }
    }
}

#[test]
fn model_recovery_tool_calls_and_remote_side_effects_are_not_replayed() {
    let _guard = test_llm_stream_guard();
    for tool_event in [
        llm::StreamEvent::ToolCalls(vec![llm::ToolCall {
            kind: Default::default(),
            namespace: None,
            id: "call-1".into(),
            name: "shell".into(),
            arguments: "{}".into(),
        }]),
        llm::StreamEvent::RemoteToolUpdate {
            call_id: "call-1".into(),
            name: "shell".into(),
            is_initial: true,
            payload: serde_json::json!({"status":"running"}),
        },
    ] {
        enqueue_test_llm_stream(vec![
            tool_event,
            llm::StreamEvent::Error(recovery_api_error("temporary failure", true)),
        ]);
        assert!(
            super::run_llm_step_with_retry::<ScriptedStreamHost>(
                "recovery",
                &[],
                &[],
                None,
                &serde_json::json!({}),
                &[],
                &HashSet::new(),
                &mut |_| true,
                2,
                0,
                None
            )
            .is_err()
        );
        assert_eq!(take_test_llm_stream_tool_ids().len(), 1);
    }
}

#[test]
fn model_recovery_partial_output_blocks_outer_empty_and_compaction_replay() {
    let _guard = test_llm_stream_guard();
    for failure in [
        vec![
            llm::StreamEvent::ReasoningDelta("partial thinking".into()),
            llm::StreamEvent::Done { usage: Default::default(), finish_reason: None },
        ],
        vec![
            llm::StreamEvent::Delta("partial text".into()),
            llm::StreamEvent::Error(recovery_api_error("context length exceeded", false)),
        ],
        vec![
            llm::StreamEvent::Delta("partial text".into()),
            llm::StreamEvent::Error(AssistantError::ContextOverflowError {
                message: "context length exceeded".into(),
                response_body: None,
            }),
        ],
    ] {
        enqueue_test_llm_stream(failure);
        let error = super::run_llm_step_with_retry::<ScriptedStreamHost>(
            "recovery",
            &[],
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
        assert_ne!(error, "模型未返回内容");
        assert!(!super::is_prompt_too_long_error(&error));
        assert!(!super::llm_step_retry_allowed(&error, true));
        assert_eq!(take_test_llm_stream_tool_ids().len(), 1);
    }
}

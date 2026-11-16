use crate::{llm_stream as llm, compaction_summary::{request_input, MAX_OVERFLOW_RECOVERIES}};
use agent_config::runtime::ReliabilityConfig;
use model_types::ModelLimit;
use serde_json::{Value,json};
use tokio_util::sync::CancellationToken;
use std::{collections::{VecDeque,HashSet,HashMap},sync::{LazyLock,Mutex}};
use tool_types::ToolSpec;
static LOCK: Mutex<()> = Mutex::new(());
static STREAMS: LazyLock<Mutex<VecDeque<Vec<llm::StreamEvent>>>> = LazyLock::new(Default::default);
struct Request { session: String, messages: Vec<Value>, options: Value }
static REQUESTS: Mutex<Vec<Request>> = Mutex::new(Vec::new());
fn test_llm_stream_guard() -> std::sync::MutexGuard<'static,()> {
    let guard=LOCK.lock().unwrap(); STREAMS.lock().unwrap().clear(); REQUESTS.lock().unwrap().clear(); guard
}
fn enqueue_test_llm_stream(events: Vec<llm::StreamEvent>) { STREAMS.lock().unwrap().push_back(events); }
fn take_test_llm_messages(session: &str) -> Vec<Request> {
    let mut requests=REQUESTS.lock().unwrap(); let mut selected=Vec::new(); let mut other=Vec::new();
    for request in requests.drain(..) { if request.session == session { selected.push(request); } else { other.push(request); } }
    *requests=other; selected
}
struct ModelHost;
impl crate::llm_runner::LlmStreamHost for ModelHost {
    fn spawn(session: String, messages: Vec<Value>, _: Vec<String>, _: Option<String>, options: Value,
        _: HashMap<String,ToolSpec>, mut abort: tokio::sync::watch::Receiver<bool>, tx: std::sync::mpsc::Sender<llm::StreamEvent>) -> std::thread::JoinHandle<()> {
        REQUESTS.lock().unwrap().push(Request { session,messages,options });
        let events=STREAMS.lock().unwrap().pop_front().expect("scripted model response");
        std::thread::spawn(move || { for event in events { if *abort.borrow_and_update() { break; } let _=tx.send(event); } })
    }
}
impl crate::compaction_summary::SummaryHost for ModelHost {
    fn run_llm_step(session: &str, messages: &[Value], system: &[String], model: Option<String>, options: &Value,
        tools: &[ToolSpec], allowed: &HashSet<String>, on_event: &mut dyn FnMut(crate::processor_events::StreamEvent)->bool,
        cancellation: Option<&CancellationToken>) -> Result<crate::llm_step::LlmStep,String> {
        crate::llm_runner::run_llm_step::<Self>(session,messages,system,model,options,tools,allowed,on_event,cancellation)
    }
}
fn summarize(session: &str, history: &[Value], model: &str, reliability: &ReliabilityConfig, cancellation: Option<&CancellationToken>, limit: &ModelLimit) -> Result<String,String> {
    crate::compaction_summary::summarize::<ModelHost>(session,history,model,reliability,cancellation,limit)
}
fn response(text: &str) -> Vec<llm::StreamEvent> { vec![llm::StreamEvent::Delta(text.into()),llm::StreamEvent::Done { finish_reason: Some("stop".into()),usage:Default::default() }] }
fn reliability() -> ReliabilityConfig { ReliabilityConfig { provider_retries:0,provider_backoff_ms:0,..Default::default() } }
fn model_limit(context:u64,input:u64,output:u64)->ModelLimit { ModelLimit { context,input:Some(input),output } }
fn overflow_error(message:&str)->llm::StreamEvent { llm::StreamEvent::Error(message_types::AssistantError::APIError { message:message.into(),status_code:Some(400),is_retryable:false,response_headers:None,response_body:None,metadata:None }) }
#[test]
fn compaction_single_request_preserves_structured_history() {
    let _guard = test_llm_stream_guard();
    enqueue_test_llm_stream(response("checkpoint"));
    let history = vec![
        json!({"role":"user","content":"中文😀".repeat(50_000)}),
        json!({"role":"assistant","tool_calls":[{"id":"call","type":"function","function":{"name":"read","arguments":"{}"}}]}),
        json!({"role":"tool","tool_call_id":"call","content":"important result"}),
    ];
    assert_eq!(
        summarize(
            "single-summary",
            &history,
            "test/model",
            &reliability(),
            None,
            &model_state::resolve_model_limit(None, None, None)
        )
        .unwrap(),
        "checkpoint"
    );
    let requests = take_test_llm_messages("single-summary");
    assert_eq!(requests.len(), 1);
    assert_eq!(&requests[0].messages[..history.len()], history);
}

#[test]
fn compaction_overflow_trims_request_copy_and_keeps_tool_pairs() {
    let _guard = test_llm_stream_guard();
    enqueue_test_llm_stream(vec![llm::StreamEvent::Error(
        message_types::AssistantError::Unknown {
            message: "context window exceeded".into(),
        },
    )]);
    enqueue_test_llm_stream(response("checkpoint"));
    let history = vec![
        json!({"role":"assistant","tool_calls":[{"id":"call"}]}),
        json!({"role":"tool","tool_call_id":"call","content":"old"}),
        json!({"role":"user","content":"recent"}),
    ];
    let before = history.clone();
    summarize(
        "overflow-summary",
        &history,
        "test/model",
        &reliability(),
        None,
        &model_state::resolve_model_limit(None, None, None),
    )
    .unwrap();
    let requests = take_test_llm_messages("overflow-summary");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].messages[0], history[2]);
    assert_eq!(history, before);
}

#[test]
fn compaction_reuses_cancellation_and_retry() {
    let _guard = test_llm_stream_guard();
    let token = CancellationToken::new();
    token.cancel();
    assert!(
        summarize(
            "cancel-summary",
            &[json!({"role":"user","content":"history"})],
            "test/model",
            &reliability(),
            Some(&token),
            &model_state::resolve_model_limit(None, None, None)
        )
        .is_err()
    );
    assert!(take_test_llm_messages("cancel-summary").is_empty());
    enqueue_test_llm_stream(vec![llm::StreamEvent::Error(
        message_types::AssistantError::APIError {
            message: "temporary connection failure".into(),
            status_code: Some(503),
            is_retryable: true,
            response_headers: None,
            response_body: None,
            metadata: None,
        },
    )]);
    enqueue_test_llm_stream(response("recovered"));
    let reliability = ReliabilityConfig { provider_retries: 1, ..reliability() };
    assert_eq!(
        summarize(
            "retry-summary",
            &[json!({"role":"user","content":"history"})],
            "test/model",
            &reliability,
            None,
            &model_state::resolve_model_limit(None, None, None)
        )
        .unwrap(),
        "recovered"
    );
    assert_eq!(take_test_llm_messages("retry-summary").len(), 2);
}

#[test]
fn compaction_preflight_bounds_large_tool_output_without_mutating_source() {
    let _guard = test_llm_stream_guard();
    enqueue_test_llm_stream(response("checkpoint"));
    let history = vec![
        json!({"role":"user","content":"Preserve the user's goal"}),
        json!({"role":"assistant","tool_calls":[{"id":"call","type":"function","function":{"name":"read","arguments":"{}"}}]}),
        json!({"role":"tool","tool_call_id":"call","content":format!("HEAD{}TAIL", "中文😀".repeat(10_000))}),
    ];
    let original = history.clone();
    let limit = model_limit(4_000, 4_000, 1_000);
    let summary =
        summarize("preflight-summary", &history, "test/model", &reliability(), None, &limit)
            .unwrap();
    let requests = take_test_llm_messages("preflight-summary");
    assert_eq!(requests.len(), 1);
    assert!(
        request_input::estimated_tokens(&requests[0].messages)
            <= request_input::input_budget(&limit)
    );
    let output = &requests[0].messages[2];
    assert_eq!(output["tool_call_id"], "call");
    assert!(output["content"].as_str().unwrap().starts_with("HEAD"));
    assert!(output["content"].as_str().unwrap().ends_with("TAIL"));
    assert_eq!(requests[0].messages[1], history[1]);
    assert_eq!(history, original);
    assert!(summary.contains("原始内容仍保留"));
}

#[test]
fn compaction_range_overflow_recovers_single_large_message() {
    let _guard = test_llm_stream_guard();
    enqueue_test_llm_stream(vec![overflow_error(
        "HTTP 400: {\"error\":{\"code\":\"invalid_parameter_error\",\"message\":\"Range of input length should be [1, 1000000]\"}}",
    )]);
    enqueue_test_llm_stream(response("checkpoint"));
    let history =
        vec![json!({"role":"user","content":format!("HEAD{}TAIL", "x".repeat(1_100_000))})];
    let before = history.clone();
    let limit = model_limit(2_000_000, 2_000_000, 4_096);
    let summary =
        summarize("range-summary", &history, "test/model", &reliability(), None, &limit).unwrap();
    let requests = take_test_llm_messages("range-summary");
    assert_eq!(requests.len(), 2);
    assert!(
        request_input::estimated_tokens(&requests[1].messages)
            < request_input::estimated_tokens(&requests[0].messages)
    );
    assert!(requests[1].messages[0]["content"].as_str().unwrap().ends_with("TAIL"));
    assert_eq!(history, before);
    assert!(summary.contains("checkpoint"));
}

#[test]
fn compaction_overflow_retries_are_bounded_and_preserve_source() {
    let _guard = test_llm_stream_guard();
    for _ in 0..=MAX_OVERFLOW_RECOVERIES {
        enqueue_test_llm_stream(vec![overflow_error("context_length_exceeded")]);
    }
    let history = vec![json!({"role":"user","content":"x".repeat(1_000_000)})];
    let before = history.clone();
    let result = summarize(
        "bounded-summary",
        &history,
        "test/model",
        &reliability(),
        None,
        &model_limit(2_000_000, 2_000_000, 4096),
    );
    assert!(result.unwrap_err().contains("恢复次数已用尽"));
    let requests = take_test_llm_messages("bounded-summary");
    assert_eq!(requests.len(), MAX_OVERFLOW_RECOVERIES + 1);
    for pair in requests.windows(2) {
        assert!(
            request_input::estimated_tokens(&pair[1].messages)
                < request_input::estimated_tokens(&pair[0].messages)
        );
    }
    assert_eq!(history, before);
}

#[test]
fn compaction_impossible_budget_preserves_system_and_skips_model() {
    let _guard = test_llm_stream_guard();
    let history = vec![
        json!({"role":"system","content":"rules".repeat(1000)}),
        json!({"role":"user","content":"goal"}),
    ];
    let before = history.clone();
    let result = summarize(
        "impossible-summary",
        &history,
        "test/model",
        &reliability(),
        None,
        &model_limit(100, 100, 50),
    );
    assert!(result.unwrap_err().contains("已有检查点已保留"));
    assert!(take_test_llm_messages("impossible-summary").is_empty());
    assert_eq!(history, before);
}

#[test]
fn compaction_equal_context_and_output_uses_bounded_request_allocation() {
    let _guard = test_llm_stream_guard();
    enqueue_test_llm_stream(response("checkpoint"));
    let limit = ModelLimit { context: 1_048_576, input: None, output: 1_048_576 };
    let history = vec![json!({"role":"user", "content":"Continue the task"})];
    assert_eq!(
        summarize("kimi-summary", &history, "shiju/kimi-k3", &reliability(), None, &limit).unwrap(),
        "checkpoint"
    );
    let requests = take_test_llm_messages("kimi-summary");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].options["max_output_tokens"], 20_000);
    assert_eq!(requests[0].options["disable_tools"], true);
    assert_eq!(request_input::input_budget(&limit), 977_147);
    assert_eq!(&requests[0].messages[..history.len()], history);
}

#[test]
fn compaction_unknown_window_recovers_from_provider_overflow_without_changing_history() {
    let _guard = test_llm_stream_guard();
    enqueue_test_llm_stream(vec![overflow_error("context_length_exceeded")]);
    enqueue_test_llm_stream(response("checkpoint"));
    let limit = ModelLimit { context: 0, input: None, output: 0 };
    let history = vec![json!({"role":"user", "content":"x".repeat(1_000_000)})];
    let before = history.clone();
    summarize("unknown-summary", &history, "custom/model", &reliability(), None, &limit).unwrap();
    let requests = take_test_llm_messages("unknown-summary");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].options["max_output_tokens"], 4096);
    assert_eq!(requests[1].options, requests[0].options);
    assert!(
        request_input::estimated_tokens(&requests[0].messages)
            <= request_input::input_budget(&limit)
    );
    assert!(
        request_input::estimated_tokens(&requests[1].messages)
            < request_input::estimated_tokens(&requests[0].messages)
    );
    assert_eq!(history, before);
    assert_eq!(request_input::input_budget(&limit), 186_108);
    assert_eq!(request_input::input_budget(&ModelLimit { input: Some(0), ..limit }), 186_108);
}

#[test]
fn compaction_empty_output_is_normalized_without_replaying_partial_streams() {
    let _guard = test_llm_stream_guard();
    for partial in [
        None,
        Some(llm::StreamEvent::Delta(" \n\t".into())),
        Some(llm::StreamEvent::ReasoningDelta("thinking".into())),
    ] {
        let streamed = partial.is_some();
        let mut events = partial.into_iter().collect::<Vec<_>>();
        events.push(llm::StreamEvent::Done {
            usage: Default::default(),
            finish_reason: Some("stop".into()),
        });
        enqueue_test_llm_stream(events);
        let error = summarize(
            "empty-summary",
            &[json!({"role":"user","content":"history"})],
            "test/model",
            &ReliabilityConfig { provider_retries: 3, ..reliability() },
            None,
            &model_limit(10_000, 10_000, 1000),
        )
        .unwrap_err();
        let structured = serde_json::from_str::<Value>(&error).ok();
        let message =
            structured.as_ref().and_then(|value| value["message"].as_str()).unwrap_or(&error);
        assert_eq!(message, "internal compaction generated an empty summary");
        if streamed {
            let structured = structured.expect("streamed output must retain its replay guard");
            assert_eq!(structured["metadata"]["replay_safe"], "false");
            assert_eq!(structured["is_retryable"], false);
        }
        assert_eq!(take_test_llm_messages("empty-summary").len(), 1);
    }
}

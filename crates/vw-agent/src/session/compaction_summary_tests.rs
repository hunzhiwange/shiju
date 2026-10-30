use super::*;
use crate::session::{llm, processor};

fn response(text: &str) -> Vec<llm::StreamEvent> {
    vec![
        llm::StreamEvent::Delta(text.into()),
        llm::StreamEvent::Done { finish_reason: Some("stop".into()), usage: Default::default() },
    ]
}
fn reliability() -> ReliabilityConfig {
    ReliabilityConfig { provider_retries: 0, provider_backoff_ms: 0, ..Default::default() }
}

#[test]
fn compaction_single_request_preserves_structured_history() {
    let _guard = processor::test_llm_stream_guard();
    processor::enqueue_test_llm_stream(response("checkpoint"));
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
            &crate::provider::provider::resolve_model_limit(None, None, None)
        )
        .unwrap(),
        "checkpoint"
    );
    let requests = processor::take_test_llm_messages("single-summary");
    assert_eq!(requests.len(), 1);
    assert_eq!(&requests[0].messages[..history.len()], history);
}

#[test]
fn compaction_overflow_trims_request_copy_and_keeps_tool_pairs() {
    let _guard = processor::test_llm_stream_guard();
    processor::enqueue_test_llm_stream(vec![llm::StreamEvent::Error(
        crate::session::message::AssistantError::Unknown {
            message: "context window exceeded".into(),
        },
    )]);
    processor::enqueue_test_llm_stream(response("checkpoint"));
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
        &crate::provider::provider::resolve_model_limit(None, None, None),
    )
    .unwrap();
    let requests = processor::take_test_llm_messages("overflow-summary");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].messages[0], history[2]);
    assert_eq!(history, before);
}

#[test]
fn compaction_reuses_cancellation_and_retry() {
    let _guard = processor::test_llm_stream_guard();
    let token = CancellationToken::new();
    token.cancel();
    assert!(
        summarize(
            "cancel-summary",
            &[json!({"role":"user","content":"history"})],
            "test/model",
            &reliability(),
            Some(&token),
            &crate::provider::provider::resolve_model_limit(None, None, None)
        )
        .is_err()
    );
    assert!(processor::take_test_llm_messages("cancel-summary").is_empty());
    processor::enqueue_test_llm_stream(vec![llm::StreamEvent::Error(
        crate::session::message::AssistantError::APIError {
            message: "temporary connection failure".into(),
            status_code: Some(503),
            is_retryable: true,
            response_headers: None,
            response_body: None,
            metadata: None,
        },
    )]);
    processor::enqueue_test_llm_stream(response("recovered"));
    let reliability = ReliabilityConfig { provider_retries: 1, ..reliability() };
    assert_eq!(
        summarize(
            "retry-summary",
            &[json!({"role":"user","content":"history"})],
            "test/model",
            &reliability,
            None,
            &crate::provider::provider::resolve_model_limit(None, None, None)
        )
        .unwrap(),
        "recovered"
    );
    assert_eq!(processor::take_test_llm_messages("retry-summary").len(), 2);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serialize the scripted model boundary.
async fn compaction_dropped_async_request_cancels_the_model_worker() {
    let _guard = processor::test_llm_stream_guard();
    let session = format!("cancel-summary-{}", uuid::Uuid::new_v4());
    let stream = std::sync::Arc::new(processor::ChannelCancelStream::default());
    processor::register_channel_cancel_stream(&session, stream.clone());
    let task = tokio::spawn(summarize_async(
        session,
        vec![json!({"role":"user","content":"history"})],
        "test/model".into(),
        reliability(),
        crate::provider::provider::resolve_model_limit(None, None, None),
    ));
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !stream.started.load(std::sync::atomic::Ordering::Acquire) {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("model request started");
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !stream.exited.load(std::sync::atomic::Ordering::Acquire) {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("dropping the compaction future stops its model worker");
}

fn model_limit(context: u64, input: u64, output: u64) -> ModelLimit {
    ModelLimit { context, input: Some(input), output }
}

fn overflow_error(message: &str) -> llm::StreamEvent {
    llm::StreamEvent::Error(crate::session::message::AssistantError::APIError {
        message: message.into(),
        status_code: Some(400),
        is_retryable: false,
        response_headers: None,
        response_body: None,
        metadata: None,
    })
}

#[test]
fn compaction_preflight_bounds_large_tool_output_without_mutating_source() {
    let _guard = processor::test_llm_stream_guard();
    processor::enqueue_test_llm_stream(response("checkpoint"));
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
    let requests = processor::take_test_llm_messages("preflight-summary");
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
    let _guard = processor::test_llm_stream_guard();
    processor::enqueue_test_llm_stream(vec![overflow_error(
        "HTTP 400: {\"error\":{\"code\":\"invalid_parameter_error\",\"message\":\"Range of input length should be [1, 1000000]\"}}",
    )]);
    processor::enqueue_test_llm_stream(response("checkpoint"));
    let history =
        vec![json!({"role":"user","content":format!("HEAD{}TAIL", "x".repeat(1_100_000))})];
    let before = history.clone();
    let limit = model_limit(2_000_000, 2_000_000, 4_096);
    let summary =
        summarize("range-summary", &history, "test/model", &reliability(), None, &limit).unwrap();
    let requests = processor::take_test_llm_messages("range-summary");
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
    let _guard = processor::test_llm_stream_guard();
    for _ in 0..=MAX_OVERFLOW_RECOVERIES {
        processor::enqueue_test_llm_stream(vec![overflow_error("context_length_exceeded")]);
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
    let requests = processor::take_test_llm_messages("bounded-summary");
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
    let _guard = processor::test_llm_stream_guard();
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
    assert!(processor::take_test_llm_messages("impossible-summary").is_empty());
    assert_eq!(history, before);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serialize the scripted model boundary.
async fn compaction_failed_attempt_leaves_previous_checkpoint_restorable() {
    let _guard = processor::test_llm_stream_guard();
    let session = format!("failed-checkpoint-{}", uuid::Uuid::new_v4());
    let source = vec![json!({"role":"user","content":"original request"})];
    crate::session::compaction_checkpoint::save(&session, &source, "previous decision", None)
        .await
        .unwrap();
    let restored = crate::session::compaction_checkpoint::restore(&session, &source).await.unwrap();
    processor::enqueue_test_llm_stream(vec![overflow_error("invalid API key")]);
    assert!(
        summarize_async(
            session.clone(),
            restored.clone(),
            "test/model".into(),
            reliability(),
            model_limit(10_000, 10_000, 1000)
        )
        .await
        .is_err()
    );
    assert_eq!(processor::take_test_llm_messages(&session).len(), 1);
    assert_eq!(
        crate::session::compaction_checkpoint::restore(&session, &source).await.unwrap(),
        restored
    );
    crate::session::compaction_checkpoint::remove(&session, &crate::project::instance::directory())
        .await
        .unwrap();
}

#[test]
fn compaction_budget_reserves_output_and_accepts_unset_input_limit() {
    assert_eq!(request_input::input_budget(&model_limit(1000, 1000, 200)), 760);
    assert_eq!(request_input::input_budget(&model_limit(1000, 500, 200)), 475);
    assert_eq!(request_input::input_budget(&model_limit(1000, 0, 200)), 760);
    assert_eq!(request_input::input_budget(&model_limit(1000, 1000, 2000)), 712);
}

#[test]
fn compaction_equal_context_and_output_uses_bounded_request_allocation() {
    let _guard = processor::test_llm_stream_guard();
    processor::enqueue_test_llm_stream(response("checkpoint"));
    let limit = ModelLimit { context: 1_048_576, input: None, output: 1_048_576 };
    let history = vec![json!({"role":"user", "content":"Continue the task"})];
    assert_eq!(
        summarize("kimi-summary", &history, "shiju/kimi-k3", &reliability(), None, &limit).unwrap(),
        "checkpoint"
    );
    let requests = processor::take_test_llm_messages("kimi-summary");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].options["max_output_tokens"], 20_000);
    assert_eq!(requests[0].options["disable_tools"], true);
    assert_eq!(request_input::input_budget(&limit), 977_147);
    assert_eq!(&requests[0].messages[..history.len()], history);
}

#[test]
fn compaction_unknown_window_recovers_from_provider_overflow_without_changing_history() {
    let _guard = processor::test_llm_stream_guard();
    processor::enqueue_test_llm_stream(vec![overflow_error("context_length_exceeded")]);
    processor::enqueue_test_llm_stream(response("checkpoint"));
    let limit = ModelLimit { context: 0, input: None, output: 0 };
    let history = vec![json!({"role":"user", "content":"x".repeat(1_000_000)})];
    let before = history.clone();
    summarize("unknown-summary", &history, "custom/model", &reliability(), None, &limit).unwrap();
    let requests = processor::take_test_llm_messages("unknown-summary");
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
    let _guard = processor::test_llm_stream_guard();
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
        processor::enqueue_test_llm_stream(events);
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
        assert_eq!(processor::take_test_llm_messages("empty-summary").len(), 1);
    }
}

#[test]
fn compaction_empty_error_translation_preserves_metadata_and_other_failures() {
    let original = json!({"name":"APIError", "message":"模型未返回内容", "status_code":400,
        "is_retryable":false, "response_body":"original body", "metadata":{"replay_safe":"false", "source":"aisdk"}});
    let mut expected = original.clone();
    expected["message"] = json!("internal compaction generated an empty summary");
    let mapped: Value = serde_json::from_str(&summary_error(original.to_string())).unwrap();
    for key in ["message", "status_code", "is_retryable", "response_body", "metadata"] {
        assert_eq!(mapped[key], expected[key]);
    }
    for error in [
        "执行已取消".to_string(),
        "HTTP 401: 模型未返回内容".to_string(),
        json!({"name":"APIError","message":"upstream failed: 模型未返回内容","is_retryable":false})
            .to_string(),
    ] {
        assert_eq!(summary_error(error.clone()), error);
    }
}

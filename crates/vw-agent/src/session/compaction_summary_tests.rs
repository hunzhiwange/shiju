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
        summarize("single-summary", &history, "test/model", &reliability(), None).unwrap(),
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
    summarize("overflow-summary", &history, "test/model", &reliability(), None).unwrap();
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
            Some(&token)
        )
        .is_err()
    );
    assert!(processor::take_test_llm_messages("cancel-summary").is_empty());
    processor::enqueue_test_llm_stream(vec![llm::StreamEvent::Error(
        crate::session::message::AssistantError::Unknown {
            message: "temporary connection failure".into(),
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
            None
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

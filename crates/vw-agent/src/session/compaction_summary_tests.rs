use super::*;
use crate::session::processor;

fn reliability() -> ReliabilityConfig {
    ReliabilityConfig { provider_retries: 0, provider_backoff_ms: 0, ..Default::default() }
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


#[tokio::test]
async fn model_protocol_016_http_sse_thinking_receipt_replays() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    super::super::messages_thinking_http_tests::thinking_replays_through_bound_http().await;
}

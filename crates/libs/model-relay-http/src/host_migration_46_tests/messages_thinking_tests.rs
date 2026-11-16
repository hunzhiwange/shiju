#[tokio::test]
async fn model_protocol_015_http_tool_round_and_sse_preserve_reasoning() {
    let _lock = super::global_async_env_lock().lock().await;
    super::messages_thinking_http_tests::thinking_replays_through_bound_http().await;
}

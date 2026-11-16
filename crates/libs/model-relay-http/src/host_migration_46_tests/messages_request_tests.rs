//! The complete lib suite also checks shared routing capability registration.



#[tokio::test]
async fn model_protocol_014_http_request_reaches_bound_chat_and_finishes_tool_round() {
    let _lock = super::global_async_env_lock().lock().await;
    super::messages_request_http_tests::request_reaches_bound_chat_and_finishes_tool_round().await;
}

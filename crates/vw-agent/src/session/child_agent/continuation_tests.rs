#[tokio::test]
async fn child_first_response_continuation_preserves_published_title_and_history() {
    gateway_session::child_acceptance::child_first_response_continuation_preserves_published_title_and_history::<
        crate::tools::context::AgentContextHost,
        crate::session::session_io::NativeSessionIo,
    >().await;
}

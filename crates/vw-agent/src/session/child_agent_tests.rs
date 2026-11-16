//! The domain acceptance suite runs with the actual host stream, context and storage ports.
#[tokio::test(flavor = "current_thread")]
async fn child_first_response_gateway_loads_workspace_agents_and_publishes() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    gateway_session::child_acceptance::child_first_response_gateway_loads_workspace_agents_and_publishes::<
        crate::gateway::api::handlers::session::stream::Host,
        crate::tools::context::AgentContextHost,
        crate::session::session_io::NativeSessionIo,
    >().await;
}

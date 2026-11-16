//! Real native bindings for the domain's streaming acceptance assertions.
use super::stream::Host;
use gateway_session::stream_runtime_acceptance_host::{CapturedRequest, CapturedTool, RuntimeAcceptanceHost};
impl RuntimeAcceptanceHost for Host {
    type Fixture = crate::session::llm::recovery_test_support::RecoveryConfig;
    type Boundary = std::sync::Arc<crate::session::processor::llm_runner::ChannelCancelStream>;
    fn env_lock() -> &'static tokio::sync::Mutex<()> { crate::test_support::global_async_env_lock() }
    async fn recovery(api: &str, key: Option<&str>) -> Self::Fixture { Self::Fixture::new(api, key).await }
    fn stream_guard() -> std::sync::MutexGuard<'static, ()> { crate::session::processor::test_llm_stream_guard() }
    fn enqueue(events: Vec<llm_runtime::StreamEvent>) { crate::session::processor::enqueue_test_llm_stream(events); }
    fn captured(session: &str) -> Vec<CapturedRequest> {
        crate::session::processor::take_test_llm_messages(session).into_iter().map(|r| CapturedRequest {
            messages: r.messages, system: r.system, model: r.model,
            tool_specs: r.tool_specs.into_iter().map(|s| CapturedTool { id: s.id, description: s.description }).collect(),
        }).collect()
    }
    fn tool_ids() -> Vec<Vec<String>> { crate::session::processor::llm_runner::take_test_llm_stream_tool_ids() }
    async fn update_config(value: serde_json::Value) -> anyhow::Result<()> { crate::config::update(value).await?; Ok(()) }
    async fn update_project(path: &std::path::Path, value: serde_json::Value) -> anyhow::Result<()> { crate::config::update_project(path, value).await?; Ok(()) }
    fn register_boundary(session: &str) -> Self::Boundary {
        let boundary = std::sync::Arc::new(crate::session::processor::llm_runner::ChannelCancelStream::default());
        crate::session::processor::llm_runner::register_channel_cancel_stream(session, boundary.clone());
        boundary
    }
    fn boundary_started(boundary: &Self::Boundary) -> bool { boundary.started.load(std::sync::atomic::Ordering::Acquire) }
    fn release_boundary(boundary: &Self::Boundary) { boundary.release.store(true, std::sync::atomic::Ordering::Release); }
}

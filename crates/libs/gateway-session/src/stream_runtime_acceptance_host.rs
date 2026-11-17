//! Test-only ports for actual native queue, request capture and configuration fixtures.
use std::{future::Future, path::Path};
use serde_json::Value;

pub struct CapturedRequest {
    pub messages: Vec<Value>,
    pub system: Vec<String>,
    pub model: Option<String>,
    pub tool_specs: Vec<CapturedTool>,
}
pub struct CapturedTool {
    pub id: String,
    pub description: String,
}
pub trait RuntimeAcceptanceHost: crate::stream_host::StreamHost + Send + Sync + 'static {
    type Fixture;
    type Boundary;
    fn env_lock() -> &'static tokio::sync::Mutex<()>;
    fn recovery(api: &str, key: Option<&str>) -> impl Future<Output = Self::Fixture>;
    fn stream_guard() -> std::sync::MutexGuard<'static, ()>;
    fn enqueue(events: Vec<llm_runtime::StreamEvent>);
    fn captured(session: &str) -> Vec<CapturedRequest>;
    fn tool_ids() -> Vec<Vec<String>>;
    fn update_config(value: Value) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn update_project(path: &Path, value: Value) -> impl Future<Output = anyhow::Result<()>>;
    fn register_boundary(session: &str) -> Self::Boundary;
    fn boundary_started(boundary: &Self::Boundary) -> bool;
    fn release_boundary(boundary: &Self::Boundary);
}

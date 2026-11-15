//! ACP host composition and typed route forwarding.
use vw_api_acp_model::{acp_session::{AcpSessionOptions, AcpSessionSelection}, acp_model_source::AcpModelSourceUpdate};
use session_engine::agent_binding::AgentBinding;
use axum::{Json, extract::{Path, Query}, http::HeaderMap};
use crate::gateway::{ApiError, instance::InstanceQuery};
use super::message_ops::Host;
impl gateway_session::acp_host::AcpHost for Host {
    fn annotate_model_source(options: &mut AcpSessionOptions, agent: &str) {
        crate::session::llm::annotate_acp_model_source(options, agent);
    }
    async fn read_options(session: &str, directory: &str) -> Result<AcpSessionOptions, String> {
        crate::session::llm::read_acp_session_options(session, directory).await
    }
    async fn select_option(session: &str, directory: &str, selection: AcpSessionSelection) -> Result<AcpSessionOptions, String> {
        crate::session::llm::select_acp_session_option(session, directory, selection).await
    }
    async fn select_model_source(session: &str, directory: &str, update: AcpModelSourceUpdate) -> Result<AcpSessionOptions, String> {
        crate::session::llm::select_acp_model_source(session, directory, update).await
    }
    async fn load_binding(project: &str, session: &str) -> anyhow::Result<Option<AgentBinding>> {
        crate::session::agent_binding::load_scoped(project, session).await
    }
    async fn save_binding(project: &str, session: &str, binding: &AgentBinding) -> anyhow::Result<()> {
        crate::session::agent_binding::save_scoped(project, session, binding).await
    }
    async fn fork_started(project: &str, session: &str) -> anyhow::Result<Option<bool>> {
        crate::session::fork_execution::load_scoped(project, session).await.map(|state| state.map(|state| state.started))
    }
    async fn agent(name: &str) -> Option<config_agent::DelegateAgentConfig> {
        crate::agent::get(name).await
    }
}
pub(super) async fn get(path: Path<String>, query: Query<InstanceQuery>, headers: HeaderMap) -> Result<Json<AcpSessionOptions>, ApiError> {
    gateway_session::acp_options::get::<Host>(path, Query(gateway_instance::host::InstanceQuery { directory: query.0.directory }), headers).await
}
pub(super) async fn set(path: Path<String>, query: Query<InstanceQuery>, headers: HeaderMap, body: Json<AcpSessionSelection>) -> Result<Json<AcpSessionOptions>, ApiError> {
    gateway_session::acp_options::set::<Host>(path, Query(gateway_instance::host::InstanceQuery { directory: query.0.directory }), headers, body).await
}
pub(super) async fn set_source(path: Path<String>, query: Query<InstanceQuery>, headers: HeaderMap, body: Json<AcpModelSourceUpdate>) -> Result<Json<AcpSessionOptions>, ApiError> {
    gateway_session::acp_options::set_source::<Host>(path, Query(gateway_instance::host::InstanceQuery { directory: query.0.directory }), headers, body).await
}
pub(super) async fn source_capabilities(path: Path<String>) -> Json<AcpSessionOptions> {
    gateway_session::acp_options::source_capabilities::<Host>(path).await
}

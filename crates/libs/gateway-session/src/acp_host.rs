//! Raw ACP, binding and agent catalog operations supplied by the host.
use vw_api_acp_model::{acp_session::{AcpSessionOptions, AcpSessionSelection}, acp_model_source::AcpModelSourceUpdate};
use session_engine::agent_binding::AgentBinding;
use std::future::Future;
use crate::message_host::MessageHost;
pub trait AcpHost: MessageHost {
    fn annotate_model_source(options: &mut AcpSessionOptions, agent: &str);
    fn read_options(session: &str, directory: &str) -> impl Future<Output = Result<AcpSessionOptions, String>> + Send;
    fn select_option(session: &str, directory: &str, selection: AcpSessionSelection) -> impl Future<Output = Result<AcpSessionOptions, String>> + Send;
    fn select_model_source(session: &str, directory: &str, update: AcpModelSourceUpdate) -> impl Future<Output = Result<AcpSessionOptions, String>> + Send;
    fn load_binding(project: &str, session: &str) -> impl Future<Output = anyhow::Result<Option<AgentBinding>>> + Send;
    fn save_binding(project: &str, session: &str, binding: &AgentBinding) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn fork_started(project: &str, session: &str) -> impl Future<Output = anyhow::Result<Option<bool>>> + Send;
    fn agent(name: &str) -> impl Future<Output = Option<config_agent::DelegateAgentConfig>> + Send;
}

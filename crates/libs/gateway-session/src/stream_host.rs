//! Strongly typed environment and execution binding for gateway stream orchestration.
use serde_json::Value;
use std::{future::Future, path::Path};
use vw_api_transport::TenantScope;

/// Inputs actually consumed by the native processor adapter.
pub struct PromptInput {
    pub session: String,
    pub query: String,
    pub root: String,
    pub model: Option<String>,
    pub reliability: config_reliability::ReliabilityConfig,
    pub options: Value,
    pub background_acp: bool,
    pub approval_reply_target: String,
    pub assistant_message_id: Option<String>,
    pub history: Vec<session_ui_types::ChatMessage>,
    pub persisted_history: Option<Vec<Value>>,
    pub persist_app_session_artifacts: bool,
    pub app_session_scope: Option<String>,
    pub abort_token: Option<tokio_util::sync::CancellationToken>,
}

pub trait StreamHost: crate::message_host::MessageHost {
    type ToolContext: Send + 'static;
    type Approval: Send + 'static;
    fn tool_context() -> Self::ToolContext;
    fn approval_manager() -> impl Future<Output = Self::Approval> + Send;
    fn start_prompt(
        tenant: TenantScope,
        context: Self::ToolContext,
        approval: Self::Approval,
        input: PromptInput,
        before_emit: impl FnMut(&session_port::PromptEvent) + Send + 'static,
    ) -> session_port::PromptTurn;
    fn with_tenant_scope<T: Send>(
        tenant: TenantScope,
        future: impl Future<Output = T> + Send,
    ) -> impl Future<Output = T> + Send;
    fn now_ms() -> u64;
    fn is_successful_finish_reason(reason: Option<&str>) -> bool;
    fn delegated_model_ref(config: &config_agent::DelegateAgentConfig) -> Option<String>;
    fn agent(
        name: &str,
    ) -> impl Future<Output = Option<config_agent::AgentDefinitionConfig>> + Send;
    fn config() -> impl Future<Output = anyhow::Result<config_aggregate::Config>> + Send;
    fn project_config(
        tenant: &TenantScope,
        path: &Path,
    ) -> impl Future<Output = anyhow::Result<config_aggregate::Config>> + Send;
    fn native_agent_defaults(
        config: &config_aggregate::Config,
        name: &str,
        model: Option<&str>,
        path: &Path,
    ) -> impl Future<
        Output = anyhow::Result<Option<session_engine::capability_policy::NativeAgentDefaults>>,
    > + Send;
    fn prepare_binding(
        session: &str,
        agent: Option<&str>,
        backend: Option<&str>,
        model: Option<&str>,
        path: &Path,
    ) -> impl Future<
        Output = anyhow::Result<(
            session_engine::agent_binding::AgentBinding,
            config_aggregate::Config,
        )>,
    > + Send;
    fn resolve_full_access(
        session: &str,
        explicit: Option<bool>,
        initial: bool,
    ) -> impl Future<Output = anyhow::Result<bool>> + Send;
    fn fork_execution(
        project: &str,
        session: &str,
    ) -> impl Future<Output = anyhow::Result<Option<session_engine::fork_execution::ForkExecution>>> + Send;
    fn start_fork_execution(
        project: &str,
        session: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn locked_acp_model(
        session: &str,
        directory: &str,
    ) -> Result<Option<vw_api_acp_model::acp_model_source::AcpSessionModelSelection>, String>;
    fn forked_session(
        session: &str,
    ) -> impl Future<Output = anyhow::Result<Option<session_info::Info>>> + Send;
    fn load_fork_history(
        info: &session_info::Info,
    ) -> impl Future<Output = anyhow::Result<(Vec<session_ui_types::ChatMessage>, Vec<Value>)>> + Send;
    fn project_from_directory(
        tenant: &TenantScope,
        directory: &str,
    ) -> impl Future<Output = anyhow::Result<(agent_project::Info, String)>> + Send;
    fn refresh_stream_diff_summary(
        session: &str,
        parts: &[session_engine::message::Part],
    ) -> impl Future<Output = anyhow::Result<Option<session_info::Summary>>> + Send;
    fn refresh_session_diff_summary(
        session: &str,
    ) -> impl Future<Output = anyhow::Result<session_info::Summary>> + Send;
    fn spawn_first_response_title(
        runtime: &tokio::runtime::Handle,
        tenant: TenantScope,
        context: agent_project::instance::Context,
        session: String,
        preview: String,
        model: Option<String>,
    );
}

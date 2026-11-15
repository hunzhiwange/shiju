//! 宿主能力边界；消息编排不通过回调移交宿主。
use std::{sync::Arc, collections::HashMap, future::Future, pin::Pin};
use anyhow::Result;
use channel_support::{runtime_state::ChannelRuntimeContext, scope::ChannelMessageSnapshot, routing::ChannelRouteSelection, session_events::ChannelProcessorResponse};
use channel_types::{Channel, ChannelMessage};
use config_agent::AgentDefinitionConfig;
use session_engine::agent_binding::AgentBinding;
use tokio_util::sync::CancellationToken;
#[cfg(not(target_arch="wasm32"))]
pub type ScopedFuture<'a> = Pin<Box<dyn Future<Output=()> + Send + 'a>>;
#[cfg(target_arch="wasm32")]
pub type ScopedFuture<'a> = Pin<Box<dyn Future<Output=()> + 'a>>;

/// 通道当前回合提交给现有会话处理器的强类型输入。
pub struct ChannelExecutionInput {
    pub stream: u64,
    pub session: String,
    pub query: String,
    pub directory: std::path::PathBuf,
    pub model: String,
    pub reliability: agent_config::runtime_schema::ReliabilityConfig,
    pub options: serde_json::Value,
    pub approval: Arc<agent_approval::ApprovalManager>,
    pub channel_name: String,
    pub non_cli_approval_context: tool_context::approval::NonCliApprovalContext,
    pub history: Vec<session_port::ui_types::ChatMessage>,
    pub project: String,
    pub abort: CancellationToken,
}

#[cfg_attr(target_arch="wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch="wasm32"), async_trait::async_trait)]
pub trait ChannelHost: crate::command_host::CommandHost + Send + Sync {
    fn now_ms(&self) -> u64;
    fn scrub_credentials(&self, text: &str) -> String;
    fn draft_sentinels(&self) -> (&'static str, &'static str, &'static str);
    async fn save_config(&self, config: &agent_config::runtime_schema::Config) -> Result<()>;

    async fn in_tenant_scope(&self, snapshot: &ChannelMessageSnapshot, operation: ScopedFuture<'_>);
    async fn in_scope(&self, snapshot: &ChannelMessageSnapshot, operation: ScopedFuture<'_>);
    async fn runtime_command(&self, ctx: &ChannelRuntimeContext, msg: &ChannelMessage, channel: Option<&Arc<dyn Channel>>) -> bool;
    async fn session_id(&self, snapshot: &ChannelMessageSnapshot, msg: &ChannelMessage, route: Option<ChannelRouteSelection>) -> Result<String>;
    async fn collect_response(&self, input: ChannelExecutionInput, delta: Option<tokio::sync::mpsc::Sender<String>>) -> Result<ChannelProcessorResponse>;
    fn reap_shell(&self, snapshot: &ChannelMessageSnapshot, session: &str) -> Result<()>;
    async fn load_binding(&self, project: &str, session: &str) -> Result<Option<AgentBinding>>;
    async fn agent_catalog(&self) -> Result<HashMap<String, AgentDefinitionConfig>>;
    async fn prepare_binding(&self, session: &str, agent: &str, model: Option<&str>, directory: &std::path::Path) -> Result<AgentBinding>;
    fn locked_model(&self, session: &str, directory: &str) -> Result<Option<vw_api_acp_model::acp_model_source::AcpSessionModelSelection>>;
    async fn persist_turn(&self, ctx: &ChannelRuntimeContext, msg: &ChannelMessage, route: &ChannelRouteSelection, response: &str, processed: ChannelProcessorResponse) -> Result<Option<String>>;
    async fn take_reply(&self, snapshot: &ChannelMessageSnapshot, msg: &ChannelMessage) -> Result<Option<(String, Option<String>)>>;
    async fn retry_reply(&self, snapshot: &ChannelMessageSnapshot, session: &str, msg: &ChannelMessage, reply: String) -> Result<()>;
}

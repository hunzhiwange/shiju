//! Agent状态、构建与单轮运行；宿主只组装真实工厂与执行能力。
pub mod builder;
pub mod core;
mod turn;
pub use builder::AgentBuilder;
pub use core::Agent;
use std::sync::Arc;
use anyhow::Result;
use model_types::{Provider, ChatMessage};
use agent_approval::ApprovalManager;
use agent_observability::Observer;
use agent_security::SecurityPolicy;
use tool_types::Tool;
use agent_hooks::HookRunner;
use tokio_util::sync::CancellationToken;
use agent_config::runtime::Config;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait AgentHost: crate::core::skills::CoreSkillHost {
    fn from_config(config: &Config) -> Result<Agent<Self>>;
    async fn models() -> Result<Vec<model_types::Model>>;
async fn run_tool_call_loop_with_skills(
    provider: &dyn Provider,
    history: &mut Vec<ChatMessage>,
    tools_registry: &[Box<dyn Tool>],
    observer: &dyn Observer,
    provider_name: &str,
    model: &str,
    temperature: f64,
    silent: bool,
    approval: Option<Arc<ApprovalManager>>,
    channel_name: &str,
    multimodal_config: &config_tools::MultimodalConfig,
    max_iterations: usize,
    cancellation_token: Option<CancellationToken>,
    on_delta: Option<tokio::sync::mpsc::Sender<String>>,
    hooks: Option<Arc<HookRunner>>,
    security: Option<Arc<SecurityPolicy>>,
    excluded_tools: &[String],
    skills: Option<&mut crate::core::skills::CoreSkillSession<Self>>,
) -> Result<String>;
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<H: AgentHost> crate::run::RunAgent for Agent<H> {
    fn observer(&self) -> &dyn observer_types::Observer { self.observer.as_ref() }
    async fn run_single(&mut self, message: &str) -> Result<String> {
        Agent::run_single(self, message).await
    }
}

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

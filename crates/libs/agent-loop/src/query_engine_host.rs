use agent_config::runtime::Config;
use agent_memory::Memory;
use tool_types::Tool;
use agent_observability::Observer;
use model_types::{ChatMessage, Provider};
use std::sync::Arc;
use anyhow::Result;
/// Host I/O retains the caller's tenant, tool and cancellation scopes.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait QueryEngineHost: agent_loop::core::skills::CoreSkillHost<Context: tool_context::host::ContextHost<HookRunner = agent_hooks::HookRunner>> {
    fn assemble_tools(config: &Config, session: &str) -> Result<(Arc<dyn Memory>, Vec<Box<dyn Tool>>)>;
    async fn team_runtime_instructions(session: &str, enabled: bool) -> String;
    async fn restore_history(session: &str, source: &[serde_json::Value]) -> std::result::Result<Vec<serde_json::Value>, String>;
    async fn compact_history(history: &mut Vec<ChatMessage>, session: &str, model: &str, reliability: &config_reliability::ReliabilityConfig, limit: &agent_provider::provider::ModelLimit) -> Result<bool>;
    async fn agent_turn(provider: &dyn Provider, history: &mut Vec<ChatMessage>, tools: &[Box<dyn Tool>], observer: &dyn Observer, provider_name: &str, model: &str, temperature: f64, silent: bool, multimodal: &agent_config::schema::MultimodalConfig, max_iterations: usize, hooks: Option<Arc<agent_hooks::HookRunner>>, skills: Option<&mut agent_loop::core::skills::CoreSkillSession<Self>>) -> Result<String>;
}

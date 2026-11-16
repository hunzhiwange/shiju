//! Concrete query host binding; orchestration belongs to agent-loop.
use crate::{config::Config, memory::{self, Memory}, observability::Observer, tools::Tool};
use model_types::{ChatMessage, Provider};
use std::sync::Arc;
use anyhow::Result;
pub use agent_loop::query_engine::{QueryEngineUsage, QueryEngineBudgetSnapshot, QueryEngineSessionState, QueryEngineSnapshot};
pub type QueryEngine = agent_loop::query_engine::QueryEngine<super::core::skills::NativeCoreSkillHost>;
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl agent_loop::query_engine::QueryEngineHost for super::core::skills::NativeCoreSkillHost {
    fn assemble_tools(config: &Config, session_id: &str) -> Result<(Arc<dyn Memory>, Vec<Box<dyn Tool>>)> {
        let runtime = Arc::from(crate::runtime::create_runtime(&config.runtime)?);
        let security = Arc::new(crate::security::SecurityPolicy::from_configs(&config.autonomy, &config.security, &config.workspace_dir));
        let memory: Arc<dyn Memory> = Arc::from(memory::create_memory_with_storage(&config.memory, Some(&config.storage.provider.config), &config.workspace_dir, config.api_key.as_deref())?);
        let tools = crate::tools::all_tools_with_runtime(Arc::new(config.clone()), &security, runtime, memory.clone(), &config.browser, &config.http_request, &config.web_fetch, &config.workspace_dir, &config.agents, config.api_key.as_deref(), config, Some(session_id));
        Ok((memory, tools))
    }
    async fn team_runtime_instructions(session: &str, enabled: bool) -> String { crate::session::system::team_runtime_instructions(session, enabled).await }
    async fn restore_history(session: &str, source: &[serde_json::Value]) -> std::result::Result<Vec<serde_json::Value>, String> { crate::session::compaction_checkpoint::restore(session, source).await }
    async fn compact_history(history: &mut Vec<ChatMessage>, session: &str, model: &str, reliability: &crate::config::ReliabilityConfig, limit: &crate::provider::provider::ModelLimit) -> Result<bool> { super::history::compact_history_with_internal_service(history, session, model, reliability, limit).await }
    async fn agent_turn(provider: &dyn Provider, history: &mut Vec<ChatMessage>, tools: &[Box<dyn Tool>], observer: &dyn Observer, provider_name: &str, model: &str, temperature: f64, silent: bool, multimodal: &crate::config::MultimodalConfig, max_iterations: usize, hooks: Option<Arc<crate::hooks::HookRunner>>, skills: Option<&mut super::core::skills::CoreSkillSession>) -> Result<String> { super::core::agent_turn(provider, history, tools, observer, provider_name, model, temperature, silent, multimodal, max_iterations, hooks, skills).await }
}
#[cfg(test)]
#[path = "../../../../libs/agent-loop/src/query_engine_test_binding.rs"]
mod test_binding;

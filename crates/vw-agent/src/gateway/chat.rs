//! Gateway chat composition for the real query engine and session store.
use super::state::{AppState, SharedQueryEngine};
use crate::multimodal::ocr::SessionImageTextExtractor;
#[cfg(test)]
use crate::multimodal::ocr::ImageTextExtractor;
use model_types::{ChatMessage, Provider};
use std::sync::Arc;
use agent_gateway::chat_host::{GatewayChatHost, GatewaySessionStats};
use crate::agent::loop_::query_engine::{QueryEngine, QueryEngineSnapshot};
use crate::session::session;
use crate::config::Config;
struct Host;
impl GatewayChatHost for Host {
    type Engine = QueryEngine;
    type Snapshot = QueryEngineSnapshot;
    type Channels = super::state::GatewayChannels;
    type Hooks = crate::hooks::HookRuntime;
    type Extractor = SessionImageTextExtractor;
    fn tenant() -> vw_api_transport::TenantScope { crate::session::ui_store::current_tenant_scope() }
    fn extractor() -> Self::Extractor { SessionImageTextExtractor::new() }
    fn simple_prompt(config_guard: &Config, model: &str) -> anyhow::Result<(String, Vec<ChatMessage>)> {
        let timezone = crate::app::agent::util::locale::parse_timezone(
            &config_guard.app_ui.system_settings.timezone,
        )?;
        let system_prompt = crate::app::agent::channels::build_system_prompt(
            &config_guard.workspace_dir,
            model,
            &[], // tools - 简单聊天不使用工具
            &[], // skills stay outside the system prompt
            Some(&config_guard.identity),
            None, // bootstrap_max_chars - 使用默认值
            timezone,
        );
        let skills = crate::app::agent::skills::load_skills_with_config(
            &config_guard.workspace_dir,
            &config_guard,
        );
        let protocol_messages = crate::app::agent::channels::build_workspace_protocol_messages(
            &config_guard.workspace_dir,
            &skills,
            false,
            8_192,
        );
        Ok((system_prompt, protocol_messages))
    }
    async fn forked_session(id: &str) -> anyhow::Result<Option<session::Info>> { Ok(session::forked_session(id).await?) }
    async fn load_fork_history(info: &session::Info) -> anyhow::Result<(Vec<crate::session::ui_types::ChatMessage>, Vec<serde_json::Value>)> { Ok(session::load_fork_history(info).await?) }
    async fn new_engine(config: Config, id: &str, provider: Option<Arc<dyn Provider>>, hooks: Option<Self::Hooks>) -> anyhow::Result<QueryEngine> { QueryEngine::new_with_provider_and_hook_runtime(config, id, provider, hooks.map(|hooks| hooks.shared())).await }
    fn replace_history(engine: &mut QueryEngine, history: Vec<ChatMessage>) { engine.replace_history(history); }
    fn snapshot(engine: &QueryEngine) -> Self::Snapshot { engine.snapshot() }
    fn restore_snapshot(engine: &mut QueryEngine, snapshot: Self::Snapshot) { engine.restore_snapshot(snapshot); }
    async fn submit_message<'a>(engine: &'a mut QueryEngine, message: &'a str) -> anyhow::Result<String> { engine.submit_message(message).await }
    fn stats(engine: &QueryEngine) -> GatewaySessionStats {
        let state = engine.session_state();
        let usage = state.usage.as_ui_token_usage();
        GatewaySessionStats { turn_count: state.turn_count, llm_calls: state.usage.llm_calls, total_tokens: state.usage.total_tokens(), input_tokens: usage.input_tokens, output_tokens: usage.output_tokens, cached_tokens: usage.cached_tokens, reasoning_tokens: usage.reasoning_tokens, non_system_messages: state.budget.non_system_messages, remaining_history_messages: state.budget.remaining_history_messages, max_history_messages: state.budget.max_history_messages }
    }
}
pub(super) async fn session_query_engine(state: &AppState, id: &str) -> anyhow::Result<SharedQueryEngine> { agent_gateway::chat::session_query_engine::<Host>(state, id).await }
pub(crate) fn session_query_engine_key(id: &str) -> (vw_api_transport::TenantScope, String) { agent_gateway::chat::session_query_engine_key::<Host>(id) }
pub(crate) async fn invalidate_session_query_engine(state: &AppState, id: &str) { agent_gateway::chat::invalidate_session_query_engine::<Host>(state, id).await }
pub(crate) async fn invalidate_all_session_query_engines(state: &AppState) { agent_gateway::chat::invalidate_all_session_query_engines::<Host>(state).await }
pub(crate) async fn fork_session_query_engine(state: &AppState, source: &str, target: &str) -> anyhow::Result<()> { agent_gateway::chat::fork_session_query_engine::<Host>(state, source, target).await }
pub async fn run_gateway_chat_simple(state: &AppState, message: &str) -> anyhow::Result<String> { agent_gateway::chat::run_gateway_chat_simple::<Host>(state, message).await }
#[cfg(test)]
async fn run_gateway_chat_simple_with_extractor(state: &AppState, message: &str, extractor: &dyn ImageTextExtractor) -> anyhow::Result<String> { agent_gateway::chat::run_gateway_chat_simple_with_extractor::<Host>(state, message, extractor).await }
pub async fn run_gateway_chat_with_tools(state: &AppState, message: &str, id: &str) -> anyhow::Result<String> { agent_gateway::chat::run_gateway_chat_with_tools::<Host>(state, message, id).await }
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub use agent_gateway::chat::sanitize_gateway_response;
#[cfg(any(test, target_arch = "wasm32"))]
pub use agent_gateway::chat::{sanitize_gateway_response, log_channel_message};
#[cfg(test)]
#[path = "../../../libs/agent-gateway/src/chat_tests.rs"]
mod chat_tests;

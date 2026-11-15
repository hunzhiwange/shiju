//! Raw query-engine and session-storage ports used by gateway chat.
use std::{collections::HashMap, sync::Arc, future::Future};
use agent_config::runtime::Config;
use model_types::{ChatMessage, Provider};
use session_info::Info;
use serde_json::Value;
use vw_api_transport::TenantScope;
pub type SharedQueryEngine<H> = Arc<tokio::sync::Mutex<<H as GatewayChatHost>::Engine>>;
pub type SessionQueryEngineStore<H> = Arc<tokio::sync::Mutex<HashMap<(TenantScope, String), SharedQueryEngine<H>>>>;
pub type AppState<H> = gateway_types::state::AppState<SessionQueryEngineStore<H>, <H as GatewayChatHost>::Channels, <H as GatewayChatHost>::Hooks>;
pub struct GatewaySessionStats {
    pub turn_count: usize, pub llm_calls: usize,
    pub total_tokens: i64, pub input_tokens: i64, pub output_tokens: i64,
    pub cached_tokens: i64, pub reasoning_tokens: i64,
    pub non_system_messages: usize, pub remaining_history_messages: usize, pub max_history_messages: usize,
}
pub trait GatewayChatHost: Send + Sync {
    type Engine: Send;
    type Snapshot: Send;
    type Channels;
    type Hooks: Clone;
    type Extractor: agent_multimodal::ocr::ImageTextExtractor;
    fn tenant() -> TenantScope;
    fn extractor() -> Self::Extractor;
    fn simple_prompt(config: &Config, model: &str) -> anyhow::Result<(String, Vec<ChatMessage>)>;
    fn forked_session(id: &str) -> impl Future<Output = anyhow::Result<Option<Info>>> + Send;
    fn load_fork_history(info: &Info) -> impl Future<Output = anyhow::Result<(Vec<session_ui_types::ChatMessage>, Vec<Value>)>> + Send;
    fn new_engine(config: Config, id: &str, provider: Option<Arc<dyn Provider>>, hooks: Option<Self::Hooks>) -> impl Future<Output = anyhow::Result<Self::Engine>> + Send;
    fn replace_history(engine: &mut Self::Engine, history: Vec<ChatMessage>);
    fn snapshot(engine: &Self::Engine) -> Self::Snapshot;
    fn restore_snapshot(engine: &mut Self::Engine, snapshot: Self::Snapshot);
    fn submit_message<'a>(engine: &'a mut Self::Engine, message: &'a str) -> impl Future<Output = anyhow::Result<String>> + Send + 'a;
    fn stats(engine: &Self::Engine) -> GatewaySessionStats;
}

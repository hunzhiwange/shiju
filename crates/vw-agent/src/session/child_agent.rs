//! 持久化子会话执行入口。
//!
//! 执行前为父子关系预留稳定会话 ID。首轮有效回复落盘后提交会话，
//! 失败和取消仍保留原 ID 及运行诊断。

use anyhow::Result;
use std::future::Future;
use vw_api_chat::chat::GatewayChatStreamRequest;
use crate::gateway::ApiError;
use crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected;
pub(crate) mod coordinator;
// Child history projection is owned by session-engine.
pub use session_engine::child_agent::run::{ChildAgentRunInput, ChildAgentRunResult, ChildAgentTerminalState, ForkTurns};
pub(crate) use session_engine::child_agent::run::{task_summary, visible_answer_from_execution_output, child_agent_handoff_from_output};
#[cfg(test)]
pub(crate) use session_engine::child_agent::run::stream_request;
type Host = crate::tools::context::AgentContextHost;
pub(crate) type CreatedChildAgentRun = session_engine::child_agent::run::CreatedChildAgentRun<Host>;
impl session_engine::child_agent::run::ChildHost for Host {
    async fn get_session(session: &str) -> Result<super::session::Info> { Ok(super::session::get(session).await?) }
    async fn reserve_child_agent(parent: String, directory: String, permission: Option<crate::permission::next::Ruleset>) -> Result<super::session::Info> {
        Ok(super::session::reserve_child_agent(super::session::CreateInput {parent_id:Some(parent), title:None, directory, permission}).await?)
    }
    fn execution_hook_runner(config: &crate::config::Config) -> Result<Option<std::sync::Arc<crate::hooks::HookRunner>>> { crate::hooks::execution_hook_runner(config) }
    async fn permission_rules(agent: &str) -> Option<crate::permission::next::Ruleset> { crate::agent::permission_rules(agent).await }
    async fn collect_gateway_chat_stream(directory: String, request: GatewayChatStreamRequest) -> Result<GatewayChatStreamCollected, ApiError> {
        crate::gateway::api::handlers::session::stream::collect_gateway_chat_stream(directory, request).await
    }
    async fn persist_child_agent_turn(session: &str, prompt: &str, output: &str, agent: &str, error: Option<&str>) -> Result<(String,String), ApiError> {
        crate::gateway::api::handlers::session::stream::persist_child_agent_turn(session,prompt,output,agent,error).await
    }
    fn commit_collected_child_agent_turn(session: &str, prompt: &str, output: &str, ids:(String,String)) -> Result<()> {
        crate::gateway::api::handlers::session::stream::commit_collected_child_agent_turn(session,prompt,output,ids)
    }
}
pub(crate) async fn session_depth(directory: String, session: String) -> Result<usize> { session_engine::child_agent::run::session_depth::<Host>(directory,session).await }
pub async fn run(input: ChildAgentRunInput) -> Result<ChildAgentRunResult> { session_engine::child_agent::run::run::<Host>(input).await }
pub(crate) async fn create(input: ChildAgentRunInput) -> Result<CreatedChildAgentRun> { session_engine::child_agent::run::create::<Host>(input).await }
pub(crate) async fn create_with_fork(input: ChildAgentRunInput, fork: ForkTurns) -> Result<CreatedChildAgentRun> { session_engine::child_agent::run::create_with_fork::<Host>(input,fork).await }
pub(crate) async fn run_created(created: CreatedChildAgentRun) -> Result<ChildAgentRunResult> { session_engine::child_agent::run::run_created::<Host>(created).await }
pub(crate) async fn cancel_created(created: CreatedChildAgentRun, reason: impl Into<String>) -> Result<ChildAgentRunResult> { session_engine::child_agent::run::cancel_created::<Host>(created,reason).await }
pub(crate) async fn run_created_with_repeating_executor<F,Fut>(created: CreatedChildAgentRun, execute:F) -> Result<ChildAgentRunResult>
where F: FnMut(GatewayChatStreamRequest) -> Fut + Send + 'static,
Fut: Future<Output = Result<GatewayChatStreamCollected,ApiError>> + Send + 'static {
    session_engine::child_agent::run::run_created_with_repeating_executor::<Host,_,_>(created,execute).await
}
pub(crate) async fn run_with_stream_executor<F,Fut>(input: ChildAgentRunInput, execute:F) -> Result<ChildAgentRunResult>
where F: FnOnce(GatewayChatStreamRequest) -> Fut + Send + 'static,
Fut: Future<Output = Result<GatewayChatStreamCollected,ApiError>> + Send + 'static {
    session_engine::child_agent::run::run_with_stream_executor::<Host,_,_>(input,execute).await
}
pub(crate) async fn run_created_with_stream_executor<F,Fut>(created: CreatedChildAgentRun, execute:F) -> Result<ChildAgentRunResult>
where F: FnOnce(GatewayChatStreamRequest) -> Fut + Send + 'static,
Fut: Future<Output = Result<GatewayChatStreamCollected,ApiError>> + Send + 'static {
    session_engine::child_agent::run::run_created_with_stream_executor::<Host,_,_>(created,execute).await
}
#[cfg(test)]
#[path = "child_agent/entry_tests.rs"]
mod entry_tests;

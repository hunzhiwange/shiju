use anyhow::Result;
use std::{sync::{Arc, Mutex}, time::Duration};
use tokio::sync::{mpsc, OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;
use tool_context::context::ToolUseContext;
use session_port::{child_agent::*, task_session::*};

#[async_trait::async_trait]
pub trait ChildCoordinator: Clone + Send + Sync + 'static {
    type Context: tool_context::host::ContextHost;
    fn team_capacity(&self, session_id: &str) -> Arc<Semaphore>;
    fn track_execution(
        &self,
        child: &str,
        permit: OwnedSemaphorePermit,
    ) -> Arc<Mutex<Option<OwnedSemaphorePermit>>>;
    fn pause_execution(&self, session: &str) -> bool;
    async fn resume_execution(&self, session: &str) -> Result<()>;
    fn finish_execution(&self, session: &str);
    fn register(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
        agent: &str,
        task: &str,
    ) -> Result<CancellationToken>;
    fn mark_running(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
    ) -> Result<bool>;
    fn mark_succeeded(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
        output: impl Into<String>,
    ) -> Result<bool>;
    fn mark_failed(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
        error: impl Into<String>,
    ) -> Result<bool>;
    fn finish_turn(
        &self,
        parent: &str,
        child: &str,
        turn: u64,
        output: Option<String>,
        error: Option<String>,
    ) -> Result<bool>;
    fn cancel(&self, parent_session_id: &str, child_session_id: &str) -> Result<bool>;
    fn snapshots(
        &self,
        parent_session_id: &str,
        child_session_ids: &[String],
    ) -> Result<Vec<ChildAgentTaskSnapshot>>;
    fn matching_ancestor_task(
        &self,
        parent_session_id: &str,
        task: &str,
    ) -> Result<Option<ChildAgentTaskSnapshot>>;
    async fn wait(
        &self,
        parent_session_id: &str,
        child_session_ids: &[String],
        condition: WaitCondition,
        timeout: Duration,
    ) -> Result<ChildAgentWaitOutcome>;
    fn cancel_descendants(
        &self,
        parent_session_id: &str,
        child_session_ids: &[String],
    ) -> Result<usize>;
    fn attach_continuation(&self, child: &str) -> mpsc::Receiver<String>;
    fn followup_task(&self, parent: &str, child: &str, prompt: String) -> Result<()>;
    fn turn_cancellation(
        &self,
        child: &str,
    ) -> Result<tokio_util::sync::CancellationToken>;
    fn begin_followup(&self, parent: &str, child: &str, prompt: &str) -> Result<()>;
    async fn team_snapshots(
        &self,
        context: &ToolUseContext<Self::Context>,
    ) -> Result<Vec<ChildAgentTaskSnapshot>>;
    async fn send_team_message(
        &self,
        context: &ToolUseContext<Self::Context>,
        target: &str,
        turn: u64,
        kind: TeamMessageKind,
        payload: String,
    ) -> Result<TeamMessage>;
    async fn wait_team_activity(
        &self,
        context: &ToolUseContext<Self::Context>,
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<TeamActivity>;
}

#[async_trait::async_trait]
pub trait TaskCoordinator: Clone + Send + Sync + 'static {
    fn register(
        &self,
        parent_session_id: &str,
        task_session_id: &str,
        title: Option<String>,
    ) -> Result<()>;
    fn restore(&self, snapshot: TaskSessionSnapshot);
    fn mark_running(
        &self,
        parent_session_id: &str,
        task_session_id: &str,
    ) -> Result<bool>;
    fn begin_turn(&self, parent_session_id: &str, task_session_id: &str) -> Result<()>;
    fn finish_turn(
        &self,
        parent_session_id: &str,
        task_session_id: &str,
        error: Option<String>,
        persist: impl FnOnce(&TaskSessionSnapshot) -> Result<()>,
    ) -> Result<bool>;
    fn snapshots(
        &self,
        parent_session_id: &str,
        task_session_ids: &[String],
    ) -> Result<Vec<TaskSessionSnapshot>>;
    async fn wait(
        &self,
        parent_session_id: &str,
        task_session_ids: &[String],
        condition: TaskSessionWaitCondition,
        timeout: Duration,
    ) -> Result<TaskSessionWaitOutcome>;
}

use std::{future::Future, path::PathBuf};
use futures_util::future::BoxFuture;
use vw_api_transport::id::TenantScope;
use session_port::{SessionScope, SessionSnapshot, chat_stream::GatewayChatStreamCollected};
use vw_api_chat::chat::GatewayChatStreamRequest;
use config_agent::DelegateAgentConfig;
use config_tools::MultimodalConfig;
use agent_security::SecurityPolicy;
use model_types::{ChatMessage, Provider};
use tool_types::Tool;
use observer_types::Observer;

pub type ChildStreamExecutor<E> = Arc<dyn Fn(GatewayChatStreamRequest) -> BoxFuture<'static, std::result::Result<GatewayChatStreamCollected, E>> + Send + Sync>;
pub type SessionStreamExecutor<E> = Arc<dyn Fn(String, GatewayChatStreamRequest) -> BoxFuture<'static, std::result::Result<GatewayChatStreamCollected, E>> + Send + Sync>;

#[async_trait::async_trait]
pub trait ChildRun: Clone + Send + Sync + 'static {
    fn child_session_id(&self) -> &str;
    async fn prepare_followup(&mut self, prompt: String) -> Result<()>;
}

#[async_trait::async_trait]
pub trait AgentHost: Send + Sync + 'static {
    type Context: tool_context::host::ContextHost;
    type Coordinator: ChildCoordinator<Context = Self::Context>;
    type Run: ChildRun;
    type Error: std::fmt::Display + Send + Sync + 'static;
    fn coordinator() -> Self::Coordinator;
    fn normalize_path(path: PathBuf) -> PathBuf;
    fn resolve_model_ref(config: &DelegateAgentConfig) -> Option<String>;
    fn task_summary(prompt: &str) -> String;
    fn tenant() -> TenantScope;
    fn with_tenant<F: Future + Send>(tenant: TenantScope, operation: F) -> impl Future<Output = F::Output> + Send where F::Output: Send;
    async fn session_depth(directory: String, parent: String) -> Result<usize>;
    async fn create(input: ChildAgentRunInput, fork: ForkTurns) -> Result<Self::Run>;
    async fn run(created: Self::Run, executor: Option<ChildStreamExecutor<Self::Error>>) -> Result<ChildAgentRunResult>;
    async fn cancel(created: Self::Run, reason: &str) -> Result<ChildAgentRunResult>;
}

#[async_trait::async_trait]
pub trait SessionHost: Send + Sync + 'static {
    type Context: tool_context::host::ContextHost;
    type Coordinator: TaskCoordinator;
    type Error: std::fmt::Display + Send + Sync + 'static;
    fn coordinator() -> Self::Coordinator;
    fn tenant() -> TenantScope;
    fn now_ms() -> u64;
    fn with_tenant<F: Future + Send>(tenant: TenantScope, operation: F) -> impl Future<Output = F::Output> + Send where F::Output: Send;
    fn load_states(tenant: &TenantScope, project: &str) -> Result<std::collections::HashMap<String, TaskSessionSnapshot>>;
    fn save_state(tenant: &TenantScope, project: &str, snapshot: &TaskSessionSnapshot) -> Result<()>;
    fn committed(tenant: &TenantScope, project: &str, session: &str) -> Result<bool>;
    async fn load(scope: SessionScope, session: &str) -> Result<SessionSnapshot>;
    async fn reserve(scope: SessionScope, parent: String) -> Result<SessionSnapshot>;
    async fn visible(scope: SessionScope, parent: &str) -> Result<Vec<session_info::Info>>;
    async fn collect(directory: String, request: GatewayChatStreamRequest) -> std::result::Result<GatewayChatStreamCollected, Self::Error>;
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait DelegateHost: Send + Sync + 'static {
    type Context: tool_context::host::ContextHost;
    fn tools(parent: &[Arc<dyn Tool>], config: &DelegateAgentConfig, trusted: &[String]) -> Vec<Box<dyn Tool>>;
    async fn run_loop(provider: &dyn Provider, history: &mut Vec<ChatMessage>, tools: &[Box<dyn Tool>], observer: &dyn Observer, provider_name: &str, model: &str, temperature: f64, multimodal: &MultimodalConfig, max_iterations: usize, security: Arc<SecurityPolicy>) -> Result<String>;
}

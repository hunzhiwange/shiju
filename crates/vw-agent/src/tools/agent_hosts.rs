use tool_agents::ports::*;
use session_port::{child_agent::*, task_session::*};
use tool_context::context::ToolUseContext;
use std::{sync::{Arc, Mutex}, time::Duration};
use tokio::sync::{mpsc, OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;
use anyhow::Result;

#[derive(Clone)]
pub struct ChildCoordinatorHost(pub(crate) crate::session::child_agent::coordinator::ChildAgentCoordinator);
impl From<crate::session::child_agent::coordinator::ChildAgentCoordinator> for ChildCoordinatorHost { fn from(value: crate::session::child_agent::coordinator::ChildAgentCoordinator) -> Self { Self(value) } }
#[async_trait::async_trait]
impl ChildCoordinator for ChildCoordinatorHost {
    fn team_capacity(&self, session_id: &str) -> Arc<Semaphore> { self.0.team_capacity(session_id) }
    fn track_execution(
        &self,
        child: &str,
        permit: OwnedSemaphorePermit,
    ) -> Arc<Mutex<Option<OwnedSemaphorePermit>>> { self.0.track_execution(child, permit) }
    fn pause_execution(&self, session: &str) -> bool { self.0.pause_execution(session) }
    async fn resume_execution(&self, session: &str) -> Result<()> { self.0.resume_execution(session).await }
    fn finish_execution(&self, session: &str) { self.0.finish_execution(session) }
    fn register(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
        agent: &str,
        task: &str,
    ) -> Result<CancellationToken> { self.0.register(parent_session_id, child_session_id, agent, task) }
    fn mark_running(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
    ) -> Result<bool> { self.0.mark_running(parent_session_id, child_session_id) }
    fn mark_succeeded(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
        output: impl Into<String>,
    ) -> Result<bool> { self.0.mark_succeeded(parent_session_id, child_session_id, output) }
    fn mark_failed(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
        error: impl Into<String>,
    ) -> Result<bool> { self.0.mark_failed(parent_session_id, child_session_id, error) }
    fn finish_turn(
        &self,
        parent: &str,
        child: &str,
        turn: u64,
        output: Option<String>,
        error: Option<String>,
    ) -> Result<bool> { self.0.finish_turn(parent, child, turn, output, error) }
    fn cancel(&self, parent_session_id: &str, child_session_id: &str) -> Result<bool> { self.0.cancel(parent_session_id, child_session_id) }
    fn snapshots(
        &self,
        parent_session_id: &str,
        child_session_ids: &[String],
    ) -> Result<Vec<ChildAgentTaskSnapshot>> { self.0.snapshots(parent_session_id, child_session_ids) }
    fn matching_ancestor_task(
        &self,
        parent_session_id: &str,
        task: &str,
    ) -> Result<Option<ChildAgentTaskSnapshot>> { self.0.matching_ancestor_task(parent_session_id, task) }
    async fn wait(
        &self,
        parent_session_id: &str,
        child_session_ids: &[String],
        condition: WaitCondition,
        timeout: Duration,
    ) -> Result<ChildAgentWaitOutcome> { self.0.wait(parent_session_id, child_session_ids, condition, timeout).await }
    fn cancel_descendants(
        &self,
        parent_session_id: &str,
        child_session_ids: &[String],
    ) -> Result<usize> { self.0.cancel_descendants(parent_session_id, child_session_ids) }
    fn attach_continuation(&self, child: &str) -> mpsc::Receiver<String> { self.0.attach_continuation(child) }
    fn followup_task(&self, parent: &str, child: &str, prompt: String) -> Result<()> { self.0.followup_task(parent, child, prompt) }
    fn turn_cancellation(
        &self,
        child: &str,
    ) -> Result<tokio_util::sync::CancellationToken> { self.0.turn_cancellation(child) }
    fn begin_followup(&self, parent: &str, child: &str, prompt: &str) -> Result<()> { self.0.begin_followup(parent, child, prompt) }
    async fn team_snapshots(
        &self,
        context: &ToolUseContext,
    ) -> Result<Vec<ChildAgentTaskSnapshot>> { self.0.team_snapshots(context).await }
    async fn send_team_message(
        &self,
        context: &ToolUseContext,
        target: &str,
        turn: u64,
        kind: TeamMessageKind,
        payload: String,
    ) -> Result<TeamMessage> { self.0.send_team_message(context, target, turn, kind, payload).await }
    async fn wait_team_activity(
        &self,
        context: &ToolUseContext,
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<TeamActivity> { self.0.wait_team_activity(context, timeout, cancellation).await }
}

#[derive(Clone)]
pub struct TaskCoordinatorHost(pub(crate) crate::session::task_session::TaskSessionCoordinator);
impl From<crate::session::task_session::TaskSessionCoordinator> for TaskCoordinatorHost { fn from(value: crate::session::task_session::TaskSessionCoordinator) -> Self { Self(value) } }
#[async_trait::async_trait]
impl TaskCoordinator for TaskCoordinatorHost {
    fn register(
        &self,
        parent_session_id: &str,
        task_session_id: &str,
        title: Option<String>,
    ) -> Result<()> { self.0.register(parent_session_id, task_session_id, title) }
    fn restore(&self, snapshot: TaskSessionSnapshot) { self.0.restore(snapshot) }
    fn mark_running(
        &self,
        parent_session_id: &str,
        task_session_id: &str,
    ) -> Result<bool> { self.0.mark_running(parent_session_id, task_session_id) }
    fn begin_turn(&self, parent_session_id: &str, task_session_id: &str) -> Result<()> { self.0.begin_turn(parent_session_id, task_session_id) }
    fn finish_turn(
        &self,
        parent_session_id: &str,
        task_session_id: &str,
        error: Option<String>,
        persist: impl FnOnce(&TaskSessionSnapshot) -> Result<()>,
    ) -> Result<bool> { self.0.finish_turn(parent_session_id, task_session_id, error, persist) }
    fn snapshots(
        &self,
        parent_session_id: &str,
        task_session_ids: &[String],
    ) -> Result<Vec<TaskSessionSnapshot>> { self.0.snapshots(parent_session_id, task_session_ids) }
    async fn wait(
        &self,
        parent_session_id: &str,
        task_session_ids: &[String],
        condition: TaskSessionWaitCondition,
        timeout: Duration,
    ) -> Result<TaskSessionWaitOutcome> { self.0.wait(parent_session_id, task_session_ids, condition, timeout).await }
}

use std::{future::Future, path::PathBuf};
use vw_api_transport::id::TenantScope;
use session_port::{SessionScope, SessionSnapshot, chat_stream::GatewayChatStreamCollected};
use vw_api_chat::chat::GatewayChatStreamRequest;
use config_agent::DelegateAgentConfig;
use config_tools::MultimodalConfig;
use agent_security::SecurityPolicy;
use model_types::{ChatMessage, Provider};
use tool_types::Tool;
use crate::observability::traits::Observer;
use crate::session::{child_agent, runtime, ui_store};

pub struct ToolAgentHost;
#[derive(Clone)]
pub struct HostChildRun(child_agent::CreatedChildAgentRun);
#[async_trait::async_trait]
impl ChildRun for HostChildRun {
    fn child_session_id(&self) -> &str { &self.0.child_session_id }
    async fn prepare_followup(&mut self, prompt: String) -> Result<()> { self.0.prepare_followup(prompt).await }
}
#[async_trait::async_trait]
impl AgentHost for ToolAgentHost {
    type Coordinator = ChildCoordinatorHost;
    type Run = HostChildRun;
    type Error = crate::gateway::ApiError;
    fn coordinator() -> Self::Coordinator { child_agent::coordinator::global_child_agent_coordinator().into() }
    fn normalize_path(path: PathBuf) -> PathBuf { crate::util::filesystem::normalize_path(path) }
    fn resolve_model_ref(config: &DelegateAgentConfig) -> Option<String> { crate::agent::resolve_model_ref(config).map(|m| format!("{}/{}", m.provider_id, m.model_id)) }
    fn task_summary(prompt: &str) -> String { child_agent::task_summary(prompt) }
    fn tenant() -> TenantScope { ui_store::current_tenant_scope() }
    fn with_tenant<F: Future + Send>(tenant: TenantScope, operation: F) -> impl Future<Output = F::Output> + Send where F::Output: Send { ui_store::with_tenant_scope(tenant, operation) }
    async fn session_depth(directory: String, parent: String) -> Result<usize> { child_agent::session_depth(directory, parent).await }
    async fn create(input: ChildAgentRunInput, fork: ForkTurns) -> Result<Self::Run> { child_agent::create_with_fork(input, fork).await.map(HostChildRun) }
    async fn run(created: Self::Run, executor: Option<ChildStreamExecutor<Self::Error>>) -> Result<ChildAgentRunResult> {
        if let Some(executor) = executor { return child_agent::run_created_with_stream_executor(created.0, move |request| executor(request)).await; }
        child_agent::run_created(created.0).await
    }
    async fn cancel(created: Self::Run, reason: &str) -> Result<ChildAgentRunResult> { child_agent::cancel_created(created.0, reason).await }
}
#[async_trait::async_trait]
impl SessionHost for ToolAgentHost {
    type Coordinator = TaskCoordinatorHost;
    type Error = crate::gateway::ApiError;
    fn coordinator() -> Self::Coordinator { crate::session::task_session::global_task_session_coordinator().into() }
    fn tenant() -> TenantScope { ui_store::current_tenant_scope() }
    fn now_ms() -> u64 { crate::session::session::now_ms() }
    fn with_tenant<F: Future + Send>(tenant: TenantScope, operation: F) -> impl Future<Output = F::Output> + Send where F::Output: Send { ui_store::with_tenant_scope(tenant, operation) }
    fn load_states(tenant: &TenantScope, project: &str) -> Result<std::collections::HashMap<String, TaskSessionSnapshot>> { ui_store::load_visible_task_states(tenant, project) }
    fn save_state(tenant: &TenantScope, project: &str, snapshot: &TaskSessionSnapshot) -> Result<()> { ui_store::save_visible_task_state(tenant, project, snapshot) }
    fn committed(tenant: &TenantScope, project: &str, session: &str) -> Result<bool> { ui_store::has_committed_first_response(tenant, project, session) }
    async fn load(scope: SessionScope, session: &str) -> Result<SessionSnapshot> { Ok(runtime::load_session(scope, session).await?) }
    async fn reserve(scope: SessionScope, parent: String) -> Result<SessionSnapshot> { Ok(runtime::reserve_visible_task_session(scope, parent).await?) }
    async fn visible(scope: SessionScope, parent: &str) -> Result<Vec<session_info::Info>> { Ok(runtime::visible_tasks_for_parent(scope, parent).await?) }
    async fn collect(directory: String, request: GatewayChatStreamRequest) -> std::result::Result<GatewayChatStreamCollected, Self::Error> { crate::gateway::api::handlers::session::stream::collect_gateway_chat_stream(directory, request).await }
}
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl DelegateHost for ToolAgentHost {
    fn tools(parent: &[Arc<dyn Tool>], config: &DelegateAgentConfig, trusted: &[String]) -> Vec<Box<dyn Tool>> { super::delegated_tools::build_agentic_tools(parent, config, trusted) }
    async fn run_loop(provider: &dyn Provider, history: &mut Vec<ChatMessage>, tools: &[Box<dyn Tool>], observer: &dyn Observer, provider_name: &str, model: &str, temperature: f64, multimodal: &MultimodalConfig, max_iterations: usize, security: Arc<SecurityPolicy>) -> Result<String> {
        crate::agent::loop_::run_tool_call_loop(provider, history, tools, observer, provider_name, model, temperature, true, None, "delegate", multimodal, max_iterations, None, None, None, Some(security), &[]).await
    }
}

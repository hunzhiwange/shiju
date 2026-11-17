//! Fail-closed fixture: specification/input tests must never reach host services.
use crate::ports::{AgentHost, ChildCoordinator, ChildRun, ChildStreamExecutor};
use anyhow::Result;
use std::{future::Future, path::PathBuf, sync::{Arc, Mutex}, time::Duration};
use tokio::sync::{mpsc, OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;
type ToolUseContext = tool_context::context::ToolUseContext<InputContext>;
#[path = "input_context.rs"]
mod input_context;
use input_context::InputContext;
use session_port::child_agent::*;
use vw_api_transport::id::TenantScope;
use config_agent::DelegateAgentConfig;

pub(super) struct InputHost;
#[derive(Clone)]
pub(super) struct InputCoordinator;
#[derive(Clone)]
pub(super) struct InputRun;

#[allow(unused_variables)]
#[async_trait::async_trait]
impl ChildCoordinator for InputCoordinator {
    type Context = InputContext;
    fn team_capacity(&self, session_id: &str) -> Arc<Semaphore> {
        panic!("input/specification test reached host service: team_capacity")
    }
    fn track_execution(
        &self,
        child: &str,
        permit: OwnedSemaphorePermit,
    ) -> Arc<Mutex<Option<OwnedSemaphorePermit>>> {
        panic!("input/specification test reached host service: track_execution")
    }
    fn pause_execution(&self, session: &str) -> bool {
        panic!("input/specification test reached host service: pause_execution")
    }
    async fn resume_execution(&self, session: &str) -> Result<()> {
        panic!("input/specification test reached host service: resume_execution")
    }
    fn finish_execution(&self, session: &str) {
        panic!("input/specification test reached host service: finish_execution")
    }
    fn register(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
        agent: &str,
        task: &str,
    ) -> Result<CancellationToken> {
        panic!("input/specification test reached host service: register")
    }
    fn mark_running(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
    ) -> Result<bool> {
        panic!("input/specification test reached host service: mark_running")
    }
    fn mark_succeeded(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
        output: impl Into<String>,
    ) -> Result<bool> {
        panic!("input/specification test reached host service: mark_succeeded")
    }
    fn mark_failed(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
        error: impl Into<String>,
    ) -> Result<bool> {
        panic!("input/specification test reached host service: mark_failed")
    }
    fn finish_turn(
        &self,
        parent: &str,
        child: &str,
        turn: u64,
        output: Option<String>,
        error: Option<String>,
    ) -> Result<bool> {
        panic!("input/specification test reached host service: finish_turn")
    }
    fn cancel(&self, parent_session_id: &str, child_session_id: &str) -> Result<bool> {
        panic!("input/specification test reached host service: cancel")
    }
    fn snapshots(
        &self,
        parent_session_id: &str,
        child_session_ids: &[String],
    ) -> Result<Vec<ChildAgentTaskSnapshot>> {
        panic!("input/specification test reached host service: snapshots")
    }
    fn matching_ancestor_task(
        &self,
        parent_session_id: &str,
        task: &str,
    ) -> Result<Option<ChildAgentTaskSnapshot>> {
        panic!("input/specification test reached host service: matching_ancestor_task")
    }
    async fn wait(
        &self,
        parent_session_id: &str,
        child_session_ids: &[String],
        condition: WaitCondition,
        timeout: Duration,
    ) -> Result<ChildAgentWaitOutcome> {
        panic!("input/specification test reached host service: wait")
    }
    fn cancel_descendants(
        &self,
        parent_session_id: &str,
        child_session_ids: &[String],
    ) -> Result<usize> {
        panic!("input/specification test reached host service: cancel_descendants")
    }
    fn attach_continuation(&self, child: &str) -> mpsc::Receiver<String> {
        panic!("input/specification test reached host service: attach_continuation")
    }
    fn followup_task(&self, parent: &str, child: &str, prompt: String) -> Result<()> {
        panic!("input/specification test reached host service: followup_task")
    }
    fn turn_cancellation(
        &self,
        child: &str,
    ) -> Result<tokio_util::sync::CancellationToken> {
        panic!("input/specification test reached host service: turn_cancellation")
    }
    fn begin_followup(&self, parent: &str, child: &str, prompt: &str) -> Result<()> {
        panic!("input/specification test reached host service: begin_followup")
    }
    async fn team_snapshots(
        &self,
        context: &ToolUseContext,
    ) -> Result<Vec<ChildAgentTaskSnapshot>> {
        panic!("input/specification test reached host service: team_snapshots")
    }
    async fn send_team_message(
        &self,
        context: &ToolUseContext,
        target: &str,
        turn: u64,
        kind: TeamMessageKind,
        payload: String,
    ) -> Result<TeamMessage> {
        panic!("input/specification test reached host service: send_team_message")
    }
    async fn wait_team_activity(
        &self,
        context: &ToolUseContext,
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<TeamActivity> {
        panic!("input/specification test reached host service: wait_team_activity")
    }
}

#[allow(unused_variables)]
#[async_trait::async_trait]
impl ChildRun for InputRun {
    fn child_session_id(&self) -> &str {
        panic!("input/specification test reached host service: child_session_id")
    }
    async fn prepare_followup(&mut self, prompt: String) -> Result<()> {
        panic!("input/specification test reached host service: prepare_followup")
    }
}

#[allow(unused_variables)]
#[async_trait::async_trait]
impl AgentHost for InputHost {
    type Context = InputContext;
    type Coordinator = InputCoordinator;
    type Run = InputRun;
    type Error = String;
    fn coordinator() -> Self::Coordinator {
        InputCoordinator
    }
    fn normalize_path(path: PathBuf) -> PathBuf {
        path
    }
    fn resolve_model_ref(config: &DelegateAgentConfig) -> Option<String> {
        panic!("input/specification test reached host service: resolve_model_ref")
    }
    fn task_summary(prompt: &str) -> String {
        panic!("input/specification test reached host service: task_summary")
    }
    fn tenant() -> TenantScope {
        panic!("input/specification test reached host service: tenant")
    }
    fn with_tenant<F: Future + Send>(tenant: TenantScope, operation: F) -> impl Future<Output = F::Output> + Send where F::Output: Send {
        async move { panic!("input/specification test reached host service: with_tenant") }
    }
    async fn session_depth(directory: String, parent: String) -> Result<usize> {
        panic!("input/specification test reached host service: session_depth")
    }
    async fn create(input: ChildAgentRunInput, fork: ForkTurns) -> Result<Self::Run> {
        panic!("input/specification test reached host service: create")
    }
    async fn run(created: Self::Run, executor: Option<ChildStreamExecutor<Self::Error>>) -> Result<ChildAgentRunResult> {
        panic!("input/specification test reached host service: run")
    }
    async fn cancel(created: Self::Run, reason: &str) -> Result<ChildAgentRunResult> {
        panic!("input/specification test reached host service: cancel")
    }
}

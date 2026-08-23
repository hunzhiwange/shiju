//! 管理 Matrix 工作节点运行时，将房间事件与调度运行状态、心跳及回执相互同步。

use std::collections::HashSet;
use std::path::{Path, PathBuf};
#[cfg(feature = "channel-matrix")]
use std::time::Duration;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use serde_json::json;
use vw_api_types::agent_scheduler::{
    AgentSchedulerMatrixDispatchEventDto, AgentSchedulerScopeDto,
    AgentSchedulerWorkerRunEventKindDto, AgentSchedulerWorkerRunEventRequest,
    AgentSchedulerWorkspaceContextDto,
};
#[cfg(feature = "channel-matrix")]
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
#[cfg(feature = "channel-matrix")]
use vw_gateway_client::{GatewayAuth, GatewayClient, GatewayEndpoint};

use crate::agent::loop_::scrub_credentials;
#[cfg(feature = "channel-matrix")]
use crate::agent_scheduler::domain::AgentSchedulerContext;
#[cfg(feature = "channel-matrix")]
use crate::agent_scheduler::domain::AgentWorkerStatus;
use crate::agent_scheduler::domain::{RunId, TaskId, WorkerName};
#[cfg(feature = "channel-matrix")]
use crate::agent_scheduler::runner::gateway_executor::GatewayLocalAgentExecutor;
use crate::agent_scheduler::runner::local::{
    LocalAgentExecutionOutput, LocalAgentExecutionRequest, LocalAgentExecutor, LocalAgentWorkspace,
    LocalAgentWorkspaceRecycleRequest, LocalAgentWorkspaceReleaseRequest,
    LocalAgentWorkspaceRequest,
};
#[cfg(feature = "channel-matrix")]
use crate::agent_scheduler::store::AgentSchedulerStore;
use crate::agent_scheduler::store::workers::StoredAgentWorker;
#[cfg(feature = "channel-matrix")]
use crate::agent_scheduler::store::workers::UpdateAgentWorkerLifecycleRequest;
use crate::app::agent::config::Config;

pub const MATRIX_WORKER_RUNTIME: &str = "matrix";
pub const MATRIX_WORKER_DESIRED_RUNNING: &str = "Running";
#[cfg(feature = "channel-matrix")]
const MATRIX_WORKER_CONTAINER_RUNNING: &str = "running";
#[cfg(feature = "channel-matrix")]
const MATRIX_WORKER_CONTAINER_STOPPED: &str = "stopped";
#[cfg(feature = "channel-matrix")]
const MATRIX_WORKER_HEARTBEAT_SECONDS: u64 = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixAgentWorkerRuntimeConfig {
    pub platform_id: String,
    pub company_id: String,
    pub project_id: String,
    pub project_path: PathBuf,
    pub worker_id: WorkerName,
    pub matrix_user_id: String,
    pub room_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfiguredMatrixWorkerComponent {
    pub project_path: PathBuf,
    pub worker_id: WorkerName,
    pub matrix_user_id: String,
    pub room_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixAgentDispatchEvent {
    pub matrix_event_id: String,
    pub room_id: String,
    pub dispatch: AgentSchedulerMatrixDispatchEventDto,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatrixAgentWorkerRuntimeSummary {
    pub started: usize,
    pub ignored: usize,
    pub duplicates: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatrixWorkerRunEventApiRequest {
    pub platform_id: String,
    pub company_id: String,
    pub project_id: String,
    pub task_id: String,
    pub run_id: String,
    pub body: AgentSchedulerWorkerRunEventRequest,
}

#[async_trait]
pub trait MatrixAgentDispatchEventSource {
    async fn next_dispatch_event(&mut self) -> Result<Option<MatrixAgentDispatchEvent>>;
}

#[async_trait]
pub trait MatrixWorkerRunEventApi: Send + Sync {
    async fn send_worker_run_event(&self, request: MatrixWorkerRunEventApiRequest) -> Result<()>;
}

pub fn matrix_worker_desired_state_is_running(worker: &StoredAgentWorker) -> bool {
    worker
        .desired_state
        .as_deref()
        .is_some_and(|value| value.trim() == MATRIX_WORKER_DESIRED_RUNNING)
}

pub fn matrix_worker_runtime_is_matrix(worker: &StoredAgentWorker) -> bool {
    worker
        .runtime
        .as_deref()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case(MATRIX_WORKER_RUNTIME))
}

pub fn configured_matrix_worker_from_stored(
    project_path: &Path,
    worker: StoredAgentWorker,
) -> Result<ConfiguredMatrixWorkerComponent> {
    if !matrix_worker_runtime_is_matrix(&worker) {
        bail!("worker {} runtime must be matrix", worker.id.as_str());
    }
    if !matrix_worker_desired_state_is_running(&worker) {
        bail!("worker {} desired_state must be Running", worker.id.as_str());
    }
    let matrix_user_id = required_worker_field(
        worker.matrix_user_id.as_deref(),
        worker.id.as_str(),
        "matrix_user_id",
    )?;
    let room_id = required_worker_field(worker.room_id.as_deref(), worker.id.as_str(), "room_id")?;

    Ok(ConfiguredMatrixWorkerComponent {
        project_path: project_path.to_path_buf(),
        worker_id: worker.id,
        matrix_user_id,
        room_id,
    })
}

pub async fn run_configured_matrix_worker_until_shutdown(
    config: Config,
    project_path: PathBuf,
    worker_id: WorkerName,
) -> Result<()> {
    run_configured_matrix_worker_until_shutdown_for_tenant(
        config,
        TenantScope::default(),
        project_path,
        worker_id,
    )
    .await
}

pub async fn run_configured_matrix_worker_until_shutdown_for_tenant(
    config: Config,
    tenant_scope: TenantScope,
    project_path: PathBuf,
    worker_id: WorkerName,
) -> Result<()> {
    let worker =
        run_configured_matrix_worker_for_tenant(config, tenant_scope, project_path, worker_id);
    tokio::pin!(worker);

    tokio::select! {
        result = &mut worker => result,
        signal = wait_for_matrix_worker_shutdown_signal() => {
            signal?;
            Ok(())
        }
    }
}

pub async fn run_configured_matrix_worker(
    config: Config,
    project_path: PathBuf,
    worker_id: WorkerName,
) -> Result<()> {
    run_configured_matrix_worker_for_tenant(config, TenantScope::default(), project_path, worker_id)
        .await
}

pub async fn run_configured_matrix_worker_for_tenant(
    config: Config,
    tenant_scope: TenantScope,
    project_path: PathBuf,
    worker_id: WorkerName,
) -> Result<()> {
    #[cfg(not(feature = "channel-matrix"))]
    {
        let _ = (config, tenant_scope, project_path, worker_id);
        bail!("Matrix worker runtime requires the channel-matrix feature");
    }

    #[cfg(feature = "channel-matrix")]
    {
        run_configured_matrix_worker_impl(config, tenant_scope, project_path, worker_id).await
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StartedOnlyMatrixAgentExecutor;

impl LocalAgentExecutor for StartedOnlyMatrixAgentExecutor {
    fn execute(&self, _request: LocalAgentExecutionRequest) -> Result<LocalAgentExecutionOutput> {
        bail!("Matrix worker runtime was not configured with a local agent executor")
    }
}

pub struct MatrixAgentWorkerRuntime<'a, A, E = StartedOnlyMatrixAgentExecutor>
where
    A: MatrixWorkerRunEventApi + ?Sized,
    E: LocalAgentExecutor + ?Sized,
{
    config: MatrixAgentWorkerRuntimeConfig,
    api: &'a A,
    executor: Option<&'a E>,
    processed_event_ids: HashSet<String>,
}

impl<'a, A> MatrixAgentWorkerRuntime<'a, A, StartedOnlyMatrixAgentExecutor>
where
    A: MatrixWorkerRunEventApi + ?Sized,
{
    pub fn new(config: MatrixAgentWorkerRuntimeConfig, api: &'a A) -> Result<Self> {
        validate_runtime_config(&config)?;
        Ok(Self { config, api, executor: None, processed_event_ids: HashSet::new() })
    }
}

impl<'a, A, E> MatrixAgentWorkerRuntime<'a, A, E>
where
    A: MatrixWorkerRunEventApi + ?Sized,
    E: LocalAgentExecutor + ?Sized,
{
    pub fn new_with_executor(
        config: MatrixAgentWorkerRuntimeConfig,
        api: &'a A,
        executor: &'a E,
    ) -> Result<Self> {
        validate_runtime_config(&config)?;
        Ok(Self { config, api, executor: Some(executor), processed_event_ids: HashSet::new() })
    }

    pub async fn run_until_idle<S>(
        &mut self,
        source: &mut S,
    ) -> Result<MatrixAgentWorkerRuntimeSummary>
    where
        S: MatrixAgentDispatchEventSource + ?Sized,
    {
        let mut summary = MatrixAgentWorkerRuntimeSummary::default();
        while let Some(event) = source.next_dispatch_event().await? {
            match self.handle_dispatch_event(event).await? {
                MatrixAgentDispatchEventOutcome::Started => summary.started += 1,
                MatrixAgentDispatchEventOutcome::Ignored => summary.ignored += 1,
                MatrixAgentDispatchEventOutcome::Duplicate => summary.duplicates += 1,
            }
        }
        Ok(summary)
    }

    pub async fn handle_dispatch_event(
        &mut self,
        event: MatrixAgentDispatchEvent,
    ) -> Result<MatrixAgentDispatchEventOutcome> {
        let matrix_event_id = event.matrix_event_id.trim();
        if matrix_event_id.is_empty() {
            tracing::warn!("Ignoring Matrix scheduler dispatch event without Matrix event_id");
            return Ok(MatrixAgentDispatchEventOutcome::Ignored);
        }
        if self.processed_event_ids.contains(matrix_event_id) {
            tracing::debug!(
                matrix_event_id,
                worker_id = self.config.worker_id.as_str(),
                "Matrix worker ignored duplicate scheduler dispatch event"
            );
            return Ok(MatrixAgentDispatchEventOutcome::Duplicate);
        }
        if let Err(error) = event.dispatch.validate_for_matrix_room_event() {
            tracing::warn!(
                matrix_event_id,
                error = %error,
                "Ignoring invalid Matrix scheduler dispatch event"
            );
            return Ok(MatrixAgentDispatchEventOutcome::Ignored);
        }
        if !self.dispatch_matches_runtime(&event) {
            return Ok(MatrixAgentDispatchEventOutcome::Ignored);
        }

        let request = self.started_request(&event);
        self.api.send_worker_run_event(request).await.with_context(|| {
            format!("report Matrix worker started event for run {}", event.dispatch.run_id)
        })?;
        self.processed_event_ids.insert(matrix_event_id.to_string());
        if let Some(executor) = self.executor {
            self.execute_dispatch_event(&event, executor).await?;
        }
        Ok(MatrixAgentDispatchEventOutcome::Started)
    }

    fn dispatch_matches_runtime(&self, event: &MatrixAgentDispatchEvent) -> bool {
        let dispatch = &event.dispatch;
        if dispatch.platform_id != self.config.platform_id
            || dispatch.company_id != self.config.company_id
            || dispatch.project_id != self.config.project_id
        {
            tracing::debug!(
                matrix_event_id = event.matrix_event_id.as_str(),
                dispatch_platform_id = dispatch.platform_id.as_str(),
                dispatch_company_id = dispatch.company_id.as_str(),
                dispatch_project_id = dispatch.project_id.as_str(),
                worker_platform_id = self.config.platform_id.as_str(),
                worker_company_id = self.config.company_id.as_str(),
                worker_project_id = self.config.project_id.as_str(),
                "Matrix worker ignored dispatch for another scope"
            );
            return false;
        }
        if dispatch.worker_id != self.config.worker_id.as_str() {
            tracing::debug!(
                matrix_event_id = event.matrix_event_id.as_str(),
                dispatch_worker_id = dispatch.worker_id.as_str(),
                worker_id = self.config.worker_id.as_str(),
                "Matrix worker ignored dispatch for another worker"
            );
            return false;
        }
        if dispatch.matrix_user_id != self.config.matrix_user_id {
            tracing::debug!(
                matrix_event_id = event.matrix_event_id.as_str(),
                dispatch_matrix_user_id = dispatch.matrix_user_id.as_str(),
                matrix_user_id = self.config.matrix_user_id.as_str(),
                "Matrix worker ignored dispatch for another Matrix user"
            );
            return false;
        }
        if event.room_id != self.config.room_id || dispatch.room_id != self.config.room_id {
            tracing::debug!(
                matrix_event_id = event.matrix_event_id.as_str(),
                event_room_id = event.room_id.as_str(),
                dispatch_room_id = dispatch.room_id.as_str(),
                worker_room_id = self.config.room_id.as_str(),
                "Matrix worker ignored dispatch for another room"
            );
            return false;
        }
        true
    }

    fn started_request(&self, event: &MatrixAgentDispatchEvent) -> MatrixWorkerRunEventApiRequest {
        MatrixWorkerRunEventApiRequest {
            platform_id: event.dispatch.platform_id.clone(),
            company_id: event.dispatch.company_id.clone(),
            project_id: event.dispatch.project_id.clone(),
            task_id: event.dispatch.task_id.clone(),
            run_id: event.dispatch.run_id.clone(),
            body: AgentSchedulerWorkerRunEventRequest {
                platform_id: event.dispatch.platform_id.clone(),
                company_id: event.dispatch.company_id.clone(),
                project_id: event.dispatch.project_id.clone(),
                worker_id: self.config.worker_id.as_str().to_string(),
                event: AgentSchedulerWorkerRunEventKindDto::Started,
                message: Some("Matrix worker accepted scheduler dispatch".to_string()),
                result: None,
                error_message: None,
                workspace: None,
                payload: json!({
                    "source": "matrix_dispatch",
                    "matrix_event_id": event.matrix_event_id.as_str(),
                    "room_id": event.room_id.as_str(),
                }),
                now_ms: None,
                lease_expires_at_ms: None,
            },
        }
    }

    async fn execute_dispatch_event(
        &self,
        event: &MatrixAgentDispatchEvent,
        executor: &E,
    ) -> Result<()> {
        let task_id = TaskId::new(event.dispatch.task_id.clone())
            .with_context(|| format!("parse Matrix dispatch task_id {}", event.dispatch.task_id))?;
        let run_id = RunId::new(event.dispatch.run_id.clone())
            .with_context(|| format!("parse Matrix dispatch run_id {}", event.dispatch.run_id))?;
        let workspace_request = LocalAgentWorkspaceRequest {
            project_path: self.config.project_path.clone(),
            task_id: task_id.clone(),
            worker_id: self.config.worker_id.clone(),
            run_id: run_id.clone(),
        };
        let workspace = match executor.lease_workspace(workspace_request) {
            Ok(workspace) => workspace,
            Err(error) => {
                self.send_failed_event(
                    event,
                    None,
                    format!("Matrix worker failed to acquire task worktree: {error}"),
                )
                .await?;
                return Ok(());
            }
        };

        match self
            .execute_leased_dispatch_event(event, executor, &task_id, &run_id, &workspace)
            .await
        {
            Ok(MatrixWorkspaceExecutionOutcome::Succeeded) => {
                executor
                    .release_workspace(LocalAgentWorkspaceReleaseRequest {
                        project_path: self.config.project_path.clone(),
                        task_id,
                        worker_id: self.config.worker_id.clone(),
                        run_id,
                        workspace,
                    })
                    .context("release completed Matrix worker task worktree")?;
                Ok(())
            }
            Ok(MatrixWorkspaceExecutionOutcome::Failed { taint_reason }) => {
                executor
                    .recycle_workspace(LocalAgentWorkspaceRecycleRequest {
                        project_path: self.config.project_path.clone(),
                        task_id,
                        worker_id: self.config.worker_id.clone(),
                        run_id,
                        workspace,
                        taint_reason,
                    })
                    .context("recycle failed Matrix worker task worktree")?;
                Ok(())
            }
            Err(error) => {
                let taint_reason = format!("Matrix worker control-plane failure: {error}");
                executor
                    .recycle_workspace(LocalAgentWorkspaceRecycleRequest {
                        project_path: self.config.project_path.clone(),
                        task_id,
                        worker_id: self.config.worker_id.clone(),
                        run_id,
                        workspace,
                        taint_reason,
                    })
                    .context("recycle Matrix worker task worktree after control-plane failure")?;
                Err(error)
            }
        }
    }

    async fn execute_leased_dispatch_event(
        &self,
        event: &MatrixAgentDispatchEvent,
        executor: &E,
        task_id: &TaskId,
        run_id: &RunId,
        workspace: &LocalAgentWorkspace,
    ) -> Result<MatrixWorkspaceExecutionOutcome> {
        self.api
            .send_worker_run_event(self.heartbeat_request(event, workspace))
            .await
            .with_context(|| {
                format!("report Matrix worker heartbeat for run {}", run_id.as_str())
            })?;

        let execution = executor.execute(LocalAgentExecutionRequest {
            project_path: self.config.project_path.clone(),
            task_id: task_id.clone(),
            worker_id: self.config.worker_id.clone(),
            run_id: run_id.clone(),
            title: matrix_execution_title(&event.dispatch),
            prompt: matrix_execution_prompt(&event.dispatch),
            model: matrix_execution_model(&event.dispatch),
            agent: optional_trimmed_string(event.dispatch.agent.as_deref()),
            acp_agent: optional_trimmed_string(event.dispatch.acp_agent.as_deref()),
            reasoning_effort: optional_trimmed_string(event.dispatch.acp_agent.as_deref())
                .and(event.dispatch.reasoning_effort),
            workspace: workspace.clone(),
        });

        let output = match execution {
            Ok(output) => output,
            Err(error) => {
                let message =
                    scrub_credentials(&format!("Matrix worker ACP execution failed: {error}"));
                self.send_failed_event(event, Some(workspace), message.clone()).await?;
                return Ok(MatrixWorkspaceExecutionOutcome::Failed { taint_reason: message });
            }
        };

        self.api
            .send_worker_run_event(self.submitted_request(event, workspace, &output.raw_output))
            .await
            .with_context(|| {
                format!("report Matrix worker submitted event for run {}", run_id.as_str())
            })?;
        self.api
            .send_worker_run_event(self.completed_request(event, workspace, &output.raw_output))
            .await
            .with_context(|| {
                format!("report Matrix worker completed event for run {}", run_id.as_str())
            })?;
        Ok(MatrixWorkspaceExecutionOutcome::Succeeded)
    }

    fn heartbeat_request(
        &self,
        event: &MatrixAgentDispatchEvent,
        workspace: &LocalAgentWorkspace,
    ) -> MatrixWorkerRunEventApiRequest {
        self.worker_event_request(
            event,
            AgentSchedulerWorkerRunEventKindDto::Heartbeat,
            Some("Matrix worker heartbeat during ACP execution".to_string()),
            None,
            None,
            Some(workspace),
            json!({
                "source": "matrix_worker",
                "matrix_event_id": event.matrix_event_id.as_str(),
                "room_id": event.room_id.as_str(),
            }),
        )
    }

    fn submitted_request(
        &self,
        event: &MatrixAgentDispatchEvent,
        workspace: &LocalAgentWorkspace,
        output: &str,
    ) -> MatrixWorkerRunEventApiRequest {
        self.worker_event_request(
            event,
            AgentSchedulerWorkerRunEventKindDto::Submitted,
            Some("Matrix worker submitted ACP execution result".to_string()),
            Some(output.to_string()),
            None,
            Some(workspace),
            json!({
                "source": "matrix_worker",
                "matrix_event_id": event.matrix_event_id.as_str(),
                "room_id": event.room_id.as_str(),
            }),
        )
    }

    fn completed_request(
        &self,
        event: &MatrixAgentDispatchEvent,
        workspace: &LocalAgentWorkspace,
        output: &str,
    ) -> MatrixWorkerRunEventApiRequest {
        self.worker_event_request(
            event,
            AgentSchedulerWorkerRunEventKindDto::Completed,
            Some("Matrix worker completed ACP execution".to_string()),
            Some(output.to_string()),
            None,
            Some(workspace),
            json!({
                "source": "matrix_worker",
                "matrix_event_id": event.matrix_event_id.as_str(),
                "room_id": event.room_id.as_str(),
            }),
        )
    }

    async fn send_failed_event(
        &self,
        event: &MatrixAgentDispatchEvent,
        workspace: Option<&LocalAgentWorkspace>,
        error_message: String,
    ) -> Result<()> {
        self.api
            .send_worker_run_event(self.worker_event_request(
                event,
                AgentSchedulerWorkerRunEventKindDto::Failed,
                Some("Matrix worker failed ACP execution".to_string()),
                None,
                Some(error_message),
                workspace,
                json!({
                    "source": "matrix_worker",
                    "matrix_event_id": event.matrix_event_id.as_str(),
                    "room_id": event.room_id.as_str(),
                }),
            ))
            .await
            .with_context(|| {
                format!("report Matrix worker failed event for run {}", event.dispatch.run_id)
            })?;
        Ok(())
    }

    fn worker_event_request(
        &self,
        event: &MatrixAgentDispatchEvent,
        kind: AgentSchedulerWorkerRunEventKindDto,
        message: Option<String>,
        result: Option<String>,
        error_message: Option<String>,
        workspace: Option<&LocalAgentWorkspace>,
        payload: serde_json::Value,
    ) -> MatrixWorkerRunEventApiRequest {
        MatrixWorkerRunEventApiRequest {
            platform_id: event.dispatch.platform_id.clone(),
            company_id: event.dispatch.company_id.clone(),
            project_id: event.dispatch.project_id.clone(),
            task_id: event.dispatch.task_id.clone(),
            run_id: event.dispatch.run_id.clone(),
            body: AgentSchedulerWorkerRunEventRequest {
                platform_id: event.dispatch.platform_id.clone(),
                company_id: event.dispatch.company_id.clone(),
                project_id: event.dispatch.project_id.clone(),
                worker_id: self.config.worker_id.as_str().to_string(),
                event: kind,
                message,
                result,
                error_message,
                workspace: workspace.map(workspace_context_dto),
                payload,
                now_ms: None,
                lease_expires_at_ms: None,
            },
        }
    }
}

enum MatrixWorkspaceExecutionOutcome {
    Succeeded,
    Failed { taint_reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatrixAgentDispatchEventOutcome {
    Started,
    Ignored,
    Duplicate,
}

#[async_trait]
impl MatrixWorkerRunEventApi for vw_gateway_client::GatewayClient {
    async fn send_worker_run_event(&self, request: MatrixWorkerRunEventApiRequest) -> Result<()> {
        let tenant = TenantScope::new(request.platform_id.clone(), request.company_id.clone())?;
        let scope = AgentSchedulerScopeDto::new(ProjectScope::new(
            tenant,
            ProjectId::from(request.project_id.as_str()),
        )?);
        self.agent_scheduler_worker_run_event(
            &scope,
            &request.task_id,
            &request.run_id,
            &request.body,
        )
        .await
        .map_err(anyhow::Error::msg)?;
        Ok(())
    }
}

#[cfg(feature = "channel-matrix")]
pub struct MatrixAgentWorkerChannelConfig {
    pub homeserver: String,
    pub access_token: String,
    pub room_id: String,
    pub allowed_users: Vec<String>,
    pub user_id: Option<String>,
    pub device_id: Option<String>,
    pub instance_id: Option<String>,
    pub vibewindow_dir: Option<std::path::PathBuf>,
    pub mention_only: bool,
}

#[cfg(feature = "channel-matrix")]
impl std::fmt::Debug for MatrixAgentWorkerChannelConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MatrixAgentWorkerChannelConfig")
            .field("homeserver", &self.homeserver)
            .field("access_token", &"<redacted>")
            .field("room_id", &self.room_id)
            .field("allowed_users", &self.allowed_users)
            .field("user_id", &self.user_id)
            .field("device_id", &self.device_id)
            .field("instance_id", &self.instance_id)
            .field("vibewindow_dir", &self.vibewindow_dir)
            .field("mention_only", &self.mention_only)
            .finish()
    }
}

#[cfg(feature = "channel-matrix")]
impl MatrixAgentWorkerChannelConfig {
    pub fn into_channel(self) -> crate::channels::matrix::MatrixChannel {
        crate::channels::matrix::MatrixChannel::new_with_session_hint_instance_id_and_vibewindow_dir(
            self.homeserver,
            self.access_token,
            self.room_id,
            self.allowed_users,
            self.user_id,
            self.device_id,
            self.instance_id,
            self.vibewindow_dir,
        )
        .with_mention_only(self.mention_only)
    }
}

#[cfg(feature = "channel-matrix")]
impl<'a, A, E> MatrixAgentWorkerRuntime<'a, A, E>
where
    A: MatrixWorkerRunEventApi + ?Sized,
    E: LocalAgentExecutor + ?Sized,
{
    pub async fn run_matrix_channel(
        &mut self,
        channel: crate::channels::matrix::MatrixChannel,
    ) -> Result<MatrixAgentWorkerRuntimeSummary> {
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let listener =
            tokio::spawn(async move { channel.listen_agent_scheduler_dispatch_events(tx).await });
        let mut source = MatrixChannelDispatchEventSource::new(rx);
        let result = self.run_until_idle(&mut source).await;
        if result.is_err() {
            listener.abort();
        }
        let summary = result?;
        listener.await.context("join Matrix scheduler dispatch listener")??;
        Ok(summary)
    }
}

#[cfg(feature = "channel-matrix")]
struct MatrixChannelDispatchEventSource {
    receiver: tokio::sync::mpsc::Receiver<
        crate::channels::matrix::MatrixAgentSchedulerDispatchTimelineEvent,
    >,
}

#[cfg(feature = "channel-matrix")]
impl MatrixChannelDispatchEventSource {
    fn new(
        receiver: tokio::sync::mpsc::Receiver<
            crate::channels::matrix::MatrixAgentSchedulerDispatchTimelineEvent,
        >,
    ) -> Self {
        Self { receiver }
    }
}

#[cfg(feature = "channel-matrix")]
#[async_trait]
impl MatrixAgentDispatchEventSource for MatrixChannelDispatchEventSource {
    async fn next_dispatch_event(&mut self) -> Result<Option<MatrixAgentDispatchEvent>> {
        Ok(self.receiver.recv().await.map(|event| MatrixAgentDispatchEvent {
            matrix_event_id: event.matrix_event_id,
            room_id: event.room_id,
            dispatch: event.dispatch,
        }))
    }
}

#[cfg(feature = "channel-matrix")]
async fn run_configured_matrix_worker_impl(
    config: Config,
    tenant_scope: TenantScope,
    project_path: PathBuf,
    worker_id: WorkerName,
) -> Result<()> {
    let store = AgentSchedulerStore::new();
    let project = crate::app::agent::project::from_directory(&tenant_scope, &project_path)
        .await
        .with_context(|| format!("resolve project for Matrix worker: {}", project_path.display()))?
        .0;
    let project_scope = ProjectScope::new(tenant_scope, ProjectId::from(project.id.as_str()))?;
    let context = AgentSchedulerContext::new(
        AgentSchedulerScopeDto::new(project_scope),
        project_path.clone(),
    )?;
    let worker = store
        .load_worker(&context, &worker_id)?
        .with_context(|| format!("worker not found: {}", worker_id.as_str()))?;
    let component = configured_matrix_worker_from_stored(&project_path, worker)?;
    let scope = context.scope().project_scope();
    let runtime_config = MatrixAgentWorkerRuntimeConfig {
        platform_id: scope.platform_id().as_ref().to_string(),
        company_id: scope.company_id().as_ref().to_string(),
        project_id: scope.project_id().as_ref().to_string(),
        project_path: component.project_path.clone(),
        worker_id: component.worker_id.clone(),
        matrix_user_id: component.matrix_user_id.clone(),
        room_id: component.room_id.clone(),
    };
    let channel_config = matrix_worker_channel_config(&config, &component)?;
    let channel = channel_config.into_channel();
    let api = gateway_client_from_config(&config)?;
    let executor = GatewayLocalAgentExecutor::new(store.clone());

    tracing::info!(
        worker_id = component.worker_id.as_str(),
        matrix_user_id = component.matrix_user_id.as_str(),
        room_id = component.room_id.as_str(),
        "Starting Matrix agent scheduler worker"
    );

    let mut lifecycle =
        MatrixWorkerLifecycleGuard::start(store.clone(), context.clone(), component.worker_id)?;
    let heartbeat = spawn_matrix_worker_heartbeat(
        store,
        lifecycle.context.clone(),
        lifecycle.worker_id.clone(),
    );
    let mut runtime = MatrixAgentWorkerRuntime::new_with_executor(runtime_config, &api, &executor)?;
    let run_result = runtime.run_matrix_channel(channel).await.map(|_| ());
    heartbeat.abort();
    let _ = heartbeat.await;
    let stop_result = lifecycle.stop_now();

    match (run_result, stop_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
        (Err(error), Err(stop_error)) => {
            tracing::warn!(
                worker_id = lifecycle.worker_id.as_str(),
                error = %stop_error,
                "failed to mark Matrix worker stopped after runtime error"
            );
            Err(error)
        }
    }
}

#[cfg(feature = "channel-matrix")]
fn matrix_worker_channel_config(
    config: &Config,
    component: &ConfiguredMatrixWorkerComponent,
) -> Result<MatrixAgentWorkerChannelConfig> {
    let SelectedMatrixConfig { matrix, instance_id } = select_matrix_config(config, component)
        .with_context(|| {
            format!("resolve Matrix channel config for worker {}", component.worker_id.as_str())
        })?;

    Ok(MatrixAgentWorkerChannelConfig {
        homeserver: matrix.homeserver.clone(),
        access_token: matrix.access_token.clone(),
        room_id: component.room_id.clone(),
        allowed_users: matrix.allowed_users.clone(),
        user_id: Some(component.matrix_user_id.clone()),
        device_id: matrix.device_id.clone(),
        instance_id,
        vibewindow_dir: config.config_path.parent().map(Path::to_path_buf),
        mention_only: matrix.mention_only,
    })
}

#[cfg(feature = "channel-matrix")]
struct SelectedMatrixConfig<'a> {
    matrix: &'a crate::app::agent::config::schema::MatrixConfig,
    instance_id: Option<String>,
}

#[cfg(feature = "channel-matrix")]
fn select_matrix_config<'a>(
    config: &'a Config,
    component: &ConfiguredMatrixWorkerComponent,
) -> Result<SelectedMatrixConfig<'a>> {
    if !config.channels_config.matrix_enabled {
        anyhow::bail!("Matrix channel is disabled");
    }

    let default_id = vw_config_types::channels::chat::DEFAULT_CHANNEL_INSTANCE_ID;
    let candidates = config
        .channels_config
        .matrix
        .as_ref()
        .map(|matrix| SelectedMatrixConfig { matrix, instance_id: Some(default_id.to_string()) })
        .into_iter()
        .chain(config.channels_config.matrix_instances.iter().map(|instance| {
            SelectedMatrixConfig {
                matrix: &instance.config,
                instance_id: Some(instance.id.clone()),
            }
        }))
        .collect::<Vec<_>>();

    candidates
        .iter()
        .find(|candidate| {
            candidate
                .matrix
                .user_id
                .as_deref()
                .is_some_and(|user_id| user_id.trim() == component.matrix_user_id)
                && candidate.matrix.room_id.trim() == component.room_id
        })
        .or_else(|| {
            candidates.iter().find(|candidate| {
                candidate
                    .matrix
                    .user_id
                    .as_deref()
                    .is_some_and(|user_id| user_id.trim() == component.matrix_user_id)
            })
        })
        .or_else(|| {
            candidates.iter().find(|candidate| candidate.matrix.room_id.trim() == component.room_id)
        })
        .or_else(|| candidates.first())
        .map(|candidate| SelectedMatrixConfig {
            matrix: candidate.matrix,
            instance_id: candidate.instance_id.clone(),
        })
        .with_context(|| "channels_config.matrix is required for Matrix worker")
}

#[cfg(feature = "channel-matrix")]
fn gateway_client_from_config(config: &Config) -> Result<GatewayClient> {
    let mut endpoint = GatewayEndpoint::new(config.gateway.host.clone(), config.gateway.port);
    if let Some(raw_skey) = first_raw_gateway_skey(config) {
        endpoint = endpoint.with_auth(GatewayAuth { skey: Some(raw_skey) });
    }
    GatewayClient::new(endpoint).map_err(anyhow::Error::msg)
}

#[cfg(feature = "channel-matrix")]
fn first_raw_gateway_skey(config: &Config) -> Option<String> {
    config
        .gateway
        .skeys
        .iter()
        .filter(|entry| entry.enabled)
        .filter_map(|entry| entry.skey.as_deref())
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(feature = "channel-matrix")]
struct MatrixWorkerLifecycleGuard {
    store: AgentSchedulerStore,
    context: AgentSchedulerContext,
    worker_id: WorkerName,
    active: bool,
}

#[cfg(feature = "channel-matrix")]
impl MatrixWorkerLifecycleGuard {
    fn start(
        store: AgentSchedulerStore,
        context: AgentSchedulerContext,
        worker_id: WorkerName,
    ) -> Result<Self> {
        update_matrix_worker_lifecycle(
            &store,
            &context,
            &worker_id,
            AgentWorkerStatus::Idle,
            MATRIX_WORKER_CONTAINER_RUNNING,
        )?;
        Ok(Self { store, context, worker_id, active: true })
    }

    fn stop_now(&mut self) -> Result<()> {
        if self.active {
            update_matrix_worker_lifecycle(
                &self.store,
                &self.context,
                &self.worker_id,
                AgentWorkerStatus::Unavailable,
                MATRIX_WORKER_CONTAINER_STOPPED,
            )?;
            self.active = false;
        }
        Ok(())
    }
}

#[cfg(feature = "channel-matrix")]
impl Drop for MatrixWorkerLifecycleGuard {
    fn drop(&mut self) {
        if self.active
            && let Err(error) = self.stop_now()
        {
            tracing::warn!(
                worker_id = self.worker_id.as_str(),
                error = %error,
                "failed to mark Matrix worker stopped during lifecycle cleanup"
            );
        }
    }
}

#[cfg(feature = "channel-matrix")]
fn spawn_matrix_worker_heartbeat(
    store: AgentSchedulerStore,
    context: AgentSchedulerContext,
    worker_id: WorkerName,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(Duration::from_secs(MATRIX_WORKER_HEARTBEAT_SECONDS));
        loop {
            interval.tick().await;
            if let Err(error) = update_matrix_worker_lifecycle(
                &store,
                &context,
                &worker_id,
                AgentWorkerStatus::Idle,
                MATRIX_WORKER_CONTAINER_RUNNING,
            ) {
                tracing::warn!(
                    worker_id = worker_id.as_str(),
                    error = %error,
                    "failed to heartbeat Matrix worker lifecycle"
                );
            }
        }
    })
}

#[cfg(feature = "channel-matrix")]
fn update_matrix_worker_lifecycle(
    store: &AgentSchedulerStore,
    context: &AgentSchedulerContext,
    worker_id: &WorkerName,
    status: AgentWorkerStatus,
    container_state: &str,
) -> Result<()> {
    let now_ms = current_timestamp_ms();
    let idle_since_ms = (status == AgentWorkerStatus::Idle).then_some(now_ms);
    store.update_worker_lifecycle(
        context,
        UpdateAgentWorkerLifecycleRequest {
            worker_id: worker_id.clone(),
            status,
            container_state: container_state.to_string(),
            idle_since_ms,
            last_seen_at_ms: now_ms,
            last_heartbeat_at: Some(chrono::Utc::now().to_rfc3339()),
        },
    )?;
    Ok(())
}

fn validate_runtime_config(config: &MatrixAgentWorkerRuntimeConfig) -> Result<()> {
    validate_non_empty("platform_id", &config.platform_id)?;
    validate_non_empty("company_id", &config.company_id)?;
    validate_non_empty("project_id", &config.project_id)?;
    validate_non_empty("project_path", config.project_path.to_string_lossy().as_ref())?;
    validate_non_empty("matrix_user_id", &config.matrix_user_id)?;
    validate_non_empty("room_id", &config.room_id)?;
    Ok(())
}

fn workspace_context_dto(workspace: &LocalAgentWorkspace) -> AgentSchedulerWorkspaceContextDto {
    AgentSchedulerWorkspaceContextDto {
        slot_id: workspace.slot_id.clone(),
        path: workspace.path.clone(),
        branch: workspace.branch.clone(),
    }
}

fn matrix_execution_title(dispatch: &AgentSchedulerMatrixDispatchEventDto) -> String {
    format!("Matrix scheduled task {}", dispatch.task_id)
}

fn matrix_execution_prompt(dispatch: &AgentSchedulerMatrixDispatchEventDto) -> String {
    format!(
        "Execute the scheduled task described by {}.\n\nTask metadata: {}\nMatrix run: {}\n\nWhen finished, respond with one concise final status.",
        dispatch.spec_path, dispatch.metadata_path, dispatch.run_id
    )
}

fn matrix_execution_model(dispatch: &AgentSchedulerMatrixDispatchEventDto) -> Option<String> {
    optional_trimmed_string(Some(dispatch.model.as_str()))
        .filter(|model| !model.eq_ignore_ascii_case("auto"))
}

fn optional_trimmed_string(value: Option<&str>) -> Option<String> {
    value.map(str::trim).filter(|value| !value.is_empty()).map(str::to_string)
}

fn validate_non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{field} must not be empty");
    }
    Ok(())
}

fn required_worker_field(value: Option<&str>, worker_id: &str, field: &str) -> Result<String> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        bail!("worker {worker_id} requires non-empty {field}");
    };
    Ok(value.to_string())
}

async fn wait_for_matrix_worker_shutdown_signal() -> Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm = signal(SignalKind::terminate())?;
        tokio::select! {
            ctrl_c = tokio::signal::ctrl_c() => {
                ctrl_c?;
                Ok(())
            }
            sigterm_result = sigterm.recv() => match sigterm_result {
                Some(()) => Ok(()),
                None => bail!("SIGTERM signal stream unexpectedly closed"),
            },
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
        Ok(())
    }
}

#[cfg(feature = "channel-matrix")]
fn current_timestamp_ms() -> TimestampMs {
    TimestampMs(chrono::Utc::now().timestamp_millis().max(0))
}

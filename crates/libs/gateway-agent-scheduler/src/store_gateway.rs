//! Agent Scheduler API 路由。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{Map, Value, json};
use tokio::sync::Mutex as AsyncMutex;
use vw_api_chat_identity::ReasoningEffortDto;
use vw_api_agent_scheduler::{
    AgentCapabilityKindDto, AgentEventDto, AgentEventKindDto, AgentOutboxMessageDto,
    AgentOutboxMessageStatusDto, AgentRunDto, AgentRunStatusDto, AgentSchedulerBulkCommandRequest,
    AgentSchedulerBulkCommandResponse, AgentSchedulerCommandKindDto,
    AgentSchedulerCommandRequest as AgentSchedulerProjectCommandRequest,
    AgentSchedulerCommandResponse as AgentSchedulerProjectCommandResponse,
    AgentSchedulerCreateTaskRequest, AgentSchedulerCreateTaskResponse,
    AgentSchedulerDispatchModeDto, AgentSchedulerLegacyImportRequest,
    AgentSchedulerLegacyImportResponse,
    AgentSchedulerOutboxDrainRequest as AgentSchedulerProjectOutboxDrainRequest,
    AgentSchedulerOutboxDrainResponse as AgentSchedulerProjectOutboxDrainResponse,
    AgentSchedulerReplaceTaskGraphRequest, AgentSchedulerTaskArtifactDto,
    AgentSchedulerTaskArtifactsResponse, AgentSchedulerTaskBoardEventDto,
    AgentSchedulerTaskBoardEventsResponse, AgentSchedulerTaskBoardSnapshotReadModelDto,
    AgentSchedulerTaskBoardStatusResponse, AgentSchedulerTaskBoardTaskDto,
    AgentSchedulerTaskBoardTaskReadModelDto, AgentSchedulerTaskBoardWorkflowResponse,
    AgentSchedulerTaskGraphEdgeDto, AgentSchedulerTaskGraphReadinessDto,
    AgentSchedulerTaskGraphResponse, AgentSchedulerTaskGraphTaskDto, AgentSchedulerTaskLogDto,
    AgentSchedulerTaskLogsResponse, AgentSchedulerTaskMetadataUpdateRequest,
    AgentSchedulerTaskMetadataUpdateResponse,
    AgentSchedulerWakeRequest as AgentSchedulerProjectWakeRequest,
    AgentSchedulerWakeResponse as AgentSchedulerProjectWakeResponse,
    AgentSchedulerWorkspaceContextDto, AgentTaskDto, AgentTaskKindDto, AgentTaskStatusDto,
    TaskDependencyNodeDto,
};
use vw_api_transport::common::TimestampMs;
use vw_api_task::{TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusRoleDto};

use task_scheduler::adapters::dispatch::LocalOnlyDispatchAdapter;
use task_scheduler::domain::{
    AgentCapabilityKind, AgentRunStatus, AgentSchedulerContext, AssigneeId, RunId, TaskId, TeamId,
};
use task_scheduler::engine::dispatch::{
    DispatchEngine, DispatchMode, DispatchOutcome, load_dispatch_workflow,
    ready_queued_finite_task, task_lifecycle_allows_dispatch,
};
use task_scheduler::engine::recovery::{RecoveryEngine, RecoveryOutcome};
use task_scheduler::events::AgentEventKind;
use task_scheduler::lifecycle_bridge::{
    LifecycleStatusProjection, status_role_key as lifecycle_status_role_key,
};
use task_scheduler::outbox_dispatcher::OutboxDispatcher;
use task_scheduler::policy::{
    AgentSchedulerPolicyConfig, AgentSchedulerPolicyOutcome, AgentSchedulerPolicyService,
};
use task_scheduler::runner::gateway_executor::{GatewayLocalAgentExecutor, GatewayExecutorHost};
use task_scheduler::runner::local::{
    LocalAgentExecutor, LocalAgentRunner, LocalAgentRunnerOutcome, LocalAgentRunnerRequest,
};
use task_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
use task_scheduler::store::artifacts::StoredTaskArtifact;
use task_scheduler::store::events::{AppendAgentEventRequest, StoredAgentEvent};
use task_scheduler::store::logs::{
    AppendAgentTaskLogRequest, StoredAgentRunWorkspaceContext, StoredAgentTaskLog,
};
use task_scheduler::store::outbox::{OutboxMessageStatus, StoredOutboxMessage};
use task_scheduler::store::runs::StoredAgentRun;
use task_scheduler::store::tasks::{
    AgentTaskDagNode, AgentTaskFilter, NewAgentTask, StoredAgentTask,
};
use task_scheduler::store::{AgentSchedulerStorageContext, AgentSchedulerStore};
use task_scheduler::task_board::AgentTaskBoardService;
use task_scheduler::task_graph::{
    AgentTaskDependency, TaskGraph, TaskGraphReadiness, TaskGraphTaskSnapshot,
};
use gateway_types::ApiError;

use crate::*;
const DEFAULT_EVENT_LIMIT: usize = 100;
const MAX_EVENT_LIMIT: usize = 1_000;
const DEFAULT_OUTBOX_DRAIN_LIMIT: usize = 50;
const MAX_OUTBOX_DRAIN_LIMIT: usize = 500;
const LOCAL_GATEWAY_WORKER_ID: &str = "gateway-local";
const DEFAULT_WORKER_RUN_LEASE_EXTENSION_MS: i64 = 30 * 60 * 1_000;
const WORKER_OUTCOME_SUCCESS: &str = "success";
const WORKER_OUTCOME_FAILURE: &str = "failure";

type ProjectDispatchLock = Arc<AsyncMutex<()>>;

const DEFAULT_PROJECT_WAKE_MAX_CONCURRENT: usize = 3;
const MAX_PROJECT_WAKE_MAX_CONCURRENT: usize = 10;

#[derive(Clone, Default)]
struct ProjectDispatchLocks {
    locks: Arc<Mutex<HashMap<PathBuf, ProjectDispatchLock>>>,
}

impl ProjectDispatchLocks {
    fn lock_for(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> ProjectDispatchLock {
        let mut locks = self.locks.lock().unwrap_or_else(|poison| poison.into_inner());
        Arc::clone(
            locks
                .entry(project_path.storage_project_path().to_path_buf())
                .or_insert_with(|| Arc::new(AsyncMutex::new(()))),
        )
    }
}

#[derive(Clone)]
pub struct StoreBackedAgentSchedulerGateway<H: GatewayExecutorHost> {
    host: std::marker::PhantomData<H>,
    pub store: AgentSchedulerStore,
    executor: Arc<dyn LocalAgentExecutor + Send + Sync>,
    dispatch_locks: ProjectDispatchLocks,
}

impl<H: GatewayExecutorHost> Default for StoreBackedAgentSchedulerGateway<H> {
    fn default() -> Self {
        let store = AgentSchedulerStore::new();
        Self {
            host: std::marker::PhantomData,
            executor: Arc::new(GatewayLocalAgentExecutor::<H>::new(store.clone())),
            dispatch_locks: ProjectDispatchLocks::default(),
            store,
        }
    }
}

impl<H: GatewayExecutorHost> StoreBackedAgentSchedulerGateway<H> {
    #[cfg(any(test, feature = "test-support"))]
    pub fn with_executor<E>(executor: E) -> Self
    where
        E: LocalAgentExecutor + Send + Sync + 'static,
    {
        Self {
            host: std::marker::PhantomData,
            store: AgentSchedulerStore::new(),
            executor: Arc::new(executor),
            dispatch_locks: ProjectDispatchLocks::default(),
        }
    }

    fn project_dispatch_lock(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
    ) -> ProjectDispatchLock {
        self.dispatch_locks.lock_for(project_path)
    }

    fn active_dispatch_finite_task_count(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        _dispatch_mode: AgentSchedulerDispatchModeDto,
    ) -> Result<usize, ApiError> {
        let tasks = self
            .store
            .list_active_tasks(project_path, AgentTaskFilter::default())
            .map_err(map_scheduler_error)?;
        Ok(tasks
            .into_iter()
            .filter(|task| task.kind == AgentTaskKind::Finite && execution_slot_status(task.status))
            .count())
    }
}

impl<H: GatewayExecutorHost> StoreBackedAgentSchedulerGateway<H> {
    fn task_graph_snapshot(
        &self,
        project_id: String,
        project_path: &AgentSchedulerContext,
    ) -> Result<AgentSchedulerTaskGraphResponse, ApiError> {
        self.store.initialize_project(project_path).map_err(map_scheduler_error)?;
        let graph = self.store.load_task_graph(project_path).map_err(map_scheduler_error)?;
        let tasks = graph
            .tasks()
            .map(|task| task_graph_task_to_dto(task, &graph))
            .collect::<Result<Vec<_>, _>>()?;
        let edges = graph
            .dependencies()
            .iter()
            .map(|dependency| AgentSchedulerTaskGraphEdgeDto {
                predecessor_task_id: dependency.predecessor.as_str().to_string(),
                successor_task_id: dependency.successor.as_str().to_string(),
            })
            .collect();
        Ok(AgentSchedulerTaskGraphResponse { project_id: project_id.into(), edges, tasks })
    }
}

#[path = "store_routes.rs"]
mod routes;

impl<H: GatewayExecutorHost> StoreBackedAgentSchedulerGateway<H> {
    async fn apply_policy_tick(
        &self,
        _project_id: Option<&str>,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
        auto_promote_pool_tasks: bool,
    ) -> Result<AgentSchedulerPolicyOutcome, ApiError> {
        let config = AgentSchedulerPolicyConfig {
            auto_promote_pool_tasks,
            ..AgentSchedulerPolicyConfig::default()
        };
        let outcome = AgentSchedulerPolicyService::new(&self.store)
            .apply_project_policy(project_path, now_ms, &config)
            .map_err(map_scheduler_error)?;
        tracing::info!(
            project_path = %project_path.storage_project_path().display(),
            promoted = outcome.promoted_task_ids.len(),
            retries = outcome.retry_task_ids.len(),
            timeout_graces = outcome.timeout_grace_task_ids.len(),
            timed_out = outcome.timed_out_task_ids.len(),
            review = outcome.review_task_ids.len(),
            skipped_review = outcome.skipped_review_task_ids.len(),
            merge = outcome.merge_task_ids.len(),
            blocked = outcome.blocked_task_ids.len(),
            "agent scheduler policy tick applied"
        );
        Ok(outcome)
    }

    async fn apply_project_command(
        &self,
        project_id: &str,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        command: AgentSchedulerCommandKindDto,
        dispatch_mode: AgentSchedulerDispatchModeDto,
        max_concurrent: Option<u32>,
        now_ms: TimestampMs,
    ) -> Result<Option<StoredAgentRun>, ApiError> {
        match command {
            AgentSchedulerCommandKindDto::Start | AgentSchedulerCommandKindDto::Promote => {
                if start_command_is_idempotent_for_status(task.status) {
                    return Ok(None);
                }
                self.ensure_queued_for_local_run(project_path, task, now_ms)?;
                self.run_or_dispatch_project_task(
                    project_path,
                    &task.id,
                    now_ms,
                    dispatch_mode,
                    max_concurrent,
                )
                .await
            }
            AgentSchedulerCommandKindDto::Retry => {
                if task.status != AgentTaskStatus::Failed {
                    return Err(ApiError::bad_request("retry requires a failed task"));
                }
                self.store
                    .transition_task_status(project_path, &task.id, AgentTaskStatus::Queued, now_ms)
                    .map_err(map_scheduler_command_error)?
                    .ok_or_else(|| ApiError::not_found("task not found"))?;
                self.run_or_dispatch_project_task(
                    project_path,
                    &task.id,
                    now_ms,
                    dispatch_mode,
                    max_concurrent,
                )
                .await
            }
            AgentSchedulerCommandKindDto::Pause => {
                self.transition_for_command(
                    project_path,
                    &task.id,
                    AgentTaskStatus::Paused,
                    now_ms,
                )?;
                Ok(None)
            }
            AgentSchedulerCommandKindDto::Cancel => {
                self.transition_for_command(
                    project_path,
                    &task.id,
                    AgentTaskStatus::Cancelled,
                    now_ms,
                )?;
                Ok(None)
            }
            AgentSchedulerCommandKindDto::Archive => {
                self.transition_for_command(
                    project_path,
                    &task.id,
                    AgentTaskStatus::Archived,
                    now_ms,
                )?;
                Ok(None)
            }
            AgentSchedulerCommandKindDto::Merge => {
                self.merge_project_task(project_id, project_path, task, now_ms).await?;
                Ok(None)
            }
            AgentSchedulerCommandKindDto::Resume
            | AgentSchedulerCommandKindDto::Move
            | AgentSchedulerCommandKindDto::UpdateMetadata
            | AgentSchedulerCommandKindDto::SoftDelete
            | AgentSchedulerCommandKindDto::Restore
            | AgentSchedulerCommandKindDto::ImportLegacyTask => Err(ApiError::bad_request(
                format!("unsupported agent scheduler command: {}", command_kind_key(command)),
            )),
        }
    }

    async fn apply_task_board_command(
        &self,
        project_id: &str,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        request: AgentSchedulerProjectCommandRequest,
        now_ms: TimestampMs,
    ) -> Result<AgentSchedulerProjectCommandResponse, ApiError> {
        let service = AgentTaskBoardService::new(&self.store);
        let command = request.command;
        let outcome = match command {
            AgentSchedulerCommandKindDto::Move => {
                let payload = request.payload.move_task.ok_or_else(|| {
                    ApiError::bad_request("move command requires move_task payload")
                })?;
                service
                    .move_task(project_path, task_id, payload, request.reason.as_deref(), now_ms)
                    .map_err(map_task_board_error)?
            }
            AgentSchedulerCommandKindDto::UpdateMetadata => {
                let patch = request.payload.metadata_update.ok_or_else(|| {
                    ApiError::bad_request(
                        "update_metadata command requires metadata_update payload",
                    )
                })?;
                service
                    .update_metadata(
                        project_path,
                        task_id,
                        patch,
                        request.reason.as_deref(),
                        now_ms,
                    )
                    .map_err(map_task_board_error)?
            }
            AgentSchedulerCommandKindDto::Archive => service
                .archive_task(project_path, task_id, request.reason.as_deref(), now_ms)
                .map_err(map_task_board_error)?,
            AgentSchedulerCommandKindDto::SoftDelete => service
                .soft_delete_task(project_path, task_id, request.reason.as_deref(), now_ms)
                .map_err(map_task_board_error)?,
            AgentSchedulerCommandKindDto::Restore => service
                .restore_task(project_path, task_id, request.reason.as_deref(), now_ms)
                .map_err(map_task_board_error)?,
            AgentSchedulerCommandKindDto::ImportLegacyTask => {
                let legacy = request.payload.legacy_import.ok_or_else(|| {
                    ApiError::bad_request(
                        "import_legacy_task command requires legacy_import payload",
                    )
                })?;
                let import = service
                    .import_legacy_tasks(project_path, None, vec![legacy], false, now_ms)
                    .map_err(map_task_board_error)?;
                let task = import.tasks.into_iter().next();
                let event = import.events.into_iter().next();
                return Ok(AgentSchedulerProjectCommandResponse {
                    accepted: true,
                    task: task
                        .map(|task| {
                            task_board_task_to_dto_with_latest_run(
                                &self.store,
                                project_id,
                                project_path,
                                task,
                            )
                        })
                        .transpose()?,
                    event: event
                        .map(|event| task_board_event_to_dto(project_id, event))
                        .transpose()?,
                });
            }
            AgentSchedulerCommandKindDto::Start
            | AgentSchedulerCommandKindDto::Retry
            | AgentSchedulerCommandKindDto::Pause
            | AgentSchedulerCommandKindDto::Resume
            | AgentSchedulerCommandKindDto::Cancel
            | AgentSchedulerCommandKindDto::Promote
            | AgentSchedulerCommandKindDto::Merge => {
                return Err(ApiError::bad_request(format!(
                    "unsupported task board command dispatch: {}",
                    command_kind_key(command)
                )));
            }
        }
        .ok_or_else(|| ApiError::not_found("task not found"))?;
        let task = task_board_task_to_dto_with_latest_run(
            &self.store,
            project_id,
            project_path,
            outcome.task,
        )?;
        let event = task_board_event_to_dto(project_id, outcome.event)?;
        Ok(AgentSchedulerProjectCommandResponse {
            accepted: true,
            task: Some(task),
            event: Some(event),
        })
    }

    async fn merge_project_task(
        &self,
        _project_id: &str,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        now_ms: TimestampMs,
    ) -> Result<(), ApiError> {
        let message = "merge requires an explicit workflow using the permission-checked git_merge tool; configure and run that workflow";
        self.append_merge_diagnostic_log(project_path, task, now_ms, message);
        Err(ApiError::bad_request(message))
    }

    fn append_merge_diagnostic_log(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        now_ms: TimestampMs,
        message: impl Into<String>,
    ) {
        let message = message.into();
        if let Err(error) = self.store.append_task_log(
            project_path,
            AppendAgentTaskLogRequest {
                task_id: task.id.clone(),
                run_id: None,
                timestamp_ms: now_ms,
                from_status: None,
                to_status: None,
                from_status_key: None,
                to_status_key: None,
                message: message.clone(),
            },
        ) {
            tracing::warn!(
                error = %error,
                task_id = task.id.as_str(),
                message = %message,
                "failed to append agent scheduler merge diagnostic log"
            );
        }
    }

    async fn run_or_dispatch_project_task(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        now_ms: TimestampMs,
        dispatch_mode: AgentSchedulerDispatchModeDto,
        max_concurrent: Option<u32>,
    ) -> Result<Option<StoredAgentRun>, ApiError> {
        tracing::info!(
            project_path = %project_path.storage_project_path().display(),
            task_id = task_id.as_str(),
            dispatch_mode = ?dispatch_mode,
            now_ms = now_ms.0,
            "agent scheduler project command dispatch requested"
        );
        let dispatch_lock = self.project_dispatch_lock(project_path);
        let _dispatch_guard = dispatch_lock.lock().await;
        let max_concurrent = normalize_project_wake_max_concurrent(max_concurrent);
        let active_count = self.active_dispatch_finite_task_count(project_path, dispatch_mode)?;
        if active_count >= max_concurrent {
            tracing::info!(
                project_path = %project_path.storage_project_path().display(),
                task_id = task_id.as_str(),
                dispatch_mode = ?dispatch_mode,
                active_count,
                max_concurrent,
                "agent scheduler project command skipped because concurrency limit is reached"
            );
            return Ok(None);
        }
        let task = self
            .store
            .load_task(project_path, task_id)
            .map_err(map_scheduler_error)?
            .ok_or_else(|| ApiError::not_found("task not found"))?;
        let workflow =
            load_dispatch_workflow(&self.store, project_path).map_err(map_scheduler_error)?;
        if !task_lifecycle_allows_dispatch(project_path, &task, &workflow) {
            return Ok(None);
        }
        if dispatch_mode == AgentSchedulerDispatchModeDto::Local {
            return self
                .run_local_task(owned_scheduler_context(project_path)?, task_id.clone(), now_ms)
                .await
                .map(|outcome| outcome.map(|outcome| outcome.run));
        }

        let outcome =
            self.dispatch_project_task_once(project_path, task_id, now_ms, dispatch_mode).await?;
        match outcome {
            Some(outcome) => {
                tracing::info!(
                    project_path = %project_path.storage_project_path().display(),
                    task_id = outcome.task_id.as_str(),
                    run_id = outcome.run_id.as_str(),
                    outbox_message_id = ?outcome.outbox_message_id.as_deref(),
                    "agent scheduler project command dispatch outcome"
                );
                self.store.load_run(project_path, &outcome.run_id).map_err(map_scheduler_error)
            }
            None => {
                tracing::info!(
                    project_path = %project_path.storage_project_path().display(),
                    task_id = task_id.as_str(),
                    dispatch_mode = ?dispatch_mode,
                    "agent scheduler project command dispatch produced no outcome"
                );
                Ok(None)
            }
        }
    }

    async fn dispatch_project_once(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
        dispatch_mode: AgentSchedulerDispatchModeDto,
    ) -> Result<Option<DispatchOutcome>, ApiError> {
        tracing::info!(
            project_path = %project_path.storage_project_path().display(),
            dispatch_mode = ?dispatch_mode,
            now_ms = now_ms.0,
            "agent scheduler dispatch project once"
        );
        DispatchEngine::new(&self.store)
            .with_dispatch_mode(dispatch_mode_from_dto(dispatch_mode))
            .dispatch_once(project_path, now_ms)
            .await
            .map_err(map_scheduler_error)
    }

    async fn dispatch_project_task_once(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        now_ms: TimestampMs,
        dispatch_mode: AgentSchedulerDispatchModeDto,
    ) -> Result<Option<DispatchOutcome>, ApiError> {
        tracing::info!(
            project_path = %project_path.storage_project_path().display(),
            task_id = task_id.as_str(),
            dispatch_mode = ?dispatch_mode,
            now_ms = now_ms.0,
            "agent scheduler dispatch project task once"
        );
        DispatchEngine::new(&self.store)
            .with_dispatch_mode(dispatch_mode_from_dto(dispatch_mode))
            .dispatch_task_once(project_path, task_id, now_ms)
            .await
            .map_err(map_scheduler_error)
    }

    fn ensure_queued_for_local_run(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        now_ms: TimestampMs,
    ) -> Result<(), ApiError> {
        match task.status {
            AgentTaskStatus::Queued => Ok(()),
            AgentTaskStatus::Draft => {
                self.store
                    .transition_task_status(project_path, &task.id, AgentTaskStatus::Queued, now_ms)
                    .map_err(map_scheduler_command_error)?
                    .ok_or_else(|| ApiError::not_found("task not found"))?;
                Ok(())
            }
            _ => Err(ApiError::bad_request(format!(
                "command requires queued task, got {}",
                task.status.as_str()
            ))),
        }
    }

    fn transition_for_command(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task_id: &TaskId,
        to_status: AgentTaskStatus,
        now_ms: TimestampMs,
    ) -> Result<(), ApiError> {
        self.store
            .transition_task_status(project_path, task_id, to_status, now_ms)
            .map_err(map_scheduler_command_error)?
            .ok_or_else(|| ApiError::not_found("task not found"))?;
        Ok(())
    }

    async fn run_local_task(
        &self,
        project_path: AgentSchedulerContext,
        task_id: TaskId,
        now_ms: TimestampMs,
    ) -> Result<Option<LocalAgentRunnerOutcome>, ApiError> {
        tracing::info!(
            project_path = %project_path.storage_project_path().display(),
            task_id = task_id.as_str(),
            now_ms = now_ms.0,
            "agent scheduler local task start requested"
        );
        let request = LocalAgentRunnerRequest { task_id, now_ms };
        let start_store = self.store.clone();
        let start_executor = Arc::clone(&self.executor);
        let start_project_path = project_path.clone();
        let start_request = request.clone();
        let started = tokio::task::spawn_blocking(move || {
            let runner = LocalAgentRunner::new(&start_store, start_executor.as_ref());
            runner.start_once(&start_project_path, start_request)
        })
        .await
        .map_err(|error| {
            ApiError::internal(format!("agent scheduler local runner join failed: {error}"))
        })?
        .map_err(map_scheduler_error)?;

        if let Some(started_run) = started.clone() {
            tracing::info!(
                project_path = %project_path.storage_project_path().display(),
                task_id = started_run.task_id.as_str(),
                run_id = started_run.run.id.as_str(),
                "agent scheduler local task started; background finish scheduled"
            );
            let finish_store = self.store.clone();
            let finish_executor = Arc::clone(&self.executor);
            let finish_project_path = project_path;
            let finish_task_id = started_run.task_id.clone();
            let finish_run_id = started_run.run.id.clone();
            let finish_request = request;
            let _background = tokio::task::spawn_blocking(move || {
                let runner = LocalAgentRunner::new(&finish_store, finish_executor.as_ref());
                if let Err(error) = runner.finish_started_run(
                    &finish_project_path,
                    &finish_task_id,
                    &finish_run_id,
                    finish_request,
                ) {
                    tracing::warn!(
                        error = %error,
                        task_id = finish_task_id.as_str(),
                        run_id = finish_run_id.as_str(),
                        "agent scheduler local runner background finish failed"
                    );
                }
            });
        } else {
            tracing::info!(
                project_path = %project_path.storage_project_path().display(),
                task_id = request.task_id.as_str(),
                "agent scheduler local task start skipped"
            );
        }

        Ok(started)
    }
}

#[path = "store_projection.rs"]
mod projection;
use projection::*;
pub use projection::{start_command_is_idempotent_for_status, normalize_project_wake_max_concurrent, execution_slot_status, task_board_write_command, owned_scheduler_context, add_ms_or_saturate, command_event_kind, task_board_status_role_from_metadata, task_board_status_key, legacy_task_board_status_key, valid_merge_branch, workspace_context_from_value, event_error_message, command_kind_key, string_field, string_array_field, bool_field, u32_field, i64_field, non_empty_str, optional_text, parse_project_path, map_bad_request, map_scheduler_command_error, map_task_board_error};

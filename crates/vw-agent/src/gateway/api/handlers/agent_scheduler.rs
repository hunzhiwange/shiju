//! Agent Scheduler API 路由。

use std::collections::HashMap;
use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use axum::extract::{Extension, Path as AxumPath, Query};
use axum::response::{
    Sse,
    sse::{Event, KeepAlive},
};
use axum::routing::{patch, post};
use axum::{Json, Router};
use futures_util::stream;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tokio::sync::Mutex as AsyncMutex;
use vw_api_types::ReasoningEffortDto;
use vw_api_types::agent_scheduler::{
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
    AgentSchedulerReplaceTaskGraphRequest, AgentSchedulerScope, AgentSchedulerTaskArtifactDto,
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
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_api_types::task::{TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusRoleDto};

use crate::agent_scheduler::adapters::dispatch::LocalOnlyDispatchAdapter;
use crate::agent_scheduler::domain::{
    AgentCapabilityKind, AgentRunStatus, AgentSchedulerContext, AssigneeId, RunId, TaskId, TeamId,
};
use crate::agent_scheduler::engine::dispatch::{
    DispatchEngine, DispatchMode, DispatchOutcome, load_dispatch_workflow,
    ready_queued_finite_task, task_lifecycle_allows_dispatch,
};
use crate::agent_scheduler::engine::recovery::{RecoveryEngine, RecoveryOutcome};
use crate::agent_scheduler::events::AgentEventKind;
use crate::agent_scheduler::lifecycle_bridge::{
    LifecycleStatusProjection, status_role_key as lifecycle_status_role_key,
};
use crate::agent_scheduler::outbox_dispatcher::OutboxDispatcher;
use crate::agent_scheduler::policy::{
    AgentSchedulerPolicyConfig, AgentSchedulerPolicyOutcome, AgentSchedulerPolicyService,
};
use crate::agent_scheduler::runner::gateway_executor::GatewayLocalAgentExecutor;
#[cfg(test)]
use crate::agent_scheduler::runner::gateway_executor::{
    append_gateway_stream_payload_task_log, gateway_local_agent_stream_request,
};
#[cfg(test)]
use crate::agent_scheduler::runner::local::{
    LocalAgentExecutionOutput, LocalAgentExecutionRequest, LocalAgentWorkspace,
    LocalAgentWorkspaceRequest,
};
use crate::agent_scheduler::runner::local::{
    LocalAgentExecutor, LocalAgentRunner, LocalAgentRunnerOutcome, LocalAgentRunnerRequest,
};
use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::agent_scheduler::store::artifacts::{RecordTaskArtifactRequest, StoredTaskArtifact};
use crate::agent_scheduler::store::events::{AppendAgentEventRequest, StoredAgentEvent};
use crate::agent_scheduler::store::logs::{
    AppendAgentTaskLogRequest, StoredAgentRunWorkspaceContext, StoredAgentTaskLog,
};
use crate::agent_scheduler::store::outbox::{OutboxMessageStatus, StoredOutboxMessage};
use crate::agent_scheduler::store::runs::StoredAgentRun;
use crate::agent_scheduler::store::tasks::{
    AgentTaskDagNode, AgentTaskFilter, NewAgentTask, StoredAgentTask,
};
use crate::agent_scheduler::store::{AgentSchedulerStorageContext, AgentSchedulerStore};
use crate::agent_scheduler::task_board::AgentTaskBoardService;
use crate::agent_scheduler::task_graph::{
    AgentTaskDependency, TaskGraph, TaskGraphReadiness, TaskGraphTaskSnapshot,
};
use crate::app::agent::gateway::ApiError;
use crate::app::agent::project;

const DEFAULT_EVENT_LIMIT: usize = 100;
const MAX_EVENT_LIMIT: usize = 1_000;
const DEFAULT_OUTBOX_DRAIN_LIMIT: usize = 50;
const MAX_OUTBOX_DRAIN_LIMIT: usize = 500;
const LOCAL_GATEWAY_WORKER_ID: &str = "gateway-local";
const DEFAULT_WORKER_RUN_LEASE_EXTENSION_MS: i64 = 30 * 60 * 1_000;
const WORKER_OUTCOME_SUCCESS: &str = "success";
const WORKER_OUTCOME_FAILURE: &str = "failure";

type SharedAgentSchedulerGateway = Arc<dyn AgentSchedulerGateway>;

pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router_with_gateway(Arc::new(StoreBackedAgentSchedulerGateway::default()))
}

fn router_with_gateway<S>(gateway: SharedAgentSchedulerGateway) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let router = Router::new()
        .route(
            "/agent-scheduler/projects/{project_id}/commands",
            post(agent_scheduler_project_command),
        )
        .route(
            "/agent-scheduler/projects/{project_id}/commands/bulk",
            post(agent_scheduler_project_bulk_command),
        )
        .route(
            "/agent-scheduler/projects/{project_id}/workflow",
            axum::routing::get(agent_scheduler_project_workflow),
        )
        .route(
            "/agent-scheduler/projects/{project_id}/tasks",
            axum::routing::get(agent_scheduler_project_tasks)
                .post(agent_scheduler_project_create_task),
        )
        .route(
            "/agent-scheduler/projects/{project_id}/task-graph",
            axum::routing::get(agent_scheduler_project_task_graph)
                .put(agent_scheduler_project_replace_task_graph),
        )
        .route(
            "/agent-scheduler/projects/{project_id}/tasks/{task_id}/metadata",
            patch(agent_scheduler_project_update_task_metadata),
        )
        .route(
            "/agent-scheduler/projects/{project_id}/legacy-import",
            post(agent_scheduler_project_legacy_import),
        )
        .route(
            "/agent-scheduler/projects/{project_id}/tasks/{task_id}/logs",
            axum::routing::get(agent_scheduler_project_logs),
        )
        .route(
            "/agent-scheduler/projects/{project_id}/tasks/{task_id}/artifacts",
            axum::routing::get(agent_scheduler_project_artifacts),
        )
        .route(
            "/agent-scheduler/projects/{project_id}/events",
            axum::routing::get(agent_scheduler_project_events),
        )
        .route(
            "/agent-scheduler/projects/{project_id}/events/stream",
            axum::routing::get(agent_scheduler_project_events_stream),
        )
        .route("/agent-scheduler/projects/{project_id}/wake", post(agent_scheduler_project_wake))
        .route(
            "/agent-scheduler/projects/{project_id}/outbox/drain",
            post(agent_scheduler_project_drain_outbox),
        )
        .layer(Extension(gateway));

    #[cfg(test)]
    let router = router.layer(axum::middleware::from_fn(test_default_tenant_scope));

    router
}

#[cfg(test)]
async fn test_default_tenant_scope(
    mut request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if request.extensions().get::<TenantScope>().is_none() {
        request.extensions_mut().insert(project::default_tenant_scope());
    }
    next.run(request).await
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct AgentSchedulerTasksQuery {
    pub project_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AgentSchedulerTasksResponse {
    pub tasks: Vec<AgentTaskDto>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct AgentSchedulerWakeRequest {
    pub project_path: String,
    pub now_ms: TimestampMs,
    #[serde(default, skip_serializing_if = "is_default_dispatch_mode")]
    pub dispatch_mode: AgentSchedulerDispatchModeDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrent: Option<u32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub auto_promote_pool_tasks: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AgentSchedulerWakeResponse {
    pub dispatched: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbox_message_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct AgentSchedulerRecoveryRequest {
    pub project_path: String,
    pub now_ms: TimestampMs,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AgentSchedulerRecoveryResponse {
    pub graced_run_ids: Vec<String>,
    pub recovered_run_ids: Vec<String>,
    pub released_resource_lease_ids: Vec<String>,
    pub tainted_resource_lease_ids: Vec<String>,
    pub drifted_worker_ids: Vec<String>,
    pub slept_worker_ids: Vec<String>,
    pub outbox_message_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct AgentSchedulerEventsQuery {
    pub project_path: String,
    #[serde(default)]
    pub after_cursor: Option<i64>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AgentSchedulerEventsResponse {
    pub cursor: i64,
    pub events: Vec<AgentEventDto>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct AgentSchedulerOutboxDrainRequest {
    pub project_path: String,
    pub now_ms: TimestampMs,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AgentSchedulerOutboxDrainResponse {
    pub claimed: usize,
    pub messages: Vec<AgentOutboxMessageDto>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct AgentSchedulerProjectEventsQuery {
    #[serde(default)]
    pub after_cursor: Option<i64>,
    #[serde(default)]
    pub limit: Option<usize>,
}

async fn agent_scheduler_create_task(
    Json(body): Json<AgentSchedulerCreateTaskRequest>,
) -> Result<Json<AgentSchedulerCreateTaskResponse>, ApiError> {
    let gateway = StoreBackedAgentSchedulerGateway::default();
    agent_scheduler_create_task_with_gateway(&gateway, body).await.map(Json)
}

async fn agent_scheduler_list_tasks(
    Query(query): Query<AgentSchedulerTasksQuery>,
) -> Result<Json<AgentSchedulerTasksResponse>, ApiError> {
    let gateway = StoreBackedAgentSchedulerGateway::default();
    gateway.list_tasks(query).await.map(Json)
}

async fn agent_scheduler_wake(
    Json(body): Json<AgentSchedulerWakeRequest>,
) -> Result<Json<AgentSchedulerWakeResponse>, ApiError> {
    let gateway = StoreBackedAgentSchedulerGateway::default();
    agent_scheduler_wake_with_gateway(&gateway, body).await.map(Json)
}

async fn agent_scheduler_recovery(
    Json(body): Json<AgentSchedulerRecoveryRequest>,
) -> Result<Json<AgentSchedulerRecoveryResponse>, ApiError> {
    let gateway = StoreBackedAgentSchedulerGateway::default();
    gateway.recover(body).await.map(Json)
}

async fn agent_scheduler_list_events(
    Query(query): Query<AgentSchedulerEventsQuery>,
) -> Result<Json<AgentSchedulerEventsResponse>, ApiError> {
    let gateway = StoreBackedAgentSchedulerGateway::default();
    agent_scheduler_list_events_with_gateway(&gateway, query).await.map(Json)
}

async fn agent_scheduler_drain_outbox(
    Json(body): Json<AgentSchedulerOutboxDrainRequest>,
) -> Result<Json<AgentSchedulerOutboxDrainResponse>, ApiError> {
    let gateway = StoreBackedAgentSchedulerGateway::default();
    agent_scheduler_drain_outbox_with_gateway(&gateway, body).await.map(Json)
}

async fn agent_scheduler_project_command(
    AxumPath(project_id): AxumPath<String>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
    Json(body): Json<AgentSchedulerProjectCommandRequest>,
) -> Result<Json<AgentSchedulerProjectCommandResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    gateway.command(project_id, project_path, body).await.map(Json)
}

async fn agent_scheduler_project_bulk_command(
    AxumPath(project_id): AxumPath<String>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
    Json(body): Json<AgentSchedulerBulkCommandRequest>,
) -> Result<Json<AgentSchedulerBulkCommandResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    gateway.bulk_command(project_id, project_path, body).await.map(Json)
}

async fn agent_scheduler_project_workflow(
    AxumPath(project_id): AxumPath<String>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
) -> Result<Json<AgentSchedulerTaskBoardWorkflowResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    gateway.task_board_workflow(project_id, project_path).await.map(Json)
}

async fn agent_scheduler_project_tasks(
    AxumPath(project_id): AxumPath<String>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
) -> Result<Json<AgentSchedulerTaskBoardStatusResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    gateway.task_board_status(project_id, project_path).await.map(Json)
}

async fn agent_scheduler_project_create_task(
    AxumPath(project_id): AxumPath<String>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
    Json(body): Json<AgentSchedulerCreateTaskRequest>,
) -> Result<Json<AgentSchedulerCreateTaskResponse>, ApiError> {
    let context = resolve_project_context(tenant, &project_id).await?;
    gateway.create_task_scoped(context, body).await.map(Json)
}

async fn agent_scheduler_project_task_graph(
    AxumPath(project_id): AxumPath<String>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
) -> Result<Json<AgentSchedulerTaskGraphResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    gateway.task_graph(project_id, project_path).await.map(Json)
}

async fn agent_scheduler_project_replace_task_graph(
    AxumPath(project_id): AxumPath<String>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
    Json(body): Json<AgentSchedulerReplaceTaskGraphRequest>,
) -> Result<Json<AgentSchedulerTaskGraphResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    gateway.replace_task_graph(project_id, project_path, body).await.map(Json)
}

async fn agent_scheduler_project_update_task_metadata(
    AxumPath((project_id, task_id)): AxumPath<(String, String)>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
    Json(body): Json<AgentSchedulerTaskMetadataUpdateRequest>,
) -> Result<Json<AgentSchedulerTaskMetadataUpdateResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    gateway.update_task_metadata(project_id, project_path, task_id, body).await.map(Json)
}

async fn agent_scheduler_project_legacy_import(
    AxumPath(project_id): AxumPath<String>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
    Json(body): Json<AgentSchedulerLegacyImportRequest>,
) -> Result<Json<AgentSchedulerLegacyImportResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    gateway.import_legacy_tasks(project_id, project_path, body).await.map(Json)
}

async fn agent_scheduler_project_logs(
    AxumPath((project_id, task_id)): AxumPath<(String, String)>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
) -> Result<Json<AgentSchedulerTaskLogsResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    gateway.task_logs(project_path, task_id).await.map(Json)
}

async fn agent_scheduler_project_artifacts(
    AxumPath((project_id, task_id)): AxumPath<(String, String)>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
) -> Result<Json<AgentSchedulerTaskArtifactsResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    gateway.task_artifacts(project_path, task_id).await.map(Json)
}

async fn agent_scheduler_project_events(
    AxumPath(project_id): AxumPath<String>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
    Query(query): Query<AgentSchedulerProjectEventsQuery>,
) -> Result<Json<AgentSchedulerTaskBoardEventsResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    gateway.task_board_events(project_id, project_path, query).await.map(Json)
}

async fn agent_scheduler_project_events_stream(
    AxumPath(project_id): AxumPath<String>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
    Query(query): Query<AgentSchedulerProjectEventsQuery>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    let response = gateway.task_board_events(project_id, project_path, query).await?;
    let events = response.events;
    let stream = stream::iter(events.into_iter().map(|event| {
        let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
        Ok(Event::default().event("agent_scheduler").data(data))
    }));
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

async fn agent_scheduler_project_wake(
    AxumPath(project_id): AxumPath<String>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
    Json(body): Json<AgentSchedulerProjectWakeRequest>,
) -> Result<Json<AgentSchedulerProjectWakeResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    gateway.wake_project(project_id, project_path, body).await.map(Json)
}

async fn agent_scheduler_project_drain_outbox(
    AxumPath(project_id): AxumPath<String>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
    Json(body): Json<AgentSchedulerProjectOutboxDrainRequest>,
) -> Result<Json<AgentSchedulerProjectOutboxDrainResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    gateway.drain_project_outbox(project_path, body).await.map(Json)
}

async fn agent_scheduler_create_task_with_gateway<G>(
    gateway: &G,
    request: AgentSchedulerCreateTaskRequest,
) -> Result<AgentSchedulerCreateTaskResponse, ApiError>
where
    G: AgentSchedulerGateway + ?Sized,
{
    gateway.create_task(request).await
}

async fn agent_scheduler_wake_with_gateway<G>(
    gateway: &G,
    request: AgentSchedulerWakeRequest,
) -> Result<AgentSchedulerWakeResponse, ApiError>
where
    G: AgentSchedulerGateway + ?Sized,
{
    gateway.wake(request).await
}

async fn agent_scheduler_list_events_with_gateway<G>(
    gateway: &G,
    query: AgentSchedulerEventsQuery,
) -> Result<AgentSchedulerEventsResponse, ApiError>
where
    G: AgentSchedulerGateway + ?Sized,
{
    gateway.list_events(query).await
}

async fn agent_scheduler_drain_outbox_with_gateway<G>(
    gateway: &G,
    request: AgentSchedulerOutboxDrainRequest,
) -> Result<AgentSchedulerOutboxDrainResponse, ApiError>
where
    G: AgentSchedulerGateway + ?Sized,
{
    gateway.drain_outbox(request).await
}

#[async_trait]
pub(crate) trait AgentSchedulerGateway: Send + Sync {
    async fn create_task(
        &self,
        request: AgentSchedulerCreateTaskRequest,
    ) -> Result<AgentSchedulerCreateTaskResponse, ApiError>;

    async fn create_task_scoped(
        &self,
        _context: AgentSchedulerContext,
        request: AgentSchedulerCreateTaskRequest,
    ) -> Result<AgentSchedulerCreateTaskResponse, ApiError> {
        self.create_task(request).await
    }

    async fn list_tasks(
        &self,
        query: AgentSchedulerTasksQuery,
    ) -> Result<AgentSchedulerTasksResponse, ApiError>;

    async fn wake(
        &self,
        _request: AgentSchedulerWakeRequest,
    ) -> Result<AgentSchedulerWakeResponse, ApiError> {
        Err(ApiError::internal("agent scheduler wake gateway is not configured"))
    }

    async fn recover(
        &self,
        _request: AgentSchedulerRecoveryRequest,
    ) -> Result<AgentSchedulerRecoveryResponse, ApiError> {
        Err(ApiError::internal("agent scheduler recovery gateway is not configured"))
    }

    async fn list_events(
        &self,
        query: AgentSchedulerEventsQuery,
    ) -> Result<AgentSchedulerEventsResponse, ApiError>;

    async fn drain_outbox(
        &self,
        request: AgentSchedulerOutboxDrainRequest,
    ) -> Result<AgentSchedulerOutboxDrainResponse, ApiError>;

    async fn command(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
        _request: AgentSchedulerProjectCommandRequest,
    ) -> Result<AgentSchedulerProjectCommandResponse, ApiError> {
        Err(ApiError::internal("agent scheduler command gateway is not configured"))
    }

    async fn bulk_command(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
        _request: AgentSchedulerBulkCommandRequest,
    ) -> Result<AgentSchedulerBulkCommandResponse, ApiError> {
        Err(ApiError::internal("agent scheduler bulk command gateway is not configured"))
    }

    async fn task_board_workflow(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
    ) -> Result<AgentSchedulerTaskBoardWorkflowResponse, ApiError> {
        Err(ApiError::internal("agent scheduler task board workflow gateway is not configured"))
    }

    async fn task_board_status(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
    ) -> Result<AgentSchedulerTaskBoardStatusResponse, ApiError> {
        Err(ApiError::internal("agent scheduler task board gateway is not configured"))
    }

    async fn task_graph(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
    ) -> Result<AgentSchedulerTaskGraphResponse, ApiError> {
        Err(ApiError::internal("agent scheduler task graph gateway is not configured"))
    }

    async fn replace_task_graph(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
        _request: AgentSchedulerReplaceTaskGraphRequest,
    ) -> Result<AgentSchedulerTaskGraphResponse, ApiError> {
        Err(ApiError::internal("agent scheduler task graph gateway is not configured"))
    }

    async fn update_task_metadata(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
        _task_id: String,
        _request: AgentSchedulerTaskMetadataUpdateRequest,
    ) -> Result<AgentSchedulerTaskMetadataUpdateResponse, ApiError> {
        Err(ApiError::internal("agent scheduler task board metadata gateway is not configured"))
    }

    async fn import_legacy_tasks(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
        _request: AgentSchedulerLegacyImportRequest,
    ) -> Result<AgentSchedulerLegacyImportResponse, ApiError> {
        Err(ApiError::internal("agent scheduler legacy import gateway is not configured"))
    }

    async fn task_logs(
        &self,
        _project_path: AgentSchedulerContext,
        _task_id: String,
    ) -> Result<AgentSchedulerTaskLogsResponse, ApiError> {
        Err(ApiError::internal("agent scheduler log gateway is not configured"))
    }

    async fn task_artifacts(
        &self,
        _project_path: AgentSchedulerContext,
        _task_id: String,
    ) -> Result<AgentSchedulerTaskArtifactsResponse, ApiError> {
        Err(ApiError::internal("agent scheduler artifact gateway is not configured"))
    }

    async fn task_board_events(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
        _query: AgentSchedulerProjectEventsQuery,
    ) -> Result<AgentSchedulerTaskBoardEventsResponse, ApiError> {
        Err(ApiError::internal("agent scheduler event gateway is not configured"))
    }

    async fn wake_project(
        &self,
        _project_id: String,
        _project_path: AgentSchedulerContext,
        _request: AgentSchedulerProjectWakeRequest,
    ) -> Result<AgentSchedulerProjectWakeResponse, ApiError> {
        Err(ApiError::internal("agent scheduler project wake gateway is not configured"))
    }

    async fn drain_project_outbox(
        &self,
        _project_path: AgentSchedulerContext,
        _request: AgentSchedulerProjectOutboxDrainRequest,
    ) -> Result<AgentSchedulerProjectOutboxDrainResponse, ApiError> {
        Err(ApiError::internal("agent scheduler project outbox gateway is not configured"))
    }
}

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
pub(crate) struct StoreBackedAgentSchedulerGateway {
    pub(crate) store: AgentSchedulerStore,
    executor: Arc<dyn LocalAgentExecutor + Send + Sync>,
    dispatch_locks: ProjectDispatchLocks,
}

impl Default for StoreBackedAgentSchedulerGateway {
    fn default() -> Self {
        let store = AgentSchedulerStore::new();
        Self {
            executor: Arc::new(GatewayLocalAgentExecutor::new(store.clone())),
            dispatch_locks: ProjectDispatchLocks::default(),
            store,
        }
    }
}

impl StoreBackedAgentSchedulerGateway {
    #[cfg(test)]
    fn with_executor<E>(executor: E) -> Self
    where
        E: LocalAgentExecutor + Send + Sync + 'static,
    {
        Self {
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

impl StoreBackedAgentSchedulerGateway {
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

#[async_trait]
impl AgentSchedulerGateway for StoreBackedAgentSchedulerGateway {
    async fn create_task(
        &self,
        _request: AgentSchedulerCreateTaskRequest,
    ) -> Result<AgentSchedulerCreateTaskResponse, ApiError> {
        Err(ApiError::bad_request("explicit agent scheduler scope is required"))
    }

    async fn create_task_scoped(
        &self,
        context: AgentSchedulerContext,
        request: AgentSchedulerCreateTaskRequest,
    ) -> Result<AgentSchedulerCreateTaskResponse, ApiError> {
        let task = new_task_from_request(request)?;
        let task_id = task.id.clone();
        let existed =
            self.store.load_task(&context, &task_id).map_err(map_scheduler_error)?.is_some();
        let stored = self.store.create_task(&context, task).map_err(map_scheduler_error)?;
        if !existed {
            append_task_created_event(&self.store, &context, &stored)?;
        }

        Ok(AgentSchedulerCreateTaskResponse {
            task_id: stored.id.as_str().to_string(),
            task: task_to_dto(stored),
        })
    }

    async fn list_tasks(
        &self,
        query: AgentSchedulerTasksQuery,
    ) -> Result<AgentSchedulerTasksResponse, ApiError> {
        let project_path = parse_project_path(&query.project_path)?;
        let tasks = self
            .store
            .list_active_tasks(&project_path, AgentTaskFilter::default())
            .map_err(map_scheduler_error)?
            .into_iter()
            .map(task_to_dto)
            .collect();
        Ok(AgentSchedulerTasksResponse { tasks })
    }

    async fn wake(
        &self,
        request: AgentSchedulerWakeRequest,
    ) -> Result<AgentSchedulerWakeResponse, ApiError> {
        let project_path = parse_project_path(&request.project_path)?;
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        self.apply_policy_tick(None, &project_path, request.now_ms, false).await?;
        let dispatch_lock = self.project_dispatch_lock(&project_path);
        let _dispatch_guard = dispatch_lock.lock().await;
        let max_concurrent = normalize_project_wake_max_concurrent(request.max_concurrent);
        let active_count =
            self.active_dispatch_finite_task_count(&project_path, request.dispatch_mode)?;
        if active_count >= max_concurrent {
            tracing::info!(
                project_path = %project_path.storage_project_path().display(),
                dispatch_mode = ?request.dispatch_mode,
                active_count,
                max_concurrent,
                "agent scheduler wake skipped because concurrency limit is reached"
            );
            return Ok(AgentSchedulerWakeResponse {
                dispatched: false,
                task_id: None,
                run_id: None,
                worker_id: None,
                outbox_message_id: None,
            });
        }
        let engine = DispatchEngine::new(&self.store)
            .with_dispatch_mode(dispatch_mode_from_dto(request.dispatch_mode));
        let outcome = engine
            .dispatch_once(&project_path, request.now_ms)
            .await
            .map_err(map_scheduler_error)?;
        Ok(wake_response_from_outcome(outcome))
    }

    async fn recover(
        &self,
        request: AgentSchedulerRecoveryRequest,
    ) -> Result<AgentSchedulerRecoveryResponse, ApiError> {
        let project_path = parse_project_path(&request.project_path)?;
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let engine = RecoveryEngine::new(&self.store);
        let outcome = engine
            .reconcile_once(&project_path, request.now_ms)
            .await
            .map_err(map_scheduler_error)?;
        Ok(recovery_response_from_outcome(outcome))
    }

    async fn list_events(
        &self,
        query: AgentSchedulerEventsQuery,
    ) -> Result<AgentSchedulerEventsResponse, ApiError> {
        let project_path = parse_project_path(&query.project_path)?;
        let limit = query.limit.unwrap_or(DEFAULT_EVENT_LIMIT).min(MAX_EVENT_LIMIT);
        let events = self
            .store
            .list_events_after(&project_path, query.after_cursor, limit)
            .map_err(map_scheduler_error)?;
        let cursor = events.last().map_or(query.after_cursor.unwrap_or(0), |event| event.cursor);
        let events = events.into_iter().map(event_to_dto).collect();
        Ok(AgentSchedulerEventsResponse { cursor, events })
    }

    async fn drain_outbox(
        &self,
        request: AgentSchedulerOutboxDrainRequest,
    ) -> Result<AgentSchedulerOutboxDrainResponse, ApiError> {
        let project_path = parse_project_path(&request.project_path)?;
        let limit = request.limit.unwrap_or(DEFAULT_OUTBOX_DRAIN_LIMIT).min(MAX_OUTBOX_DRAIN_LIMIT);
        let dispatch_adapter = LocalOnlyDispatchAdapter;
        let dispatcher = OutboxDispatcher::new(&self.store, &dispatch_adapter);
        let (summary, messages) = dispatcher
            .drain_messages_once(&project_path, request.now_ms, limit)
            .await
            .map_err(map_scheduler_error)?;
        let messages = messages.into_iter().map(outbox_to_dto).collect();
        Ok(AgentSchedulerOutboxDrainResponse { claimed: summary.claimed, messages })
    }

    async fn command(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
        request: AgentSchedulerProjectCommandRequest,
    ) -> Result<AgentSchedulerProjectCommandResponse, ApiError> {
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let task_id = task_id_from_api(&request.task_id)?;
        let now_ms = current_timestamp_ms();
        if task_board_write_command(request.command) {
            return self
                .apply_task_board_command(&project_id, &project_path, &task_id, request, now_ms)
                .await;
        }
        let task_before = self
            .store
            .load_task(&project_path, &task_id)
            .map_err(map_scheduler_error)?
            .ok_or_else(|| ApiError::not_found("task not found"))?;
        let outcome = self
            .apply_project_command(
                &project_id,
                &project_path,
                &task_before,
                request.command,
                request.dispatch_mode,
                request.max_concurrent,
                now_ms,
            )
            .await?;
        let task_after = self
            .store
            .load_task(&project_path, &task_id)
            .map_err(map_scheduler_error)?
            .ok_or_else(|| ApiError::not_found("task not found"))?;
        let event = append_command_event(
            &self.store,
            &project_path,
            &task_after,
            outcome.as_ref(),
            request.command,
            task_before.status,
            request.reason.as_deref(),
            now_ms,
        )?;
        let task = task_board_task_to_dto(
            &self.store,
            &project_id,
            &project_path,
            task_after,
            outcome.as_ref(),
        )?;
        let event = task_board_event_to_dto(&project_id, event)?;
        Ok(AgentSchedulerProjectCommandResponse {
            accepted: true,
            task: Some(task),
            event: Some(event),
        })
    }

    async fn bulk_command(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
        request: AgentSchedulerBulkCommandRequest,
    ) -> Result<AgentSchedulerBulkCommandResponse, ApiError> {
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let mut accepted = 0;
        let mut rejected = 0;
        let mut results = Vec::new();
        for command in request.commands {
            match self.command(project_id.clone(), project_path.clone(), command).await {
                Ok(result) => {
                    if result.accepted {
                        accepted += 1;
                    } else {
                        rejected += 1;
                    }
                    results.push(result);
                }
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        project_id = %project_id,
                        "agent scheduler bulk command item rejected"
                    );
                    rejected += 1;
                    results.push(AgentSchedulerProjectCommandResponse {
                        accepted: false,
                        task: None,
                        event: None,
                    });
                    if request.stop_on_error {
                        break;
                    }
                }
            }
        }
        Ok(AgentSchedulerBulkCommandResponse { accepted, rejected, results })
    }

    async fn task_board_workflow(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
    ) -> Result<AgentSchedulerTaskBoardWorkflowResponse, ApiError> {
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let workflow =
            self.store.load_task_board_workflow(&project_path).map_err(map_scheduler_error)?;
        Ok(AgentSchedulerTaskBoardWorkflowResponse { project_id: project_id.into(), workflow })
    }

    async fn task_board_status(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
    ) -> Result<AgentSchedulerTaskBoardStatusResponse, ApiError> {
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let tasks = self
            .store
            .list_task_board_tasks(&project_path, AgentTaskFilter::default())
            .map_err(map_scheduler_error)?
            .into_iter()
            .map(|task| {
                task_board_task_to_dto_with_latest_run(
                    &self.store,
                    &project_id,
                    &project_path,
                    task,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let workflow =
            self.store.load_task_board_workflow(&project_path).map_err(map_scheduler_error)?;
        Ok(AgentSchedulerTaskBoardStatusResponse {
            project_id: project_id.into(),
            read_model: AgentSchedulerTaskBoardSnapshotReadModelDto { workflow },
            tasks,
        })
    }

    async fn task_graph(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
    ) -> Result<AgentSchedulerTaskGraphResponse, ApiError> {
        self.task_graph_snapshot(project_id, &project_path)
    }

    async fn replace_task_graph(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
        request: AgentSchedulerReplaceTaskGraphRequest,
    ) -> Result<AgentSchedulerTaskGraphResponse, ApiError> {
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let dependencies = request
            .edges
            .into_iter()
            .map(|edge| {
                Ok(AgentTaskDependency::new(
                    TaskId::new(edge.predecessor_task_id).map_err(map_bad_request)?,
                    TaskId::new(edge.successor_task_id).map_err(map_bad_request)?,
                ))
            })
            .collect::<Result<Vec<_>, ApiError>>()?;
        self.store
            .replace_task_dependencies(&project_path, dependencies)
            .map_err(map_bad_request)?;
        self.task_graph_snapshot(project_id, &project_path)
    }

    async fn update_task_metadata(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
        task_id: String,
        request: AgentSchedulerTaskMetadataUpdateRequest,
    ) -> Result<AgentSchedulerTaskMetadataUpdateResponse, ApiError> {
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let task_id = TaskId::new(task_id).map_err(map_bad_request)?;
        let outcome = AgentTaskBoardService::new(&self.store)
            .update_metadata(
                &project_path,
                &task_id,
                request.patch,
                request.reason.as_deref(),
                current_timestamp_ms(),
            )
            .map_err(map_task_board_error)?
            .ok_or_else(|| ApiError::not_found("task not found"))?;
        let task = task_board_task_to_dto_with_latest_run(
            &self.store,
            &project_id,
            &project_path,
            outcome.task,
        )?;
        let event = task_board_event_to_dto(&project_id, outcome.event)?;
        Ok(AgentSchedulerTaskMetadataUpdateResponse { task, event: Some(event) })
    }

    async fn import_legacy_tasks(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
        request: AgentSchedulerLegacyImportRequest,
    ) -> Result<AgentSchedulerLegacyImportResponse, ApiError> {
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        let outcome = AgentTaskBoardService::new(&self.store)
            .import_legacy_tasks(
                &project_path,
                request.workflow,
                request.tasks,
                request.dry_run,
                current_timestamp_ms(),
            )
            .map_err(map_task_board_error)?;
        let tasks = outcome
            .tasks
            .into_iter()
            .map(|task| {
                task_board_task_to_dto_with_latest_run(
                    &self.store,
                    &project_id,
                    &project_path,
                    task,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let events = outcome
            .events
            .into_iter()
            .map(|event| task_board_event_to_dto(&project_id, event))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AgentSchedulerLegacyImportResponse {
            imported: outcome.imported,
            skipped: outcome.skipped,
            tasks,
            events,
        })
    }

    async fn task_logs(
        &self,
        project_path: AgentSchedulerContext,
        task_id: String,
    ) -> Result<AgentSchedulerTaskLogsResponse, ApiError> {
        let task_id = TaskId::new(task_id).map_err(map_bad_request)?;
        let logs = self
            .store
            .list_task_logs(&project_path, &task_id)
            .map_err(map_scheduler_error)?
            .into_iter()
            .map(task_log_to_dto)
            .collect();
        Ok(AgentSchedulerTaskLogsResponse { task_id: task_id.as_str().into(), logs })
    }

    async fn task_artifacts(
        &self,
        project_path: AgentSchedulerContext,
        task_id: String,
    ) -> Result<AgentSchedulerTaskArtifactsResponse, ApiError> {
        let task_id = TaskId::new(task_id).map_err(map_bad_request)?;
        let artifacts = self
            .store
            .list_task_artifacts(&project_path, &task_id)
            .map_err(map_scheduler_error)?
            .into_iter()
            .map(task_artifact_to_dto)
            .collect();
        Ok(AgentSchedulerTaskArtifactsResponse { task_id: task_id.as_str().into(), artifacts })
    }

    async fn task_board_events(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
        query: AgentSchedulerProjectEventsQuery,
    ) -> Result<AgentSchedulerTaskBoardEventsResponse, ApiError> {
        let limit = query.limit.unwrap_or(DEFAULT_EVENT_LIMIT).min(MAX_EVENT_LIMIT);
        let events = self
            .store
            .list_events_after(&project_path, query.after_cursor, limit)
            .map_err(map_scheduler_error)?;
        let cursor = events.last().map_or(query.after_cursor.unwrap_or(0), |event| event.cursor);
        let events = events
            .into_iter()
            .map(|event| task_board_event_to_dto(&project_id, event))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AgentSchedulerTaskBoardEventsResponse { cursor, events })
    }

    async fn wake_project(
        &self,
        project_id: String,
        project_path: AgentSchedulerContext,
        request: AgentSchedulerProjectWakeRequest,
    ) -> Result<AgentSchedulerProjectWakeResponse, ApiError> {
        tracing::info!(
            project_path = %project_path.storage_project_path().display(),
            dispatch_mode = ?request.dispatch_mode,
            now_ms = request.now_ms.0,
            "agent scheduler project wake requested"
        );
        self.store.initialize_project(&project_path).map_err(map_scheduler_error)?;
        self.apply_policy_tick(
            Some(project_id.as_str()),
            &project_path,
            request.now_ms,
            request.auto_promote_pool_tasks,
        )
        .await?;
        let dispatch_lock = self.project_dispatch_lock(&project_path);
        let _dispatch_guard = dispatch_lock.lock().await;
        let max_concurrent = normalize_project_wake_max_concurrent(request.max_concurrent);
        let active_count =
            self.active_dispatch_finite_task_count(&project_path, request.dispatch_mode)?;
        if active_count >= max_concurrent {
            tracing::info!(
                project_path = %project_path.storage_project_path().display(),
                dispatch_mode = ?request.dispatch_mode,
                active_count,
                max_concurrent,
                "agent scheduler project wake skipped because concurrency limit is reached"
            );
            return Ok(AgentSchedulerProjectWakeResponse {
                dispatched: false,
                task_id: None,
                run_id: None,
                worker_id: None,
                outbox_message_id: None,
            });
        }
        if request.dispatch_mode != AgentSchedulerDispatchModeDto::Local {
            let outcome = self
                .dispatch_project_once(&project_path, request.now_ms, request.dispatch_mode)
                .await?;
            tracing::info!(
                project_path = %project_path.storage_project_path().display(),
                dispatch_mode = ?request.dispatch_mode,
                dispatched = outcome.is_some(),
                task_id = ?outcome.as_ref().map(|outcome| outcome.task_id.as_str()),
                run_id = ?outcome.as_ref().map(|outcome| outcome.run_id.as_str()),
                outbox_message_id = ?outcome.as_ref().and_then(|outcome| outcome.outbox_message_id.as_deref()),
                "agent scheduler project wake dispatch finished"
            );
            return Ok(project_wake_response_from_dispatch_outcome(outcome));
        }
        let Some(task) = ready_queued_finite_task(&self.store, &project_path, None)
            .map_err(map_scheduler_error)?
        else {
            tracing::info!(
                project_path = %project_path.storage_project_path().display(),
                dispatch_mode = ?request.dispatch_mode,
                "agent scheduler local project wake found no queued finite task"
            );
            return Ok(AgentSchedulerProjectWakeResponse {
                dispatched: false,
                task_id: None,
                run_id: None,
                worker_id: None,
                outbox_message_id: None,
            });
        };
        let outcome = self
            .run_local_task(owned_scheduler_context(&project_path)?, task.id, request.now_ms)
            .await?;
        tracing::info!(
            dispatched = outcome.is_some(),
            task_id = ?outcome.as_ref().map(|outcome| outcome.task_id.as_str()),
            run_id = ?outcome.as_ref().map(|outcome| outcome.run.id.as_str()),
            "agent scheduler local project wake started task"
        );
        Ok(project_wake_response_from_local_outcome(outcome))
    }

    async fn drain_project_outbox(
        &self,
        project_path: AgentSchedulerContext,
        request: AgentSchedulerProjectOutboxDrainRequest,
    ) -> Result<AgentSchedulerProjectOutboxDrainResponse, ApiError> {
        let limit = request.limit.unwrap_or(DEFAULT_OUTBOX_DRAIN_LIMIT).min(MAX_OUTBOX_DRAIN_LIMIT);
        tracing::info!(
            project_path = %project_path.storage_project_path().display(),
            now_ms = request.now_ms.0,
            limit,
            "agent scheduler project outbox drain requested"
        );
        let dispatch_adapter = LocalOnlyDispatchAdapter;
        let dispatcher = OutboxDispatcher::new(&self.store, &dispatch_adapter);
        let (summary, messages) = dispatcher
            .drain_messages_once(&project_path, request.now_ms, limit)
            .await
            .map_err(map_scheduler_error)?;
        tracing::info!(
            project_path = %project_path.storage_project_path().display(),
            claimed = summary.claimed,
            delivered = summary.delivered,
            failed = summary.failed,
            "agent scheduler project outbox drain finished"
        );
        let messages = messages.into_iter().map(outbox_to_dto).collect();
        Ok(AgentSchedulerProjectOutboxDrainResponse { claimed: summary.claimed, messages })
    }
}

impl StoreBackedAgentSchedulerGateway {
    async fn apply_policy_tick(
        &self,
        project_id: Option<&str>,
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
        if let Some(project_id) = project_id {
            self.process_policy_merge_actions(project_id, project_path, now_ms, &outcome).await?;
        } else if !outcome.merge_task_ids.is_empty() {
            tracing::warn!(
                project_path = %project_path.storage_project_path().display(),
                merge_actions = outcome.merge_task_ids.len(),
                "agent scheduler policy merge actions skipped without project id"
            );
        }
        Ok(outcome)
    }

    async fn process_policy_merge_actions(
        &self,
        project_id: &str,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
        outcome: &AgentSchedulerPolicyOutcome,
    ) -> Result<(), ApiError> {
        for task_id in &outcome.merge_task_ids {
            let Some(task) =
                self.store.load_task(project_path, task_id).map_err(map_scheduler_error)?
            else {
                tracing::warn!(
                    project_path = %project_path.storage_project_path().display(),
                    task_id = task_id.as_str(),
                    "agent scheduler policy merge skipped missing task"
                );
                continue;
            };
            if let Err(error) =
                self.merge_project_task(project_id, project_path, &task, now_ms).await
            {
                tracing::warn!(
                    error = %error,
                    project_path = %project_path.storage_project_path().display(),
                    task_id = task.id.as_str(),
                    "agent scheduler policy merge action failed"
                );
            }
        }
        Ok(())
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
        project_id: &str,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        now_ms: TimestampMs,
    ) -> Result<(), ApiError> {
        if task.status == AgentTaskStatus::Completed {
            self.append_merge_diagnostic_log(
                project_path,
                task,
                now_ms,
                "agent scheduler merge skipped: task already completed",
            );
            return Ok(());
        }
        if task.status != AgentTaskStatus::Submitted {
            self.append_merge_diagnostic_log(
                project_path,
                task,
                now_ms,
                format!("agent scheduler merge rejected: status={}", task.status.as_str()),
            );
            return Err(ApiError::bad_request(format!(
                "merge requires submitted task, got {}",
                task.status.as_str()
            )));
        }

        self.append_merge_diagnostic_log(
            project_path,
            task,
            now_ms,
            "agent scheduler merge requested",
        );
        let workspace = match task_merge_workspace_context(&self.store, project_path, task) {
            Ok(workspace) => workspace,
            Err(error) => {
                self.append_merge_diagnostic_log(
                    project_path,
                    task,
                    now_ms,
                    format!("agent scheduler merge rejected: workspace context error={error}"),
                );
                return Err(error);
            }
        };
        let selected_worktree_path = match string_field(&task.metadata, "selected_worktree_path")
            .or_else(|| workspace.as_ref().map(|workspace| workspace.path.clone()))
        {
            Some(path) => path,
            None => {
                self.append_merge_diagnostic_log(
                    project_path,
                    task,
                    now_ms,
                    "agent scheduler merge rejected: selected_worktree_path missing",
                );
                return Err(ApiError::bad_request("selected_worktree_path is required"));
            }
        };
        let selected_worktree_branch =
            match string_field(&task.metadata, "selected_worktree_branch")
                .or_else(|| workspace.as_ref().map(|workspace| workspace.branch.clone()))
            {
                Some(branch) => branch,
                None => {
                    self.append_merge_diagnostic_log(
                        project_path,
                        task,
                        now_ms,
                        "agent scheduler merge rejected: selected_worktree_branch missing",
                    );
                    return Err(ApiError::bad_request("selected_worktree_branch is required"));
                }
            };
        let source_branch = match string_field(&task.metadata, "merge_source_branch").or_else(
            || {
                valid_merge_branch(&selected_worktree_branch)
                    .then(|| selected_worktree_branch.clone())
            },
        ) {
            Some(branch) => branch,
            None => {
                self.append_merge_diagnostic_log(
                    project_path,
                    task,
                    now_ms,
                    format!(
                        "agent scheduler merge rejected: merge_source_branch unresolved selected_branch={}",
                        selected_worktree_branch
                    ),
                );
                return Err(ApiError::bad_request(
                    "merge_source_branch could not be resolved from selected_worktree_branch",
                ));
            }
        };
        let target_branch = match string_field(&task.metadata, "merge_target_branch")
            .map(Ok)
            .unwrap_or_else(|| project_current_branch(project_path))
        {
            Ok(branch) => branch,
            Err(error) => {
                self.append_merge_diagnostic_log(
                    project_path,
                    task,
                    now_ms,
                    format!("agent scheduler merge rejected: target branch error={error}"),
                );
                return Err(error);
            }
        };
        if source_branch == target_branch || source_branch == "HEAD" || target_branch == "HEAD" {
            self.append_merge_diagnostic_log(
                project_path,
                task,
                now_ms,
                format!(
                    "agent scheduler merge rejected: invalid branches source={} target={}",
                    source_branch, target_branch
                ),
            );
            return Err(ApiError::bad_request(format!(
                "invalid merge branches: source={} target={}",
                source_branch, target_branch
            )));
        }

        if selected_worktree_branch != source_branch {
            self.append_merge_diagnostic_log(
                project_path,
                task,
                now_ms,
                format!(
                    "agent scheduler merge rejected: selected_branch={} source_branch={}",
                    selected_worktree_branch, source_branch
                ),
            );
            return Err(ApiError::bad_request(format!(
                "selected_worktree_branch must match merge_source_branch: {} != {}",
                selected_worktree_branch, source_branch
            )));
        }
        if !Path::new(&selected_worktree_path).is_dir() {
            self.append_merge_diagnostic_log(
                project_path,
                task,
                now_ms,
                format!(
                    "agent scheduler merge rejected: selected_worktree_path missing path={}",
                    selected_worktree_path
                ),
            );
            return Err(ApiError::bad_request(
                "selected_worktree_path must reference an existing directory",
            ));
        }

        self.append_merge_diagnostic_log(
            project_path,
            task,
            now_ms,
            format!(
                "agent scheduler merge resolved: source={} target={} selected_branch={} selected_worktree={}",
                source_branch, target_branch, selected_worktree_branch, selected_worktree_path
            ),
        );
        let project = project::Info {
            id: project_id.to_string(),
            worktree: project_path.storage_project_path().to_string_lossy().into_owned(),
            vcs: Some(project::Vcs::Git),
            name: None,
            icon: None,
            commands: None,
            time: project::TimeInfo { created: 0, updated: 0, initialized: None },
            sandboxes: Vec::new(),
        };
        let task_api_id = vw_api_types::id::TaskId(task.id.as_str().to_string());
        self.append_merge_diagnostic_log(
            project_path,
            task,
            now_ms,
            format!("agent scheduler merge locking target: target={target_branch}"),
        );
        let lock = match super::task_pool::task_worktree_pool_service()
            .try_lock_merge_target(&project, &task_api_id, &target_branch)
            .await
            .map_err(map_scheduler_error)
        {
            Ok(lock) => lock,
            Err(error) => {
                self.append_merge_diagnostic_log(
                    project_path,
                    task,
                    now_ms,
                    format!("agent scheduler merge rejected: lock target error={error}"),
                );
                return Err(error);
            }
        };
        if !lock.locked {
            let holder =
                lock.holder_task_id.map(|holder| holder.0).unwrap_or_else(|| "unknown".to_string());
            let message = format!(
                "agent scheduler merge blocked: target={} holder={}",
                target_branch, holder
            );
            self.append_merge_diagnostic_log(project_path, task, now_ms, message.clone());
            self.append_merge_diagnostic_event(
                project_path,
                task,
                now_ms,
                AgentEventKind::TaskBlocked,
                Some(AgentTaskStatus::Submitted),
                Some(AgentTaskStatus::Submitted),
                message,
                json!({
                    "command": "merge",
                    "error": "merge_target_locked",
                    "target_branch": target_branch,
                    "holder_task_id": holder,
                }),
            );
            return Err(ApiError::bad_request(format!(
                "merge target locked: target={} holder={}",
                target_branch, holder
            )));
        }

        self.append_merge_diagnostic_log(
            project_path,
            task,
            now_ms,
            format!("agent scheduler merge locked target: target={target_branch}"),
        );
        self.append_merge_diagnostic_log(
            project_path,
            task,
            now_ms,
            format!(
                "agent scheduler merge git start: source={} target={} project_path={}",
                source_branch,
                target_branch,
                project_path.storage_project_path().display()
            ),
        );
        let merge_result = {
            let project_path = project_path.storage_project_path().to_path_buf();
            let source_branch = source_branch.clone();
            let target_branch = target_branch.clone();
            tokio::task::spawn_blocking(move || {
                super::git::merge_branch_blocking(
                    &project_path.to_string_lossy(),
                    &source_branch,
                    &target_branch,
                )
            })
            .await
        };

        let unlock_result = super::task_pool::task_worktree_pool_service()
            .unlock_merge_target(&project, &task_api_id, &target_branch)
            .await;
        if let Err(error) = unlock_result {
            tracing::warn!(
                error = %error,
                task_id = task.id.as_str(),
                target_branch = %target_branch,
                "agent scheduler merge lock release failed"
            );
            self.append_merge_diagnostic_log(
                project_path,
                task,
                now_ms,
                format!("agent scheduler merge lock release failed: target={target_branch} error={error}"),
            );
        } else {
            self.append_merge_diagnostic_log(
                project_path,
                task,
                now_ms,
                format!("agent scheduler merge lock released: target={target_branch}"),
            );
        }

        let merge_result = match merge_result {
            Ok(result) => result,
            Err(error) => {
                self.append_merge_diagnostic_log(
                    project_path,
                    task,
                    now_ms,
                    format!("agent scheduler merge join failed: {error}"),
                );
                return Err(ApiError::internal(format!(
                    "agent scheduler merge join failed: {error}"
                )));
            }
        };

        let response = match merge_result {
            Ok(response) if response.ok => response,
            Ok(response) => {
                let message = format!(
                    "merge failed source={} target={} workspace={} message={}",
                    response.source_branch,
                    response.target_branch,
                    response.workspace,
                    response.message
                );
                self.record_merge_failure(project_path, task, now_ms, message.clone())?;
                return Err(ApiError::bad_request(message));
            }
            Err(error) => {
                let message = format!(
                    "merge failed source={} target={} error={}",
                    source_branch, target_branch, error
                );
                self.record_merge_failure(project_path, task, now_ms, message.clone())?;
                return Err(ApiError::bad_request(message));
            }
        };

        self.store
            .transition_task_status(project_path, &task.id, AgentTaskStatus::Completed, now_ms)
            .map_err(map_scheduler_command_error)?
            .ok_or_else(|| ApiError::not_found("task not found"))?;
        self.store
            .append_task_log(
                project_path,
                AppendAgentTaskLogRequest {
                    task_id: task.id.clone(),
                    run_id: None,
                    timestamp_ms: now_ms,
                    from_status: Some(AgentTaskStatus::Submitted),
                    to_status: Some(AgentTaskStatus::Completed),
                    from_status_key: None,
                    to_status_key: None,
                    message: if response.already_merged {
                        format!(
                            "agent scheduler merge skipped: {} already in {} workspace={} already_merged=true",
                            response.source_branch, response.target_branch, response.workspace
                        )
                    } else {
                        format!(
                            "agent scheduler merge completed: {} -> {} workspace={}",
                            response.source_branch, response.target_branch, response.workspace
                        )
                    },
                },
            )
            .map_err(map_scheduler_error)?;
        self.store
            .record_task_artifact(
                project_path,
                RecordTaskArtifactRequest {
                    task_id: task.id.clone(),
                    run_id: None,
                    kind: "merge_result".to_string(),
                    name: "merge-result.json".to_string(),
                    path: Some(format!(
                        "shared/tasks/{}/artifacts/merge-result.json",
                        task.id.as_str()
                    )),
                    payload: json!({
                        "success": true,
                        "source_branch": response.source_branch,
                        "target_branch": response.target_branch,
                        "workspace": response.workspace,
                        "already_merged": response.already_merged,
                        "message": response.message,
                    }),
                    created_at_ms: now_ms,
                },
            )
            .map_err(map_scheduler_error)?;
        Ok(())
    }

    fn record_merge_failure(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        now_ms: TimestampMs,
        message: String,
    ) -> Result<(), ApiError> {
        self.store
            .transition_task_status(project_path, &task.id, AgentTaskStatus::Failed, now_ms)
            .map_err(map_scheduler_command_error)?
            .ok_or_else(|| ApiError::not_found("task not found"))?;
        self.store
            .append_task_log(
                project_path,
                AppendAgentTaskLogRequest {
                    task_id: task.id.clone(),
                    run_id: None,
                    timestamp_ms: now_ms,
                    from_status: Some(AgentTaskStatus::Submitted),
                    to_status: Some(AgentTaskStatus::Failed),
                    from_status_key: None,
                    to_status_key: None,
                    message: message.clone(),
                },
            )
            .map_err(map_scheduler_error)?;
        self.append_merge_diagnostic_event(
            project_path,
            task,
            now_ms,
            AgentEventKind::RunFailed,
            Some(AgentTaskStatus::Submitted),
            Some(AgentTaskStatus::Failed),
            "agent scheduler merge failed",
            json!({
                "command": "merge",
                "error": message,
            }),
        );
        Ok(())
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

    fn append_merge_diagnostic_event(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        task: &StoredAgentTask,
        now_ms: TimestampMs,
        kind: AgentEventKind,
        from_status: Option<AgentTaskStatus>,
        to_status: Option<AgentTaskStatus>,
        message: impl Into<String>,
        payload: Value,
    ) {
        let message = message.into();
        if let Err(error) = self.store.append_event(
            project_path,
            AppendAgentEventRequest {
                task_id: Some(task.id.clone()),
                run_id: None,
                worker_id: task.assigned_to.clone(),
                team_id: task.team_id.clone(),
                kind,
                from_status,
                to_status,
                from_status_key: None,
                to_status_key: None,
                message: message.clone(),
                payload,
                created_at_ms: now_ms,
            },
        ) {
            tracing::warn!(
                error = %error,
                task_id = task.id.as_str(),
                message = %message,
                "failed to append agent scheduler merge diagnostic event"
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

fn start_command_is_idempotent_for_status(status: AgentTaskStatus) -> bool {
    matches!(
        status,
        AgentTaskStatus::Assigned
            | AgentTaskStatus::Dispatched
            | AgentTaskStatus::Acknowledged
            | AgentTaskStatus::Running
            | AgentTaskStatus::Submitted
    )
}

fn normalize_project_wake_max_concurrent(max_concurrent: Option<u32>) -> usize {
    max_concurrent
        .unwrap_or(DEFAULT_PROJECT_WAKE_MAX_CONCURRENT as u32)
        .clamp(1, MAX_PROJECT_WAKE_MAX_CONCURRENT as u32) as usize
}

fn execution_slot_status(status: AgentTaskStatus) -> bool {
    matches!(
        status,
        AgentTaskStatus::Assigned
            | AgentTaskStatus::Dispatched
            | AgentTaskStatus::Acknowledged
            | AgentTaskStatus::Running
    )
}

fn task_board_write_command(command: AgentSchedulerCommandKindDto) -> bool {
    matches!(
        command,
        AgentSchedulerCommandKindDto::Archive
            | AgentSchedulerCommandKindDto::Move
            | AgentSchedulerCommandKindDto::UpdateMetadata
            | AgentSchedulerCommandKindDto::SoftDelete
            | AgentSchedulerCommandKindDto::Restore
            | AgentSchedulerCommandKindDto::ImportLegacyTask
    )
}

fn new_task_from_request(
    request: AgentSchedulerCreateTaskRequest,
) -> Result<NewAgentTask, ApiError> {
    let status = task_status_from_dto(request.status.unwrap_or(AgentTaskStatusDto::Queued));
    Ok(NewAgentTask {
        id: TaskId::new(request.task_id).map_err(map_bad_request)?,
        title: non_empty(request.title, "task title")?,
        kind: task_kind_from_dto(request.kind),
        status,
        spec_uri: non_empty(request.spec_uri, "task spec uri")?,
        meta_uri: non_empty(request.meta_uri, "task meta uri")?,
        assigned_to: optional_worker_name(request.assigned_to)?,
        team_id: optional_team_id(request.team_id)?,
        room_id: request.room_id,
        project_room_id: request.project_room_id,
        delegated_to_team: optional_team_id(request.delegated_to_team)?,
        schedule: request.schedule,
        timezone: request.timezone,
        next_scheduled_at: request.next_scheduled_at,
        last_executed_at: request.last_executed_at,
        dag: Vec::new(),
        metadata: request.metadata,
        created_at_ms: request.created_at_ms,
    })
}

fn append_task_created_event(
    store: &AgentSchedulerStore,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    task: &StoredAgentTask,
) -> Result<(), ApiError> {
    store
        .append_event(
            context,
            AppendAgentEventRequest {
                task_id: Some(task.id.clone()),
                run_id: None,
                worker_id: task.assigned_to.clone(),
                team_id: task.team_id.clone(),
                kind: AgentEventKind::TaskCreated,
                from_status: None,
                to_status: Some(task.status),
                from_status_key: None,
                to_status_key: None,
                message: format!("created agent scheduler task {}", task.id.as_str()),
                payload: json!({
                    "task_id": task.id.as_str(),
                    "title": task.title,
                    "kind": task_kind_to_dto(task.kind),
                    "status": task_status_to_dto(task.status),
                }),
                created_at_ms: task.created_at_ms,
            },
        )
        .map_err(map_scheduler_error)?;
    Ok(())
}

fn wake_response_from_outcome(outcome: Option<DispatchOutcome>) -> AgentSchedulerWakeResponse {
    match outcome {
        Some(outcome) => AgentSchedulerWakeResponse {
            dispatched: true,
            task_id: Some(outcome.task_id.as_str().to_string()),
            run_id: Some(outcome.run_id.as_str().to_string()),
            worker_id: None,
            outbox_message_id: outcome.outbox_message_id,
        },
        None => AgentSchedulerWakeResponse {
            dispatched: false,
            task_id: None,
            run_id: None,
            worker_id: None,
            outbox_message_id: None,
        },
    }
}

fn recovery_response_from_outcome(outcome: RecoveryOutcome) -> AgentSchedulerRecoveryResponse {
    AgentSchedulerRecoveryResponse {
        graced_run_ids: outcome
            .graced_run_ids
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect(),
        recovered_run_ids: outcome
            .recovered_run_ids
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect(),
        released_resource_lease_ids: outcome
            .released_resource_lease_ids
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect(),
        tainted_resource_lease_ids: outcome
            .tainted_resource_lease_ids
            .into_iter()
            .map(|id| id.as_str().to_string())
            .collect(),
        drifted_worker_ids: Vec::new(),
        slept_worker_ids: Vec::new(),
        outbox_message_ids: outcome.outbox_message_ids,
    }
}

async fn resolve_project_context(
    tenant: TenantScope,
    project_id: &str,
) -> Result<AgentSchedulerContext, ApiError> {
    let project_scope = ProjectScope::new(tenant, ProjectId::from(project_id))
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let project = project::get(&project_scope)
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("project not found"))?;
    AgentSchedulerContext::new(AgentSchedulerScope::new(project_scope), project.worktree)
        .map_err(map_scheduler_error)
}

fn owned_scheduler_context(
    context: &(impl AgentSchedulerStorageContext + ?Sized),
) -> Result<AgentSchedulerContext, ApiError> {
    AgentSchedulerContext::new(
        context.storage_scope().map_err(map_scheduler_error)?,
        context.storage_project_path(),
    )
    .map_err(map_scheduler_error)
}

fn task_id_from_api(task_id: &vw_api_types::id::TaskId) -> Result<TaskId, ApiError> {
    TaskId::new(task_id.as_ref().to_string()).map_err(map_bad_request)
}

fn current_timestamp_ms() -> TimestampMs {
    TimestampMs(chrono::Utc::now().timestamp_millis().max(0))
}

fn add_ms_or_saturate(timestamp: TimestampMs, delta_ms: i64) -> TimestampMs {
    TimestampMs(timestamp.0.saturating_add(delta_ms).max(timestamp.0))
}

fn workspace_payload(workspace: &AgentSchedulerWorkspaceContextDto) -> Value {
    json!({
        "slot_id": workspace.slot_id,
        "path": workspace.path,
        "branch": workspace.branch,
    })
}

fn dispatch_mode_from_dto(dispatch_mode: AgentSchedulerDispatchModeDto) -> DispatchMode {
    match dispatch_mode {
        AgentSchedulerDispatchModeDto::Local => DispatchMode::Local,
        AgentSchedulerDispatchModeDto::Auto => DispatchMode::Auto,
    }
}

fn is_default_dispatch_mode(dispatch_mode: &AgentSchedulerDispatchModeDto) -> bool {
    *dispatch_mode == AgentSchedulerDispatchModeDto::default()
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn append_command_event(
    store: &AgentSchedulerStore,
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
    task: &StoredAgentTask,
    run: Option<&StoredAgentRun>,
    command: AgentSchedulerCommandKindDto,
    from_status: AgentTaskStatus,
    reason: Option<&str>,
    now_ms: TimestampMs,
) -> Result<StoredAgentEvent, ApiError> {
    let message =
        reason.map(str::trim).filter(|value| !value.is_empty()).map(str::to_string).unwrap_or_else(
            || format!("accepted agent scheduler command {}", command_kind_key(command)),
        );
    store
        .append_event(
            project_path,
            AppendAgentEventRequest {
                task_id: Some(task.id.clone()),
                run_id: run.map(|run| run.id.clone()),
                worker_id: task.assigned_to.clone(),
                team_id: task.team_id.clone(),
                kind: command_event_kind(command),
                from_status: Some(from_status),
                to_status: Some(task.status),
                from_status_key: None,
                to_status_key: None,
                message,
                payload: json!({
                    "command": command_kind_key(command),
                    "task_id": task.id.as_str(),
                    "run_id": run.map(|run| run.id.as_str()),
                }),
                created_at_ms: now_ms,
            },
        )
        .map_err(map_scheduler_error)
}

fn command_event_kind(command: AgentSchedulerCommandKindDto) -> AgentEventKind {
    match command {
        AgentSchedulerCommandKindDto::Pause => AgentEventKind::TaskPaused,
        AgentSchedulerCommandKindDto::Resume => AgentEventKind::TaskResumed,
        AgentSchedulerCommandKindDto::Cancel => AgentEventKind::TaskCancelled,
        AgentSchedulerCommandKindDto::Archive => AgentEventKind::TaskArchived,
        _ => AgentEventKind::HumanIntervened,
    }
}

fn task_board_task_to_dto_with_latest_run(
    store: &AgentSchedulerStore,
    project_id: &str,
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
    task: StoredAgentTask,
) -> Result<AgentSchedulerTaskBoardTaskDto, ApiError> {
    let run =
        store.load_latest_run_for_task(project_path, &task.id).map_err(map_scheduler_error)?;
    task_board_task_to_dto(store, project_id, project_path, task, run.as_ref())
}

fn task_board_task_to_dto(
    store: &AgentSchedulerStore,
    project_id: &str,
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
    task: StoredAgentTask,
    run: Option<&StoredAgentRun>,
) -> Result<AgentSchedulerTaskBoardTaskDto, ApiError> {
    let metadata = &task.metadata;
    let workspace = run
        .map(|run| {
            store
                .load_run_workspace_context(project_path, &run.id)
                .map_err(map_scheduler_error)
                .map(|workspace| workspace.map(workspace_to_dto))
        })
        .transpose()?
        .flatten()
        .or_else(|| workspace_context_from_value(metadata));
    let selected_worktree_slot_id = string_field(metadata, "selected_worktree_slot_id")
        .or_else(|| workspace.as_ref().map(|workspace| workspace.slot_id.clone()));
    let selected_worktree_path = string_field(metadata, "selected_worktree_path")
        .or_else(|| workspace.as_ref().map(|workspace| workspace.path.clone()));
    let selected_worktree_branch = string_field(metadata, "selected_worktree_branch")
        .or_else(|| workspace.as_ref().map(|workspace| workspace.branch.clone()));
    let merge_source_branch = string_field(metadata, "merge_source_branch")
        .or_else(|| selected_worktree_branch.clone().filter(|branch| valid_merge_branch(branch)));
    let last_error = run
        .and_then(|run| run.error_message.as_deref().and_then(non_empty_str).map(str::to_string))
        .or_else(|| string_field(metadata, "last_error"));
    let status_key = task_board_status_key(task.status, metadata).to_string();
    let read_model = task_board_task_read_model(&task, metadata, &status_key);

    Ok(AgentSchedulerTaskBoardTaskDto {
        project_id: project_id.into(),
        task_id: task.id.as_str().into(),
        status_key,
        status: Some(task_status_to_dto(task.status)),
        priority: u32_field(metadata, "priority").unwrap_or_default(),
        order_no: i64_field(metadata, "order_no").unwrap_or_default(),
        model: string_field(metadata, "model").unwrap_or_default(),
        agent: string_field(metadata, "agent"),
        acp_agent: string_field(metadata, "acp_agent"),
        reasoning_effort: reasoning_effort_field(metadata)?,
        prompt: string_field(metadata, "prompt").unwrap_or_else(|| task.title.clone()),
        retry_count: run.map_or(0, |run| run.attempt.saturating_sub(1)),
        run_id: run.map(|run| run.id.as_str().to_string()),
        run_stage: run.and_then(|run| run_stage_from_run_status(run.status).map(str::to_string)),
        workspace,
        last_error,
        pause_reason: string_field(metadata, "pause_reason"),
        merge_source_branch,
        merge_target_branch: string_field(metadata, "merge_target_branch"),
        selected_worktree_path,
        selected_worktree_slot_id,
        selected_worktree_branch,
        read_model,
        updated_at_ms: task.updated_at_ms,
    })
}

fn task_board_task_read_model(
    task: &StoredAgentTask,
    metadata: &Value,
    status_key: &str,
) -> AgentSchedulerTaskBoardTaskReadModelDto {
    AgentSchedulerTaskBoardTaskReadModelDto {
        title: Some(task.title.clone()),
        workflow_id: string_field(metadata, "task_board.workflow_id"),
        status_label: string_field(metadata, "task_board.status_label"),
        status_role: task_board_status_role_from_metadata(metadata),
        stage_id: string_field(metadata, "task_board.stage_id")
            .or_else(|| Some(status_key.to_string())),
        stage_label: string_field(metadata, "task_board.stage_label"),
    }
}

fn task_board_status_role_from_metadata(metadata: &Value) -> Option<TaskPoolWorkflowStatusRoleDto> {
    let role = string_field(metadata, "task_board.status_role")?;
    match role.as_str() {
        "backlog" => Some(TaskPoolWorkflowStatusRoleDto::Backlog),
        "queued" => Some(TaskPoolWorkflowStatusRoleDto::Queued),
        "planning" => Some(TaskPoolWorkflowStatusRoleDto::Planning),
        "executing" => Some(TaskPoolWorkflowStatusRoleDto::Executing),
        "failed" => Some(TaskPoolWorkflowStatusRoleDto::Failed),
        "paused" => Some(TaskPoolWorkflowStatusRoleDto::Paused),
        "review_ready" => Some(TaskPoolWorkflowStatusRoleDto::ReviewReady),
        "review" => Some(TaskPoolWorkflowStatusRoleDto::Review),
        "merging" => Some(TaskPoolWorkflowStatusRoleDto::Merging),
        "done" => Some(TaskPoolWorkflowStatusRoleDto::Done),
        "archived" => Some(TaskPoolWorkflowStatusRoleDto::Archived),
        "custom" => Some(TaskPoolWorkflowStatusRoleDto::Custom),
        _ => None,
    }
}

fn workflow_status_role(
    workflow: &TaskPoolWorkflowSnapshotDto,
    status_key: &str,
) -> Option<TaskPoolWorkflowStatusRoleDto> {
    workflow
        .statuses
        .iter()
        .find(|status| status.enabled && status.key == status_key)
        .map(|status| status.role)
}

fn task_metadata_object(value: Value) -> Result<Map<String, Value>, ApiError> {
    match value {
        Value::Object(map) => Ok(map),
        Value::Null => Ok(Map::new()),
        _ => Err(ApiError::bad_request("task metadata must be a JSON object")),
    }
}

fn set_lifecycle_projection_metadata(
    metadata: &mut Map<String, Value>,
    target: &LifecycleStatusProjection,
) {
    metadata.insert("task_board.status_key".to_string(), json!(target.status_key.clone()));
    metadata.insert(
        "task_board.status_role".to_string(),
        json!(lifecycle_status_role_key(target.status_role)),
    );
}

fn task_board_status_key(status: AgentTaskStatus, metadata: &Value) -> String {
    task_board_status_key_from_metadata(status, metadata)
        .unwrap_or_else(|| legacy_task_board_status_key(status).to_string())
}

fn task_board_status_key_from_metadata(
    status: AgentTaskStatus,
    metadata: &Value,
) -> Option<String> {
    if let Some(status_key) =
        metadata.get("task_board.status_key").and_then(Value::as_str).and_then(non_empty_str)
    {
        return Some(status_key.to_string());
    }
    let status_key_map = metadata.get("task_board.status_key_map").and_then(Value::as_object)?;
    status_key_map
        .get(status.as_str())
        .and_then(Value::as_str)
        .and_then(non_empty_str)
        .map(str::to_string)
}

fn legacy_task_board_status_key(status: AgentTaskStatus) -> &'static str {
    match status {
        AgentTaskStatus::Submitted => "code_complete",
        _ => status.as_str(),
    }
}

fn valid_merge_branch(branch: &str) -> bool {
    let branch = branch.trim();
    !branch.is_empty() && branch != "HEAD"
}

fn task_merge_workspace_context(
    store: &AgentSchedulerStore,
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
    task: &StoredAgentTask,
) -> Result<Option<AgentSchedulerWorkspaceContextDto>, ApiError> {
    let run =
        store.load_latest_run_for_task(project_path, &task.id).map_err(map_scheduler_error)?;
    run.map(|run| {
        store
            .load_run_workspace_context(project_path, &run.id)
            .map_err(map_scheduler_error)
            .map(|workspace| workspace.map(workspace_to_dto))
    })
    .transpose()
    .map(|workspace| workspace.flatten().or_else(|| workspace_context_from_value(&task.metadata)))
}

fn project_current_branch(
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
) -> Result<String, ApiError> {
    let output = crate::app::agent::shell::git_std_command()
        .current_dir(project_path.storage_project_path())
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|error| {
            ApiError::bad_request(format!(
                "merge target branch could not be resolved from project current branch: {error}"
            ))
        })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(ApiError::bad_request(format!(
            "merge target branch could not be resolved from project current branch: {}",
            if detail.is_empty() { "git rev-parse failed".to_string() } else { detail }
        )));
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !valid_merge_branch(&branch) {
        return Err(ApiError::bad_request(format!(
            "invalid project current branch for merge target: {}",
            if branch.is_empty() { "<empty>" } else { branch.as_str() }
        )));
    }
    Ok(branch)
}

fn task_log_to_dto(log: StoredAgentTaskLog) -> AgentSchedulerTaskLogDto {
    AgentSchedulerTaskLogDto {
        task_id: log.task_id.as_str().into(),
        seq_no: log.seq_no,
        timestamp_ms: log.timestamp_ms,
        from_status_key: log
            .from_status_key
            .or_else(|| log.from_status.map(|status| status.as_str().to_string())),
        status_from: log.from_status.map(task_status_to_dto),
        to_status_key: log
            .to_status_key
            .or_else(|| log.to_status.map(|status| status.as_str().to_string())),
        status_to: log.to_status.map(task_status_to_dto),
        message: log.message,
    }
}

fn task_artifact_to_dto(artifact: StoredTaskArtifact) -> AgentSchedulerTaskArtifactDto {
    AgentSchedulerTaskArtifactDto {
        task_id: artifact.task_id.as_str().into(),
        run_id: artifact.run_id.as_ref().map(RunId::as_str).map(str::to_string),
        kind: artifact.kind,
        name: artifact.name,
        path: artifact.path,
        payload: artifact.payload,
        created_at_ms: artifact.created_at_ms,
    }
}

fn task_board_event_to_dto(
    project_id: &str,
    event: StoredAgentEvent,
) -> Result<AgentSchedulerTaskBoardEventDto, ApiError> {
    let task_id = event
        .task_id
        .as_ref()
        .map(|task_id| task_id.as_str().to_string())
        .or_else(|| {
            task_id_from_payload(&event.payload).map(|task_id| task_id.as_str().to_string())
        })
        .ok_or_else(|| ApiError::internal("agent scheduler task board event requires task id"))?;
    let from_status_key = event
        .from_status_key
        .or_else(|| event.from_status.map(|status| status.as_str().to_string()));
    let to_status_key =
        event.to_status_key.or_else(|| event.to_status.map(|status| status.as_str().to_string()));
    let workspace = workspace_context_from_value(&event.payload);
    let error_message =
        event_error_message(event.kind, event.to_status, &event.message, &event.payload);
    Ok(AgentSchedulerTaskBoardEventDto {
        cursor: event.cursor,
        project_id: project_id.into(),
        task_id: task_id.into(),
        run_id: event.run_id.as_ref().map(RunId::as_str).map(str::to_string),
        run_stage: event.to_status.and_then(run_stage_from_task_status).map(str::to_string),
        kind: event_kind_to_dto(event.kind),
        from_status_key,
        from_status: event.from_status.map(task_status_to_dto),
        to_status_key,
        to_status: event.to_status.map(task_status_to_dto),
        message: event.message,
        error_message,
        workspace,
        payload: event.payload,
        created_at_ms: event.created_at_ms,
    })
}

fn project_wake_response_from_local_outcome(
    outcome: Option<LocalAgentRunnerOutcome>,
) -> AgentSchedulerProjectWakeResponse {
    match outcome {
        Some(outcome) => AgentSchedulerProjectWakeResponse {
            dispatched: true,
            task_id: Some(outcome.task_id.as_str().to_string()),
            run_id: Some(outcome.run.id.as_str().to_string()),
            worker_id: None,
            outbox_message_id: None,
        },
        None => AgentSchedulerProjectWakeResponse {
            dispatched: false,
            task_id: None,
            run_id: None,
            worker_id: None,
            outbox_message_id: None,
        },
    }
}

fn empty_project_wake_response() -> AgentSchedulerProjectWakeResponse {
    AgentSchedulerProjectWakeResponse {
        dispatched: false,
        task_id: None,
        run_id: None,
        worker_id: None,
        outbox_message_id: None,
    }
}

fn project_wake_response_from_dispatch_outcome(
    outcome: Option<DispatchOutcome>,
) -> AgentSchedulerProjectWakeResponse {
    match outcome {
        Some(outcome) => AgentSchedulerProjectWakeResponse {
            dispatched: true,
            task_id: Some(outcome.task_id.as_str().to_string()),
            run_id: Some(outcome.run_id.as_str().to_string()),
            worker_id: None,
            outbox_message_id: outcome.outbox_message_id,
        },
        None => AgentSchedulerProjectWakeResponse {
            dispatched: false,
            task_id: None,
            run_id: None,
            worker_id: None,
            outbox_message_id: None,
        },
    }
}

fn task_graph_task_to_dto(
    task: &TaskGraphTaskSnapshot,
    graph: &TaskGraph,
) -> Result<AgentSchedulerTaskGraphTaskDto, ApiError> {
    let readiness = graph.readiness_for(&task.id).map_err(map_scheduler_error)?;
    let (readiness, waiting_task_ids, blocking_task_ids) = match readiness {
        TaskGraphReadiness::Ready => {
            (AgentSchedulerTaskGraphReadinessDto::Ready, Vec::new(), Vec::new())
        }
        TaskGraphReadiness::WaitingDependencies { task_ids } => (
            AgentSchedulerTaskGraphReadinessDto::WaitingDependencies,
            task_ids.into_iter().map(|task_id| task_id.as_str().to_string()).collect(),
            Vec::new(),
        ),
        TaskGraphReadiness::BlockedByDependency { task_ids } => (
            AgentSchedulerTaskGraphReadinessDto::BlockedByDependency,
            Vec::new(),
            task_ids.into_iter().map(|task_id| task_id.as_str().to_string()).collect(),
        ),
        TaskGraphReadiness::NotQueued => {
            (AgentSchedulerTaskGraphReadinessDto::NotQueued, Vec::new(), Vec::new())
        }
    };
    Ok(AgentSchedulerTaskGraphTaskDto {
        task_id: task.id.as_str().to_string(),
        status: task_status_to_dto(task.status),
        readiness,
        waiting_task_ids,
        blocking_task_ids,
    })
}

fn task_to_dto(task: StoredAgentTask) -> AgentTaskDto {
    AgentTaskDto {
        id: task.id.as_str().to_string(),
        title: task.title,
        kind: task_kind_to_dto(task.kind),
        status: task_status_to_dto(task.status),
        assigned_to: task.assigned_to.as_ref().map(AssigneeId::as_str).map(str::to_string),
        team_id: task.team_id.as_ref().map(TeamId::as_str).map(str::to_string),
        room_id: task.room_id,
        project_room_id: task.project_room_id,
        delegated_to_team: task.delegated_to_team.as_ref().map(TeamId::as_str).map(str::to_string),
        schedule: task.schedule,
        timezone: task.timezone,
        next_scheduled_at: task.next_scheduled_at,
        last_executed_at: task.last_executed_at,
        dag: task.dag.into_iter().map(dag_node_to_dto).collect(),
        created_at_ms: task.created_at_ms,
        updated_at_ms: task.updated_at_ms,
        metadata: task.metadata,
    }
}

fn dag_node_to_dto(node: AgentTaskDagNode) -> TaskDependencyNodeDto {
    TaskDependencyNodeDto {
        task_id: node.task_id.as_str().to_string(),
        title: node.title,
        depends_on: node.depends_on.iter().map(|task_id| task_id.as_str().to_string()).collect(),
        assigned_to: node.assigned_to.as_ref().map(AssigneeId::as_str).map(str::to_string),
        status: task_status_to_dto(node.status),
    }
}

fn event_to_dto(event: StoredAgentEvent) -> AgentEventDto {
    AgentEventDto {
        id: event.cursor.to_string(),
        task_id: event.task_id.as_ref().map(TaskId::as_str).map(str::to_string),
        run_id: event.run_id.as_ref().map(RunId::as_str).map(str::to_string),
        kind: event_kind_to_dto(event.kind),
        message: event.message,
        payload: event.payload,
        created_at_ms: event.created_at_ms,
    }
}

fn run_to_dto(run: StoredAgentRun) -> AgentRunDto {
    AgentRunDto {
        id: run.id.as_str().to_string(),
        task_id: run.task_id.as_str().to_string(),
        status: run_status_to_dto(run.status),
        lease_expires_at_ms: run.lease_expires_at_ms,
        attempt: run.attempt,
        started_at_ms: run.started_at_ms,
        finished_at_ms: run.finished_at_ms,
        last_heartbeat_at_ms: run.last_heartbeat_at_ms,
        error_message: run.error_message,
    }
}

fn outbox_to_dto(message: StoredOutboxMessage) -> AgentOutboxMessageDto {
    AgentOutboxMessageDto {
        id: message.id,
        target_kind: message.channel,
        target_id: message.target,
        recipient_id: message.recipient_id,
        body: message.body,
        status: outbox_status_to_dto(message.status),
        idempotency_key: message.idempotency_key,
        attempts: message.attempts,
        next_attempt_at_ms: message.next_attempt_at_ms,
        delivered_at_ms: message.delivered_at_ms,
        last_error: message.last_error,
        payload: message.payload,
        created_at_ms: message.created_at_ms,
    }
}

fn run_status_to_dto(status: AgentRunStatus) -> AgentRunStatusDto {
    match status {
        AgentRunStatus::Pending => AgentRunStatusDto::Pending,
        AgentRunStatus::Leased => AgentRunStatusDto::Leased,
        AgentRunStatus::Running => AgentRunStatusDto::Running,
        AgentRunStatus::Succeeded => AgentRunStatusDto::Succeeded,
        AgentRunStatus::Failed => AgentRunStatusDto::Failed,
        AgentRunStatus::Cancelled => AgentRunStatusDto::Cancelled,
        AgentRunStatus::TimedOut => AgentRunStatusDto::TimedOut,
        AgentRunStatus::Stale => AgentRunStatusDto::Stale,
    }
}

fn task_kind_from_dto(kind: AgentTaskKindDto) -> AgentTaskKind {
    match kind {
        AgentTaskKindDto::Finite => AgentTaskKind::Finite,
        AgentTaskKindDto::Infinite => AgentTaskKind::Infinite,
        AgentTaskKindDto::DependencyGraph => AgentTaskKind::DependencyGraph,
    }
}

fn task_kind_to_dto(kind: AgentTaskKind) -> AgentTaskKindDto {
    match kind {
        AgentTaskKind::Finite => AgentTaskKindDto::Finite,
        AgentTaskKind::Infinite => AgentTaskKindDto::Infinite,
        AgentTaskKind::DependencyGraph => AgentTaskKindDto::DependencyGraph,
    }
}

fn task_status_from_dto(status: AgentTaskStatusDto) -> AgentTaskStatus {
    match status {
        AgentTaskStatusDto::Draft => AgentTaskStatus::Draft,
        AgentTaskStatusDto::Queued => AgentTaskStatus::Queued,
        AgentTaskStatusDto::Assigned => AgentTaskStatus::Assigned,
        AgentTaskStatusDto::Dispatched => AgentTaskStatus::Dispatched,
        AgentTaskStatusDto::Acknowledged => AgentTaskStatus::Acknowledged,
        AgentTaskStatusDto::Active => AgentTaskStatus::Active,
        AgentTaskStatusDto::Running => AgentTaskStatus::Running,
        AgentTaskStatusDto::Submitted => AgentTaskStatus::Submitted,
        AgentTaskStatusDto::Revision => AgentTaskStatus::Revision,
        AgentTaskStatusDto::Blocked => AgentTaskStatus::Blocked,
        AgentTaskStatusDto::Paused => AgentTaskStatus::Paused,
        AgentTaskStatusDto::Completed => AgentTaskStatus::Completed,
        AgentTaskStatusDto::Failed => AgentTaskStatus::Failed,
        AgentTaskStatusDto::Cancelled => AgentTaskStatus::Cancelled,
        AgentTaskStatusDto::Archived => AgentTaskStatus::Archived,
    }
}

fn task_status_to_dto(status: AgentTaskStatus) -> AgentTaskStatusDto {
    match status {
        AgentTaskStatus::Draft => AgentTaskStatusDto::Draft,
        AgentTaskStatus::Queued => AgentTaskStatusDto::Queued,
        AgentTaskStatus::Assigned => AgentTaskStatusDto::Assigned,
        AgentTaskStatus::Dispatched => AgentTaskStatusDto::Dispatched,
        AgentTaskStatus::Acknowledged => AgentTaskStatusDto::Acknowledged,
        AgentTaskStatus::Active => AgentTaskStatusDto::Active,
        AgentTaskStatus::Running => AgentTaskStatusDto::Running,
        AgentTaskStatus::Submitted => AgentTaskStatusDto::Submitted,
        AgentTaskStatus::Revision => AgentTaskStatusDto::Revision,
        AgentTaskStatus::Blocked => AgentTaskStatusDto::Blocked,
        AgentTaskStatus::Paused => AgentTaskStatusDto::Paused,
        AgentTaskStatus::Completed => AgentTaskStatusDto::Completed,
        AgentTaskStatus::Failed => AgentTaskStatusDto::Failed,
        AgentTaskStatus::Cancelled => AgentTaskStatusDto::Cancelled,
        AgentTaskStatus::Archived => AgentTaskStatusDto::Archived,
    }
}

fn capability_kind_from_dto(kind: AgentCapabilityKindDto) -> AgentCapabilityKind {
    match kind {
        AgentCapabilityKindDto::Skill => AgentCapabilityKind::Skill,
        AgentCapabilityKindDto::RemoteSkill => AgentCapabilityKind::RemoteSkill,
        AgentCapabilityKindDto::McpServer => AgentCapabilityKind::McpServer,
        AgentCapabilityKindDto::Model => AgentCapabilityKind::Model,
        AgentCapabilityKindDto::Runtime => AgentCapabilityKind::Runtime,
        AgentCapabilityKindDto::Tool => AgentCapabilityKind::Tool,
    }
}

fn capability_kind_to_dto(kind: AgentCapabilityKind) -> AgentCapabilityKindDto {
    match kind {
        AgentCapabilityKind::Skill => AgentCapabilityKindDto::Skill,
        AgentCapabilityKind::RemoteSkill => AgentCapabilityKindDto::RemoteSkill,
        AgentCapabilityKind::McpServer => AgentCapabilityKindDto::McpServer,
        AgentCapabilityKind::Model => AgentCapabilityKindDto::Model,
        AgentCapabilityKind::Runtime => AgentCapabilityKindDto::Runtime,
        AgentCapabilityKind::Tool => AgentCapabilityKindDto::Tool,
    }
}

fn event_kind_to_dto(kind: AgentEventKind) -> AgentEventKindDto {
    match kind {
        AgentEventKind::TaskCreated => AgentEventKindDto::TaskCreated,
        AgentEventKind::ImportedLegacyTask => AgentEventKindDto::ImportedLegacyTask,
        AgentEventKind::TaskAssigned => AgentEventKindDto::TaskAssigned,
        AgentEventKind::WorkerSelected => AgentEventKindDto::WorkerSelected,
        AgentEventKind::WorkerAcknowledged => AgentEventKindDto::WorkerAcknowledged,
        AgentEventKind::RunStarted => AgentEventKindDto::RunStarted,
        AgentEventKind::RunSubmitted => AgentEventKindDto::RunSubmitted,
        AgentEventKind::RunCompleted => AgentEventKindDto::RunCompleted,
        AgentEventKind::RunFailed => AgentEventKindDto::RunFailed,
        AgentEventKind::RunRecovered => AgentEventKindDto::RunRecovered,
        AgentEventKind::TaskBlocked => AgentEventKindDto::TaskBlocked,
        AgentEventKind::TaskPaused => AgentEventKindDto::TaskPaused,
        AgentEventKind::TaskResumed => AgentEventKindDto::TaskResumed,
        AgentEventKind::TaskCancelled => AgentEventKindDto::TaskCancelled,
        AgentEventKind::TaskArchived => AgentEventKindDto::TaskArchived,
        AgentEventKind::InfiniteScheduled => AgentEventKindDto::InfiniteScheduled,
        AgentEventKind::InfiniteExecuted => AgentEventKindDto::InfiniteExecuted,
        AgentEventKind::TeamNodeQueued => AgentEventKindDto::TeamNodeQueued,
        AgentEventKind::TeamNodeDispatched => AgentEventKindDto::TeamNodeDispatched,
        AgentEventKind::TeamNodeCompleted => AgentEventKindDto::TeamNodeCompleted,
        AgentEventKind::ResourceLeased => AgentEventKindDto::ResourceLeased,
        AgentEventKind::ResourceReleased => AgentEventKindDto::ResourceReleased,
        AgentEventKind::OutboxQueued => AgentEventKindDto::OutboxQueued,
        AgentEventKind::OutboxDelivered => AgentEventKindDto::OutboxDelivered,
        AgentEventKind::OutboxFailed => AgentEventKindDto::OutboxFailed,
        AgentEventKind::HumanIntervened => AgentEventKindDto::HumanIntervened,
    }
}

fn outbox_status_to_dto(status: OutboxMessageStatus) -> AgentOutboxMessageStatusDto {
    match status {
        OutboxMessageStatus::Pending => AgentOutboxMessageStatusDto::Pending,
        OutboxMessageStatus::Claimed => AgentOutboxMessageStatusDto::Sending,
        OutboxMessageStatus::Failed => AgentOutboxMessageStatusDto::Failed,
        OutboxMessageStatus::Delivered => AgentOutboxMessageStatusDto::Delivered,
    }
}

fn workspace_to_dto(
    workspace: StoredAgentRunWorkspaceContext,
) -> AgentSchedulerWorkspaceContextDto {
    AgentSchedulerWorkspaceContextDto {
        slot_id: workspace.workspace.slot_id,
        path: workspace.workspace.path,
        branch: workspace.workspace.branch,
    }
}

fn workspace_context_from_value(value: &Value) -> Option<AgentSchedulerWorkspaceContextDto> {
    let workspace = value.get("workspace").unwrap_or(value);
    let slot_id = string_value(workspace, "slot_id")
        .or_else(|| string_value(value, "selected_worktree_slot_id"))?;
    let path = string_value(workspace, "path")
        .or_else(|| string_value(value, "selected_worktree_path"))?;
    let branch = string_value(workspace, "branch")
        .or_else(|| string_value(value, "selected_worktree_branch"))?;
    Some(AgentSchedulerWorkspaceContextDto { slot_id, path, branch })
}

fn run_stage_from_run_status(status: AgentRunStatus) -> Option<&'static str> {
    match status {
        AgentRunStatus::Pending => Some("planning"),
        AgentRunStatus::Leased | AgentRunStatus::Running => Some("running"),
        AgentRunStatus::Succeeded => Some("succeeded"),
        AgentRunStatus::Failed | AgentRunStatus::TimedOut | AgentRunStatus::Stale => Some("failed"),
        AgentRunStatus::Cancelled => Some("canceled"),
    }
}

fn run_stage_from_task_status(status: AgentTaskStatus) -> Option<&'static str> {
    match status {
        AgentTaskStatus::Assigned
        | AgentTaskStatus::Dispatched
        | AgentTaskStatus::Acknowledged
        | AgentTaskStatus::Active
        | AgentTaskStatus::Running
        | AgentTaskStatus::Submitted
        | AgentTaskStatus::Revision => Some("running"),
        AgentTaskStatus::Completed => Some("succeeded"),
        AgentTaskStatus::Blocked | AgentTaskStatus::Paused | AgentTaskStatus::Failed => {
            Some("failed")
        }
        AgentTaskStatus::Cancelled | AgentTaskStatus::Archived => Some("canceled"),
        AgentTaskStatus::Draft | AgentTaskStatus::Queued => None,
    }
}

fn event_error_message(
    kind: AgentEventKind,
    to_status: Option<AgentTaskStatus>,
    message: &str,
    payload: &Value,
) -> Option<String> {
    string_value(payload, "error_message").or_else(|| string_value(payload, "error")).or_else(
        || {
            let is_error_event =
                kind == AgentEventKind::RunFailed || to_status == Some(AgentTaskStatus::Failed);
            (is_error_event && !message.trim().is_empty()).then(|| message.to_string())
        },
    )
}

fn task_id_from_payload(payload: &Value) -> Option<TaskId> {
    payload
        .get("task_id")
        .and_then(Value::as_str)
        .and_then(non_empty_str)
        .and_then(|value| TaskId::new(value).ok())
}

fn command_kind_key(command: AgentSchedulerCommandKindDto) -> &'static str {
    match command {
        AgentSchedulerCommandKindDto::Start => "start",
        AgentSchedulerCommandKindDto::Retry => "retry",
        AgentSchedulerCommandKindDto::Pause => "pause",
        AgentSchedulerCommandKindDto::Resume => "resume",
        AgentSchedulerCommandKindDto::Cancel => "cancel",
        AgentSchedulerCommandKindDto::Archive => "archive",
        AgentSchedulerCommandKindDto::Promote => "promote",
        AgentSchedulerCommandKindDto::Merge => "merge",
        AgentSchedulerCommandKindDto::Move => "move",
        AgentSchedulerCommandKindDto::UpdateMetadata => "update_metadata",
        AgentSchedulerCommandKindDto::SoftDelete => "soft_delete",
        AgentSchedulerCommandKindDto::Restore => "restore",
        AgentSchedulerCommandKindDto::ImportLegacyTask => "import_legacy_task",
    }
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).and_then(non_empty_str).map(str::to_string)
}

fn reasoning_effort_field(value: &Value) -> Result<Option<ReasoningEffortDto>, ApiError> {
    value
        .get("reasoning_effort")
        .cloned()
        .filter(|value| !value.is_null())
        .map(|value| {
            serde_json::from_value(value).map_err(|error| {
                ApiError::bad_request(format!(
                    "invalid task metadata field reasoning_effort: {error}"
                ))
            })
        })
        .transpose()
}

fn string_value(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).and_then(non_empty_str).map(str::to_string)
}

fn string_array_field(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter_map(non_empty_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn bool_field(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn u32_field(value: &Value, key: &str) -> Option<u32> {
    value.get(key).and_then(Value::as_u64).and_then(|value| u32::try_from(value).ok())
}

fn i64_field(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn non_empty_str(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn optional_text(value: Option<String>) -> Option<String> {
    value.as_deref().and_then(non_empty_str).map(str::to_string)
}

fn optional_worker_name(value: Option<String>) -> Result<Option<AssigneeId>, ApiError> {
    value.map(AssigneeId::new).transpose().map_err(map_bad_request)
}

fn optional_team_id(value: Option<String>) -> Result<Option<TeamId>, ApiError> {
    value.map(TeamId::new).transpose().map_err(map_bad_request)
}

fn parse_project_path(value: &str) -> Result<PathBuf, ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request("project_path must not be empty"));
    }
    Ok(PathBuf::from(trimmed))
}

fn non_empty(value: String, field_name: &str) -> Result<String, ApiError> {
    if value.trim().is_empty() {
        return Err(ApiError::bad_request(format!("{field_name} must not be empty")));
    }
    Ok(value)
}

fn map_bad_request(error: anyhow::Error) -> ApiError {
    ApiError::bad_request(error.to_string())
}

fn map_scheduler_error(error: anyhow::Error) -> ApiError {
    ApiError::internal(error.to_string())
}

fn map_scheduler_command_error(error: anyhow::Error) -> ApiError {
    let message = error.to_string();
    if message.contains("unsupported agent task transition") {
        return ApiError::bad_request(message);
    }
    ApiError::internal(message)
}

fn map_task_board_error(error: anyhow::Error) -> ApiError {
    ApiError::bad_request(error.to_string())
}

#[cfg(test)]
#[derive(Debug, Clone, Default)]
struct FakeAgentSchedulerGateway {
    state: std::sync::Arc<std::sync::Mutex<FakeAgentSchedulerState>>,
}

#[cfg(test)]
#[derive(Debug, Clone, Default)]
struct FakeAgentSchedulerState {
    tasks: Vec<FakeTaskRecord>,
    events: Vec<AgentEventDto>,
    outbox: Vec<FakeOutboxRecord>,
    scoped_contexts: Vec<AgentSchedulerContext>,
}

#[cfg(test)]
#[derive(Debug, Clone)]
struct FakeTaskRecord {
    project_path: String,
    task: AgentTaskDto,
}

#[cfg(test)]
#[derive(Debug, Clone)]
struct FakeOutboxRecord {
    project_path: String,
    message: AgentOutboxMessageDto,
}

#[cfg(test)]
#[async_trait]
impl AgentSchedulerGateway for FakeAgentSchedulerGateway {
    async fn create_task(
        &self,
        request: AgentSchedulerCreateTaskRequest,
    ) -> Result<AgentSchedulerCreateTaskResponse, ApiError> {
        let task = AgentTaskDto {
            id: request.task_id,
            title: request.title,
            kind: request.kind,
            status: request.status.unwrap_or(AgentTaskStatusDto::Queued),
            assigned_to: request.assigned_to,
            team_id: request.team_id,
            room_id: request.room_id,
            project_room_id: request.project_room_id,
            delegated_to_team: request.delegated_to_team,
            schedule: request.schedule,
            timezone: request.timezone,
            next_scheduled_at: request.next_scheduled_at,
            last_executed_at: request.last_executed_at,
            dag: Vec::new(),
            created_at_ms: request.created_at_ms,
            updated_at_ms: request.created_at_ms,
            metadata: request.metadata,
        };
        let mut state = self.state.lock().expect("fake scheduler state lock poisoned");
        state.events.push(AgentEventDto {
            id: "1".to_string(),
            task_id: Some(task.id.clone()),
            run_id: None,
            kind: AgentEventKindDto::TaskCreated,
            message: format!("created {}", task.id),
            payload: json!({ "task_id": task.id }),
            created_at_ms: task.created_at_ms,
        });
        state.tasks.push(FakeTaskRecord { project_path: String::new(), task: task.clone() });
        Ok(AgentSchedulerCreateTaskResponse { task_id: task.id.clone(), task })
    }

    async fn list_tasks(
        &self,
        query: AgentSchedulerTasksQuery,
    ) -> Result<AgentSchedulerTasksResponse, ApiError> {
        let state = self.state.lock().expect("fake scheduler state lock poisoned");
        let tasks = state
            .tasks
            .iter()
            .filter(|record| record.project_path == query.project_path)
            .map(|record| record.task.clone())
            .collect();
        Ok(AgentSchedulerTasksResponse { tasks })
    }

    async fn wake(
        &self,
        request: AgentSchedulerWakeRequest,
    ) -> Result<AgentSchedulerWakeResponse, ApiError> {
        let mut state = self.state.lock().expect("fake scheduler state lock poisoned");
        let Some(record) = state.tasks.iter_mut().find(|record| {
            record.project_path == request.project_path
                && record.task.status == AgentTaskStatusDto::Queued
        }) else {
            return Ok(AgentSchedulerWakeResponse {
                dispatched: false,
                task_id: None,
                run_id: None,
                worker_id: None,
                outbox_message_id: None,
            });
        };
        record.task.status = AgentTaskStatusDto::Dispatched;
        let task_id = record.task.id.clone();
        let outbox_message_id = format!("outbox-{task_id}");
        state.events.push(AgentEventDto {
            id: "2".to_string(),
            task_id: Some(task_id.clone()),
            run_id: Some(format!("run-{task_id}")),
            kind: AgentEventKindDto::OutboxQueued,
            message: format!("queued local dispatch for {task_id}"),
            payload: json!({ "outbox_message_id": outbox_message_id }),
            created_at_ms: request.now_ms,
        });
        state.outbox.push(FakeOutboxRecord {
            project_path: request.project_path,
            message: AgentOutboxMessageDto {
                id: outbox_message_id.clone(),
                target_kind: "local".to_string(),
                target_id: "local-runner".to_string(),
                recipient_id: None,
                body: format!("New task [{task_id}]"),
                status: AgentOutboxMessageStatusDto::Pending,
                idempotency_key: format!("{task_id}:dispatch:worker-api"),
                attempts: 0,
                next_attempt_at_ms: None,
                delivered_at_ms: None,
                last_error: None,
                payload: json!({ "task_id": task_id }),
                created_at_ms: request.now_ms,
            },
        });
        Ok(AgentSchedulerWakeResponse {
            dispatched: true,
            task_id: Some(task_id.clone()),
            run_id: Some(format!("run-{task_id}")),
            worker_id: Some("worker-api".to_string()),
            outbox_message_id: Some(outbox_message_id),
        })
    }

    async fn recover(
        &self,
        _request: AgentSchedulerRecoveryRequest,
    ) -> Result<AgentSchedulerRecoveryResponse, ApiError> {
        Ok(AgentSchedulerRecoveryResponse {
            graced_run_ids: Vec::new(),
            recovered_run_ids: Vec::new(),
            released_resource_lease_ids: Vec::new(),
            tainted_resource_lease_ids: Vec::new(),
            drifted_worker_ids: Vec::new(),
            slept_worker_ids: Vec::new(),
            outbox_message_ids: Vec::new(),
        })
    }

    async fn list_events(
        &self,
        query: AgentSchedulerEventsQuery,
    ) -> Result<AgentSchedulerEventsResponse, ApiError> {
        let after_cursor = query.after_cursor.unwrap_or(0);
        let limit = query.limit.unwrap_or(DEFAULT_EVENT_LIMIT);
        let state = self.state.lock().expect("fake scheduler state lock poisoned");
        let events = state
            .events
            .iter()
            .filter(|event| event.id.parse::<i64>().unwrap_or(0) > after_cursor)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let cursor =
            events.last().and_then(|event| event.id.parse::<i64>().ok()).unwrap_or(after_cursor);
        Ok(AgentSchedulerEventsResponse { cursor, events })
    }

    async fn drain_outbox(
        &self,
        request: AgentSchedulerOutboxDrainRequest,
    ) -> Result<AgentSchedulerOutboxDrainResponse, ApiError> {
        let limit = request.limit.unwrap_or(DEFAULT_OUTBOX_DRAIN_LIMIT);
        let state = self.state.lock().expect("fake scheduler state lock poisoned");
        let messages = state
            .outbox
            .iter()
            .filter(|record| record.project_path == request.project_path)
            .take(limit)
            .map(|record| record.message.clone())
            .collect::<Vec<_>>();
        Ok(AgentSchedulerOutboxDrainResponse { claimed: messages.len(), messages })
    }

    async fn task_board_events(
        &self,
        _project_id: String,
        context: AgentSchedulerContext,
        _query: AgentSchedulerProjectEventsQuery,
    ) -> Result<AgentSchedulerTaskBoardEventsResponse, ApiError> {
        self.state
            .lock()
            .expect("fake scheduler state lock poisoned")
            .scoped_contexts
            .push(context);
        Ok(AgentSchedulerTaskBoardEventsResponse { cursor: 0, events: Vec::new() })
    }
}

#[cfg(test)]
#[path = "agent_scheduler_tests.rs"]
mod agent_scheduler_tests;

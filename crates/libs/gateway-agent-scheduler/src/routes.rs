//! 项目作用域调度路由。

use anyhow::Result;
use gateway_types::ApiError;
use task_scheduler_core::domain::AgentSchedulerContext;
use vw_api_agent_scheduler::{
    AgentSchedulerBulkCommandRequest,
    AgentSchedulerBulkCommandResponse,
    AgentSchedulerCreateTaskRequest,
    AgentSchedulerCreateTaskResponse,
    AgentSchedulerLegacyImportRequest,
    AgentSchedulerLegacyImportResponse,
    AgentSchedulerReplaceTaskGraphRequest,
    AgentSchedulerScope,
    AgentSchedulerTaskArtifactsResponse,
    AgentSchedulerTaskBoardEventsResponse,
    AgentSchedulerTaskBoardStatusResponse,
    AgentSchedulerTaskBoardWorkflowResponse,
    AgentSchedulerTaskGraphResponse,
    AgentSchedulerTaskLogsResponse,
    AgentSchedulerTaskMetadataUpdateRequest,
    AgentSchedulerTaskMetadataUpdateResponse,
};
use vw_api_agent_scheduler::{
    AgentSchedulerCommandRequest as AgentSchedulerProjectCommandRequest,
    AgentSchedulerCommandResponse as AgentSchedulerProjectCommandResponse,
    AgentSchedulerWakeRequest as AgentSchedulerProjectWakeRequest,
    AgentSchedulerWakeResponse as AgentSchedulerProjectWakeResponse,
    AgentSchedulerOutboxDrainRequest as AgentSchedulerProjectOutboxDrainRequest,
};
use crate::types::*;
use std::convert::Infallible;
use std::sync::Arc;
use axum::extract::{Extension, Path as AxumPath, Query};
use axum::response::{Sse, sse::{Event, KeepAlive}};
use axum::routing::{patch, post};
use axum::{Json, Router};
use futures_util::stream;
use vw_api_transport::id::{ProjectId, ProjectScope, TenantScope};
use agent_project as project;
use crate::AgentSchedulerGateway;

type SharedAgentSchedulerGateway = Arc<dyn AgentSchedulerGateway>;

pub fn router_with_gateway<S>(gateway: SharedAgentSchedulerGateway) -> Router<S>
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

    router
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

pub async fn agent_scheduler_project_create_task(
    AxumPath(project_id): AxumPath<String>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
    Json(body): Json<AgentSchedulerCreateTaskRequest>,
) -> Result<Json<AgentSchedulerCreateTaskResponse>, ApiError> {
    let context = resolve_project_context(tenant, &project_id).await?;
    gateway.create_task_scoped(context, body).await.map(Json)
}

pub async fn agent_scheduler_project_task_graph(
    AxumPath(project_id): AxumPath<String>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
) -> Result<Json<AgentSchedulerTaskGraphResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    gateway.task_graph(project_id, project_path).await.map(Json)
}

pub async fn agent_scheduler_project_replace_task_graph(
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

pub async fn agent_scheduler_project_drain_outbox(
    AxumPath(project_id): AxumPath<String>,
    Extension(tenant): Extension<TenantScope>,
    Extension(gateway): Extension<SharedAgentSchedulerGateway>,
    Json(body): Json<AgentSchedulerProjectOutboxDrainRequest>,
) -> Result<Json<AgentSchedulerProjectOutboxDrainResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    gateway.drain_project_outbox(project_path, body).await.map(Json)
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
        .map_err(|error| ApiError::internal(error.to_string()))
}

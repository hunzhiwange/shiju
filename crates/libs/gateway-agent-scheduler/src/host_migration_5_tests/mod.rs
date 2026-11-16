//! Store-backed gateway migration fixtures; external execution ports fail closed.
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use axum::extract::{Extension, Query};

use axum::Json;
use serde_json::json;

use vw_api_agent_scheduler::{AgentCapabilityKindDto, AgentEventDto, AgentEventKindDto, AgentOutboxMessageDto, AgentOutboxMessageStatusDto, AgentSchedulerBulkCommandRequest, AgentSchedulerCommandKindDto, AgentSchedulerCreateTaskRequest, AgentSchedulerCreateTaskResponse, AgentSchedulerDispatchModeDto, AgentSchedulerLegacyImportRequest, AgentSchedulerReplaceTaskGraphRequest, AgentSchedulerTaskBoardEventsResponse, AgentSchedulerTaskGraphEdgeDto, AgentSchedulerTaskMetadataUpdateRequest, AgentSchedulerWakeRequest as AgentSchedulerProjectWakeRequest, AgentTaskDto, AgentTaskKindDto, AgentTaskStatusDto};
use vw_api_agent_scheduler::AgentSchedulerScope;
use vw_api_transport::common::TimestampMs;
use vw_api_transport::id::{ProjectId, ProjectScope, TenantScope};
use vw_api_task::{TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusRoleDto};

use task_scheduler::domain::{AgentSchedulerContext, RunId, TaskId};

use task_scheduler::events::AgentEventKind;


use task_scheduler::runner::gateway_request::{append_gateway_stream_payload_task_log, gateway_local_agent_stream_request};
use task_scheduler::runner::local::{LocalAgentExecutionOutput, LocalAgentExecutionRequest, LocalAgentWorkspace, LocalAgentWorkspaceRequest};
use task_scheduler::runner::local::{LocalAgentExecutor};
use task_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
use task_scheduler::store::artifacts::StoredTaskArtifact;

use task_scheduler::store::logs::{StoredAgentTaskLog};

use task_scheduler::store::tasks::{NewAgentTask};
use task_scheduler::store::{AgentSchedulerStore};

use task_scheduler::task_graph::{AgentTaskDependency};
use gateway_types::ApiError;
use agent_project as project;

use task_scheduler::store::AgentSchedulerStorageContext;
use crate::store_gateway::{start_command_is_idempotent_for_status, normalize_project_wake_max_concurrent, execution_slot_status, task_board_write_command, owned_scheduler_context, add_ms_or_saturate, command_event_kind, task_board_status_role_from_metadata, task_board_status_key, legacy_task_board_status_key, valid_merge_branch, workspace_context_from_value, event_error_message, command_kind_key, string_field, string_array_field, bool_field, u32_field, i64_field, non_empty_str, optional_text, parse_project_path, map_bad_request, map_scheduler_command_error, map_task_board_error};
const DEFAULT_EVENT_LIMIT: usize = 100;
const DEFAULT_OUTBOX_DRAIN_LIMIT: usize = 50;

pub(super) async fn test_default_tenant_scope(
    mut request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if request.extensions().get::<TenantScope>().is_none() {
        request.extensions_mut().insert(project::default_tenant_scope());
    }
    next.run(request).await
}

use crate::*;
use self::host::TestHost;
type StoreBackedAgentSchedulerGateway = crate::store_gateway::StoreBackedAgentSchedulerGateway<TestHost>;
fn router<S: Clone + Send + Sync + 'static>() -> axum::Router<S> { router_with_gateway(std::sync::Arc::new(StoreBackedAgentSchedulerGateway::default())) }
fn router_with_gateway<S: Clone + Send + Sync + 'static>(gateway: std::sync::Arc<dyn AgentSchedulerGateway>) -> axum::Router<S> { crate::router_with_gateway(gateway).layer(axum::middleware::from_fn(test_default_tenant_scope)) }
mod host;

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

#[derive(Debug, Clone, Default)]
struct FakeAgentSchedulerGateway {
    state: std::sync::Arc<std::sync::Mutex<FakeAgentSchedulerState>>,
}

#[derive(Debug, Clone, Default)]
struct FakeAgentSchedulerState {
    tasks: Vec<FakeTaskRecord>,
    events: Vec<AgentEventDto>,
    outbox: Vec<FakeOutboxRecord>,
    scoped_contexts: Vec<AgentSchedulerContext>,
}

#[derive(Debug, Clone)]
struct FakeTaskRecord {
    project_path: String,
    task: AgentTaskDto,
}

#[derive(Debug, Clone)]
struct FakeOutboxRecord {
    project_path: String,
    message: AgentOutboxMessageDto,
}

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

mod cases;

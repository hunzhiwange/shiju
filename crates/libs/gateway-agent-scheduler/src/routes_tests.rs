//! 路由契约测试；宿主执行、存储与中间件集成测试保留在 vw-agent。

use super::*;
use agent_project as project;
use axum::{Json, extract::{Extension, Path as AxumPath}};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use gateway_types::ApiError;
use std::sync::Arc;
use tower::ServiceExt;
use serde_json::json;
use vw_api_agent_scheduler::{
    AgentSchedulerCreateTaskRequest, AgentSchedulerCreateTaskResponse,
    AgentSchedulerOutboxDrainRequest as AgentSchedulerProjectOutboxDrainRequest,
    AgentTaskDto, AgentTaskKindDto, AgentTaskStatusDto,
};
use vw_api_transport::common::TimestampMs;

#[tokio::test]
async fn agent_scheduler_routes_exclude_removed_remote_resources() {
    let app = router_with_gateway::<()>(Arc::new(MinimalGateway)).with_state(());
    for path in [
        "/agent-scheduler/projects/project-a/workers",
        "/agent-scheduler/projects/project-a/teams",
        "/agent-scheduler/projects/project-a/tasks/task-a/runs/run-a/worker-events",
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).expect("request"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "unexpected route: {path}");
    }
}

#[tokio::test]
async fn scheduler_project_route_wrappers_resolve_context_before_gateway_calls() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-route-wrappers");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let project_id = scheduler_project_id(&project_path);
    let tenant = project::default_tenant_scope();
    let gateway: Arc<dyn AgentSchedulerGateway> = Arc::new(MinimalGateway);

    let created = agent_scheduler_project_create_task(
        AxumPath(project_id.clone()),
        Extension(tenant.clone()),
        Extension(Arc::clone(&gateway)),
        Json(create_task_request("task-project-wrapper")),
    )
    .await
    .expect("project scoped create")
    .0;
    assert_eq!(created.task_id, "task-project-wrapper");

    assert_eq!(
        agent_scheduler_project_drain_outbox(
            AxumPath(project_id),
            Extension(tenant),
            Extension(gateway),
            Json(AgentSchedulerProjectOutboxDrainRequest {
                now_ms: TimestampMs(70),
                limit: Some(1),
            }),
        )
        .await
        .expect_err("default drain should fail")
        .to_string(),
        "agent scheduler project outbox gateway is not configured"
    );
}

#[derive(Default)]
struct MinimalGateway;

#[async_trait::async_trait]
impl AgentSchedulerGateway for MinimalGateway {
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
        Ok(AgentSchedulerCreateTaskResponse { task_id: task.id.clone(), task })
    }

    async fn list_tasks(
        &self,
        _query: AgentSchedulerTasksQuery,
    ) -> Result<AgentSchedulerTasksResponse, ApiError> {
        Ok(AgentSchedulerTasksResponse { tasks: Vec::new() })
    }

    async fn list_events(
        &self,
        _query: AgentSchedulerEventsQuery,
    ) -> Result<AgentSchedulerEventsResponse, ApiError> {
        Ok(AgentSchedulerEventsResponse { cursor: 0, events: Vec::new() })
    }

    async fn drain_outbox(
        &self,
        _request: AgentSchedulerOutboxDrainRequest,
    ) -> Result<AgentSchedulerOutboxDrainResponse, ApiError> {
        Ok(AgentSchedulerOutboxDrainResponse { claimed: 0, messages: Vec::new() })
    }
}

fn create_task_request(task_id: &str) -> AgentSchedulerCreateTaskRequest {
    AgentSchedulerCreateTaskRequest {
        task_id: task_id.to_string(),
        title: "Create through gateway".to_string(),
        kind: AgentTaskKindDto::Finite,
        status: Some(AgentTaskStatusDto::Queued),
        spec_uri: "spec.md".to_string(),
        meta_uri: "meta.json".to_string(),
        assigned_to: None,
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        metadata: json!({}),
        created_at_ms: TimestampMs(40),
    }
}

fn scheduler_project_id(project_path: &std::path::Path) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    project_path.hash(&mut hasher);
    let project_id = format!("test-project-{:016x}", hasher.finish());
    project::store_local_test_project(&project::Info {
        id: project_id.clone(),
        worktree: project_path.to_string_lossy().into_owned(),
        vcs: None,
        name: Some("Agent Scheduler test project".to_owned()),
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: Vec::new(),
    })
    .expect("register scheduler test project");
    project_id
}

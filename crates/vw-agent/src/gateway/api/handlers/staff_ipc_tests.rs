use super::*;
use axum::Json;
use axum::body::{Body, to_bytes};
use axum::extract::{Extension, Path as AxumPath, State};
use axum::http::{Method, Request, StatusCode, header};
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_api_types::staff::{
    StaffCapabilityIoSummaryDto, StaffCapabilityKindDto, StaffCommandDto, StaffCommandRequest,
    StaffIpcMessageRequest, StaffMemberTaskAssignmentDto, StaffProjectCapabilityCatalogRequest,
    StaffProjectCapabilityCatalogResponse, StaffProjectCapabilityDto,
    StaffProjectOrchestrationRequest, StaffProjectStateDto, StaffStageActionDto,
    StaffStageActionInputsDto, StaffStageAssignmentDto, StaffWorkflowOrchestrationDto,
    StaffWorkflowOrchestrationRequest, StaffWorkflowOrchestrationResponse,
};
use vw_config_types::staff::{
    StaffMemberTaskAssignmentConfig, StaffProjectStateConfig, StaffWorkflowOrchestrationConfig,
};

use crate::agent_scheduler::domain::{AgentSchedulerContext, TaskId};
use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::agent_scheduler::store::AgentSchedulerStore;
use crate::agent_scheduler::store::tasks::NewAgentTask;

fn staff_tenant() -> Extension<TenantScope> {
    Extension(TenantScope::default())
}

fn staff_project_path(project_id: impl Into<String>) -> AxumPath<String> {
    AxumPath(project_id.into())
}

fn staff_workflow_path(
    project_id: impl Into<String>,
    workflow_id: impl Into<String>,
) -> AxumPath<(String, String)> {
    AxumPath((project_id.into(), workflow_id.into()))
}

fn register_staff_project(project_path: &std::path::Path) -> (String, AgentSchedulerContext) {
    let project_id = format!("staff-project-{}", Uuid::new_v4());
    let now = 1_783_000_000_000;
    let info = crate::app::agent::project::Info {
        id: project_id.clone(),
        worktree: project_id.clone(),
        vcs: None,
        name: None,
        icon: None,
        commands: None,
        time: crate::app::agent::project::TimeInfo {
            created: now,
            updated: now,
            initialized: None,
        },
        sandboxes: Vec::new(),
    };
    crate::app::agent::project::runtime_store()
        .expect("project store")
        .create(&TenantScope::default(), &info)
        .expect("register staff project");
    let project_scope =
        ProjectScope::new(TenantScope::default(), ProjectId::from(project_id.as_str()))
            .expect("project scope");
    let context = AgentSchedulerContext::new(AgentSchedulerScope::new(project_scope), project_path)
        .expect("scheduler context");
    (project_id, context)
}

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl crate::app::agent::providers::Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("ok".to_string())
    }
}

fn state_with_agents_ipc(enabled: bool) -> AppState {
    let mut config = crate::app::agent::config::Config::default();
    config.agents_ipc.enabled = enabled;
    config.agents_ipc.db_path =
        tempfile::NamedTempFile::new().expect("temp db").path().to_string_lossy().to_string();

    AppState {
        config: Arc::new(Mutex::new(config)),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(crate::app::agent::memory::NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(crate::app::agent::security::pairing::PairingGuard::new(false, &[])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(crate::app::agent::gateway::GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(crate::app::agent::gateway::IdempotencyStore::new(
            std::time::Duration::from_secs(300),
            1000,
        )),
        whatsapp: None,
        whatsapp_app_secret: None,
        linq: None,
        linq_signing_secret: None,
        nextcloud_talk: None,
        nextcloud_talk_webhook_secret: None,
        wati: None,
        qq: None,
        qq_webhook_enabled: false,
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_tool_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
    }
}

#[tokio::test]
async fn staff_ipc_message_handler_rejects_disabled_config() {
    let request = StaffIpcMessageRequest {
        to_agent: "agent-a".to_string(),
        payload: "handoff ready".to_string(),
        task_id: Some("task-1".to_string()),
        from_member_id: Some("planner".to_string()),
        to_member_id: Some("coder".to_string()),
    };

    let error = handle_api_staff_ipc_message(
        State(state_with_agents_ipc(false)),
        staff_tenant(),
        Json(request),
    )
    .await
    .expect_err("disabled IPC config should be rejected");

    assert_eq!(error.status, StatusCode::NOT_IMPLEMENTED);
    assert!(error.to_string().contains("agents_ipc.enabled"));
}

#[tokio::test]
async fn staff_routes_require_header_tenant_scope() {
    let app = router()
        .layer(axum::middleware::from_fn(
            crate::app::agent::gateway::middleware::tenant_scope_middleware,
        ))
        .with_state(state_with_agents_ipc(false));

    let legacy = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(
                    "/staff/platforms/platform-a/companies/company-a/projects/project-a/projection",
                )
                .header(vw_api_types::id::PLATFORM_ID_HEADER, "platform-a")
                .header(vw_api_types::id::COMPANY_ID_HEADER, "company-a")
                .body(Body::empty())
                .expect("legacy request"),
        )
        .await
        .expect("legacy response");
    assert_eq!(legacy.status(), StatusCode::NOT_FOUND);

    let missing_tenant = app
        .oneshot(
            Request::builder()
                .uri("/staff/stage-capabilities")
                .body(Body::empty())
                .expect("request without tenant"),
        )
        .await
        .expect("missing tenant response");
    assert_eq!(missing_tenant.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn staff_project_context_uses_header_tenant_scope_and_isolates_same_project_id() {
    let project_id = format!("staff-tenant-scope-{}", Uuid::new_v4());
    let tenant_a = TenantScope::new("staff-platform-a", "staff-company-a").expect("tenant a");
    let tenant_b = TenantScope::new("staff-platform-b", "staff-company-b").expect("tenant b");
    let temp = tempfile::tempdir().expect("temp project root");
    let worktree_a = temp.path().join("tenant-a");
    let worktree_b = temp.path().join("tenant-b");
    std::fs::create_dir_all(&worktree_a).expect("tenant a worktree");
    std::fs::create_dir_all(&worktree_b).expect("tenant b worktree");

    let store = crate::app::agent::project::runtime_store().expect("project store");
    let project_a = crate::app::agent::project::Info {
        id: project_id.clone(),
        worktree: worktree_a.to_string_lossy().to_string(),
        vcs: None,
        name: None,
        icon: None,
        commands: None,
        time: crate::app::agent::project::TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: Vec::new(),
    };
    let project_b = crate::app::agent::project::Info {
        worktree: worktree_b.to_string_lossy().to_string(),
        ..project_a.clone()
    };
    store.create(&tenant_a, &project_a).expect("store tenant a project");
    store.create(&tenant_b, &project_b).expect("store tenant b project");

    let context_a =
        resolve_project_context(tenant_a.clone(), &project_id).await.expect("tenant a context");
    let context_b =
        resolve_project_context(tenant_b.clone(), &project_id).await.expect("tenant b context");
    assert_eq!(context_a.scope().project_scope().tenant_scope(), &tenant_a);
    assert_eq!(context_a.project_path(), worktree_a);
    assert_eq!(context_b.scope().project_scope().tenant_scope(), &tenant_b);
    assert_eq!(context_b.project_path(), worktree_b);

    for tenant in [&tenant_a, &tenant_b] {
        let scope = ProjectScope::new(tenant.clone(), ProjectId::from(project_id.as_str()))
            .expect("project scope");
        store.delete(&scope).expect("remove test project");
    }
}

#[tokio::test]
async fn staff_command_handler_validates_and_executes_assignment() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-staff-command");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let store = AgentSchedulerStore::new();
    let (project_id, project_context) = register_staff_project(&project_path);
    store.initialize_project(&project_context).expect("init scheduler");
    store
        .create_task(
            &project_context,
            NewAgentTask {
                id: TaskId::new("task-staff-assign").expect("task id"),
                title: "Assign staff from gateway".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-staff-assign/spec.md".to_string(),
                meta_uri: "shared/tasks/task-staff-assign/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({
                    "task_board.status_key": "pending",
                    "task_board.status_role": "queued",
                    "priority": 1,
                    "order_no": 0
                }),
                created_at_ms: TimestampMs(1_783_000_000_000),
            },
        )
        .expect("seed task");
    let state = state_with_agents_ipc(false);
    let request = StaffCommandRequest {
        command: StaffCommandDto::AssignTask {
            command_id: "cmd-staff-assign".to_string(),
            task_id: "task-staff-assign".to_string(),
            member_id: "staff-orchestrator".to_string(),
        },
        dry_run: false,
    };

    let response = handle_api_staff_command(
        State(state),
        staff_tenant(),
        staff_project_path(project_id),
        Json(request),
    )
    .await
    .expect("staff command");

    let response = response.0;
    assert!(response.accepted);
    assert!(matches!(
        response.action,
        Some(vw_api_types::staff::StaffCommandActionDto::AssignTask { .. })
    ));
    assert_eq!(
        response.scheduler_result.as_ref().and_then(|result| result.task.as_ref()).map(|task| {
            task.read_model
                .staff
                .as_ref()
                .and_then(|staff| staff.member_id.as_deref())
                .map(str::to_string)
        }),
        Some(Some("staff-orchestrator".to_string()))
    );

    let task = store
        .load_task(&project_context, &TaskId::new("task-staff-assign").expect("task id"))
        .expect("load task")
        .expect("task");
    assert_eq!(task.metadata["task_board.staff.member_id"], "staff-orchestrator");
}

#[tokio::test]
async fn staff_projection_handler_returns_capabilities_and_lanes() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-staff-projection");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let store = AgentSchedulerStore::new();
    let (project_id, project_context) = register_staff_project(&project_path);
    store.initialize_project(&project_context).expect("init scheduler");
    store
        .create_task(
            &project_context,
            NewAgentTask {
                id: TaskId::new("task-staff-projection").expect("task id"),
                title: "Projection task".to_string(),
                kind: AgentTaskKind::Finite,
                status: AgentTaskStatus::Queued,
                spec_uri: "shared/tasks/task-staff-projection/spec.md".to_string(),
                meta_uri: "shared/tasks/task-staff-projection/meta.json".to_string(),
                assigned_to: None,
                team_id: None,
                room_id: None,
                project_room_id: None,
                delegated_to_team: None,
                schedule: None,
                timezone: None,
                next_scheduled_at: None,
                last_executed_at: None,
                dag: Vec::new(),
                metadata: json!({"task_board.status_key": "pending"}),
                created_at_ms: TimestampMs(1_783_000_000_000),
            },
        )
        .expect("seed task");

    let response = handle_api_staff_projection(
        State(state_with_agents_ipc(false)),
        staff_tenant(),
        staff_project_path(project_id),
    )
    .await
    .expect("staff projection");

    let response = response.0;
    assert!(!response.capabilities.is_empty());
    assert_eq!(response.projection.total_task_count, 1);
    assert!(response.projection.lanes.iter().any(|lane| lane.status_key == "pending"));
}

#[tokio::test]
async fn staff_api_manages_project_capability_catalog() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-capability-catalog");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let (project_id, _project_context) = register_staff_project(&project_path);
    let state = state_with_agents_ipc(false);
    let config_path = temp.path().join(".vibewindow.json");
    let workspace_dir = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace_dir).expect("workspace dir");
    {
        let mut config = state.config.lock();
        config.config_path = config_path.clone();
        config.workspace_dir = workspace_dir.clone();
    }
    let project_segment = encode_path_segment(&project_id);
    let app = router().layer(Extension(TenantScope::default())).with_state(state.clone());
    let request = StaffProjectCapabilityCatalogRequest {
        capabilities: vec![StaffProjectCapabilityDto {
            id: " release_workflow ".to_string(),
            label: " Release Workflow ".to_string(),
            capability_type: StaffCapabilityKindDto::SavedWorkflow,
            built_in: false,
            input_summary: StaffCapabilityIoSummaryDto {
                description: " Workflow UUID and release channel ".to_string(),
                fields: vec![" workflow_uuid ".to_string(), "release_channel".to_string()],
            },
            result_summary: StaffCapabilityIoSummaryDto {
                description: " Release automation result ".to_string(),
                fields: vec![" summary ".to_string()],
            },
        }],
    };

    let put_response = app
        .clone()
        .oneshot(json_request(
            Method::PUT,
            &format!("/staff/projects/{project_segment}/capabilities"),
            &request,
        ))
        .await
        .expect("put project capability catalog");

    assert_eq!(put_response.status(), StatusCode::OK);
    let saved: StaffProjectCapabilityCatalogResponse = response_json(put_response).await;
    let saved_ids = saved
        .capabilities
        .iter()
        .map(|capability| capability.capability_id.as_str())
        .collect::<Vec<_>>();
    assert!(saved_ids.contains(&"manual"));
    assert!(saved_ids.contains(&"release_workflow"));
    let custom = saved
        .capabilities
        .iter()
        .find(|capability| capability.capability_id == "release_workflow")
        .expect("saved custom capability");
    assert_eq!(custom.label, "Release Workflow");
    assert_eq!(custom.input_summary.fields, vec!["workflow_uuid", "release_channel"]);

    let get_response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/staff/projects/{project_segment}/capabilities"))
                .body(Body::empty())
                .expect("get request"),
        )
        .await
        .expect("get project capability catalog");

    assert_eq!(get_response.status(), StatusCode::OK);
    let loaded: StaffProjectCapabilityCatalogResponse = response_json(get_response).await;
    assert_eq!(loaded.capabilities, saved.capabilities);

    let reloaded_state = state_reloaded_from_config(&config_path, workspace_dir).await;
    let reloaded = handle_api_staff_project_capabilities(
        State(reloaded_state),
        staff_tenant(),
        staff_project_path(project_id.clone()),
    )
    .await
    .expect("reload project capability catalog after restart")
    .0;
    assert_eq!(reloaded.capabilities, saved.capabilities);

    let projection =
        handle_api_staff_projection(State(state), staff_tenant(), staff_project_path(project_id))
            .await
            .expect("staff projection")
            .0;
    assert!(projection.capabilities.iter().any(|capability| {
        capability.capability_id == "release_workflow" && !capability.built_in
    }));
}

#[tokio::test]
async fn staff_orchestration_persists_project_state_across_gateway_restart() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-staff-orchestration");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let (project_id, _project_context) = register_staff_project(&project_path);
    let state = state_with_agents_ipc(false);
    let config_path = temp.path().join(".vibewindow.json");
    let workspace_dir = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace_dir).expect("workspace dir");
    {
        let mut config = state.config.lock();
        config.config_path = config_path.clone();
        config.workspace_dir = workspace_dir.clone();
    }
    let workflow = workflow_snapshot_from_staff_config(&state.config.lock().staff);
    let request = StaffProjectOrchestrationRequest {
        workflow,
        project_state: StaffProjectStateDto {
            stage_assignments: vec![StaffStageAssignmentDto {
                stage_id: "planning".to_string(),
                primary_member_id: Some("staff-planner".to_string()),
                backup_member_ids: Vec::new(),
                manager_member_ids: vec!["staff-orchestrator".to_string()],
                autonomous: true,
            }],
            stage_actions: vec![StaffStageActionDto {
                stage_id: "planning".to_string(),
                capability_id: "execute_task".to_string(),
                workflow_uuid: None,
                inputs: StaffStageActionInputsDto::default(),
            }],
            ..StaffProjectStateDto::default()
        },
    };

    let _saved = handle_api_staff_project_orchestration_put(
        State(state),
        staff_tenant(),
        staff_project_path(project_id.clone()),
        Json(request),
    )
    .await
    .expect("save project orchestration");

    let restarted = state_reloaded_from_config(&config_path, workspace_dir).await;
    let loaded = handle_api_staff_project_orchestration(
        State(restarted),
        staff_tenant(),
        staff_project_path(project_id),
    )
    .await
    .expect("load project orchestration after restart")
    .0;

    assert!(
        loaded
            .project_state
            .stage_assignments
            .iter()
            .any(|assignment| assignment.stage_id == "planning"
                && assignment.primary_member_id.as_deref() == Some("staff-planner"))
    );
    assert!(
        loaded.project_state.stage_actions.iter().any(|action| {
            action.stage_id == "planning" && action.capability_id == "execute_task"
        })
    );
}

#[tokio::test]
async fn staff_workflow_orchestration_persists_system_state_across_gateway_restart() {
    let temp = tempfile::tempdir().expect("temp project root");
    let config_path = temp.path().join(".vibewindow.json");
    let workspace_dir = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace_dir).expect("workspace dir");

    let project_path = temp.path().join("project-without-system-orchestration-state");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let (project_id, project_context) = register_staff_project(&project_path);
    let project_segment = encode_path_segment(&project_id);
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_context).expect("init scheduler");

    let state = state_with_agents_ipc(false);
    {
        let mut config = state.config.lock();
        config.config_path = config_path.clone();
        config.workspace_dir = workspace_dir.clone();
    }
    let workflow_id = "dev_lifecycle";
    let workflow_segment = encode_path_segment(workflow_id);
    let app = router().layer(Extension(TenantScope::default())).with_state(state);
    let request = StaffWorkflowOrchestrationRequest {
        workflow_id: workflow_id.to_string(),
        orchestration: StaffWorkflowOrchestrationDto {
            capabilities: vec![StaffProjectCapabilityDto {
                id: " product_review ".to_string(),
                label: " Product Review ".to_string(),
                capability_type: StaffCapabilityKindDto::SavedWorkflow,
                built_in: false,
                input_summary: StaffCapabilityIoSummaryDto {
                    description: " Workflow UUID and review focus ".to_string(),
                    fields: vec![" workflow_uuid ".to_string(), "focus".to_string()],
                },
                result_summary: StaffCapabilityIoSummaryDto {
                    description: " Review result ".to_string(),
                    fields: vec![" status ".to_string()],
                },
            }],
            stage_assignments: vec![StaffStageAssignmentDto {
                stage_id: " planning ".to_string(),
                primary_member_id: Some(" staff-planner ".to_string()),
                backup_member_ids: Vec::new(),
                manager_member_ids: vec![" staff-orchestrator ".to_string()],
                autonomous: true,
            }],
            stage_workflow_bindings: Vec::new(),
            stage_actions: vec![StaffStageActionDto {
                stage_id: " planning ".to_string(),
                capability_id: " execute_task ".to_string(),
                workflow_uuid: None,
                inputs: StaffStageActionInputsDto::default(),
            }],
        },
    };

    let put_response = app
        .oneshot(json_request(
            Method::PUT,
            &format!(
                "/staff/projects/{project_segment}/workflows/{workflow_segment}/orchestration"
            ),
            &request,
        ))
        .await
        .expect("put workflow orchestration");
    assert_eq!(put_response.status(), StatusCode::OK);

    let saved_config = std::fs::read_to_string(&config_path).expect("saved config json");
    assert!(saved_config.contains(workflow_id));
    assert!(
        store
            .load_task_board_staff_project_state(&project_context)
            .expect("load project staff state")
            .is_none()
    );

    let restarted = state_reloaded_from_config(&config_path, workspace_dir).await;
    let app = router().layer(Extension(TenantScope::default())).with_state(restarted);
    let get_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/staff/projects/{project_segment}/workflows/{workflow_segment}/orchestration"
                ))
                .body(Body::empty())
                .expect("get request"),
        )
        .await
        .expect("get workflow orchestration");

    assert_eq!(get_response.status(), StatusCode::OK);
    let loaded: StaffWorkflowOrchestrationResponse = response_json(get_response).await;
    assert_eq!(loaded.workflow_id, workflow_id);
    assert!(loaded.orchestration.capabilities.iter().any(|capability| {
        capability.id == "product_review" && capability.label == "Product Review"
    }));
    assert!(loaded.orchestration.stage_assignments.iter().any(|assignment| {
        assignment.stage_id == "planning"
            && assignment.primary_member_id.as_deref() == Some("staff-planner")
    }));
    assert!(
        loaded.orchestration.stage_actions.iter().any(|action| {
            action.stage_id == "planning" && action.capability_id == "execute_task"
        })
    );

    let capability_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/staff/projects/{project_segment}/workflows/{workflow_segment}/capabilities"
                ))
                .body(Body::empty())
                .expect("get capabilities request"),
        )
        .await
        .expect("get workflow capabilities");
    assert_eq!(capability_response.status(), StatusCode::OK);
    let capabilities: StaffProjectCapabilityCatalogResponse =
        response_json(capability_response).await;
    assert!(capabilities.capabilities.iter().any(|capability| {
        capability.capability_id == "product_review" && !capability.built_in
    }));

    let capability_update = StaffProjectCapabilityCatalogRequest {
        capabilities: vec![StaffProjectCapabilityDto {
            id: "release_workflow".to_string(),
            label: "Release Workflow".to_string(),
            capability_type: StaffCapabilityKindDto::SavedWorkflow,
            built_in: false,
            input_summary: StaffCapabilityIoSummaryDto {
                description: "Workflow UUID and release channel".to_string(),
                fields: vec!["workflow_uuid".to_string(), "release_channel".to_string()],
            },
            result_summary: StaffCapabilityIoSummaryDto {
                description: "Release result".to_string(),
                fields: vec!["status".to_string()],
            },
        }],
    };
    let capability_put_response = app
        .clone()
        .oneshot(json_request(
            Method::PUT,
            &format!("/staff/projects/{project_segment}/workflows/{workflow_segment}/capabilities"),
            &capability_update,
        ))
        .await
        .expect("put workflow capabilities");
    assert_eq!(capability_put_response.status(), StatusCode::OK);

    let get_response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/staff/projects/{project_segment}/workflows/{workflow_segment}/orchestration"
                ))
                .body(Body::empty())
                .expect("get request after capability update"),
        )
        .await
        .expect("get workflow orchestration after capability update");
    assert_eq!(get_response.status(), StatusCode::OK);
    let loaded: StaffWorkflowOrchestrationResponse = response_json(get_response).await;
    assert!(loaded.orchestration.capabilities.iter().any(|capability| {
        capability.id == "release_workflow" && capability.label == "Release Workflow"
    }));
    assert!(loaded.orchestration.stage_assignments.iter().any(|assignment| {
        assignment.stage_id == "planning"
            && assignment.primary_member_id.as_deref() == Some("staff-planner")
    }));
}

#[tokio::test]
async fn staff_projection_shares_workflow_orchestration_across_projects() {
    let temp = tempfile::tempdir().expect("temp project root");
    let workspace_dir = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace_dir).expect("workspace dir");
    let project_a_path = temp.path().join("project-a");
    let project_b_path = temp.path().join("project-b");
    std::fs::create_dir_all(&project_a_path).expect("project a dir");
    std::fs::create_dir_all(&project_b_path).expect("project b dir");
    let (project_a_id, project_a_context) = register_staff_project(&project_a_path);
    let (project_b_id, project_b_context) = register_staff_project(&project_b_path);

    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_a_context).expect("init project a scheduler");
    store.initialize_project(&project_b_context).expect("init project b scheduler");
    store
        .create_task(&project_a_context, projection_task("task-project-a", "planning"))
        .expect("seed project a task");
    store
        .create_task(&project_b_context, projection_task("task-project-b", "planning"))
        .expect("seed project b task");
    store
        .save_task_board_staff_project_state(
            &project_a_context,
            &StaffProjectStateConfig {
                member_task_assignments: vec![StaffMemberTaskAssignmentConfig {
                    member_id: "staff-executor".to_string(),
                    task_ids: vec!["project-a-runtime-task".to_string()],
                }],
                ..StaffProjectStateConfig::default()
            },
        )
        .expect("save project a runtime staff state");
    store
        .save_task_board_staff_project_state(
            &project_b_context,
            &StaffProjectStateConfig {
                member_task_assignments: vec![StaffMemberTaskAssignmentConfig {
                    member_id: "staff-executor".to_string(),
                    task_ids: vec!["project-b-runtime-task".to_string()],
                }],
                ..StaffProjectStateConfig::default()
            },
        )
        .expect("save project b runtime staff state");

    let state = state_with_agents_ipc(false);
    {
        let mut config = state.config.lock();
        config.config_path = temp.path().join(".vibewindow.json");
        config.workspace_dir = workspace_dir;
    }
    let workflow = workflow_snapshot_from_staff_config(&state.config.lock().staff);
    store.save_task_board_workflow(&project_a_context, &workflow).expect("save project a workflow");
    store.save_task_board_workflow(&project_b_context, &workflow).expect("save project b workflow");

    let _saved = handle_api_staff_workflow_orchestration_put(
        State(state.clone()),
        staff_tenant(),
        staff_workflow_path(project_a_id.clone(), workflow.id.clone()),
        Json(StaffWorkflowOrchestrationRequest {
            workflow_id: workflow.id.clone(),
            orchestration: StaffWorkflowOrchestrationDto {
                stage_assignments: vec![StaffStageAssignmentDto {
                    stage_id: "planning".to_string(),
                    primary_member_id: Some("staff-reviewer".to_string()),
                    backup_member_ids: Vec::new(),
                    manager_member_ids: Vec::new(),
                    autonomous: true,
                }],
                ..StaffWorkflowOrchestrationDto::default()
            },
        }),
    )
    .await
    .expect("save workflow orchestration");

    let project_a = handle_api_staff_projection(
        State(state.clone()),
        staff_tenant(),
        staff_project_path(project_a_id),
    )
    .await
    .expect("project a projection")
    .0;
    let project_b =
        handle_api_staff_projection(State(state), staff_tenant(), staff_project_path(project_b_id))
            .await
            .expect("project b projection")
            .0;

    assert_eq!(projection_member_id(&project_a, "task-project-a"), Some("staff-reviewer"));
    assert_eq!(projection_member_id(&project_b, "task-project-b"), Some("staff-reviewer"));
    let project_a_executor = project_a
        .projection
        .member_loads
        .iter()
        .find(|load| load.member_id == "staff-executor")
        .expect("project a executor load");
    let project_b_executor = project_b
        .projection
        .member_loads
        .iter()
        .find(|load| load.member_id == "staff-executor")
        .expect("project b executor load");
    assert_eq!(project_a_executor.task_ids, vec!["project-a-runtime-task".to_string()]);
    assert_eq!(project_b_executor.task_ids, vec!["project-b-runtime-task".to_string()]);
}

#[tokio::test]
async fn staff_projection_includes_custom_capabilities() {
    let temp = tempfile::tempdir().expect("temp project root");
    let project_path = temp.path().join("project-custom-projection");
    std::fs::create_dir_all(&project_path).expect("project dir");
    let (project_id, _project_context) = register_staff_project(&project_path);
    let state = state_with_agents_ipc(false);
    state.config.lock().staff.workflow_orchestrations.insert(
        "dev_lifecycle".to_string(),
        StaffWorkflowOrchestrationConfig {
            capabilities: vec![project_capability_to_config(StaffProjectCapabilityDto {
                id: "release_workflow".to_string(),
                label: "Release Workflow".to_string(),
                capability_type: StaffCapabilityKindDto::SavedWorkflow,
                built_in: false,
                input_summary: StaffCapabilityIoSummaryDto {
                    description: "Workflow UUID and release channel".to_string(),
                    fields: vec!["workflow_uuid".to_string(), "release_channel".to_string()],
                },
                result_summary: StaffCapabilityIoSummaryDto {
                    description: "Release automation result".to_string(),
                    fields: vec!["summary".to_string()],
                },
            })],
            ..StaffWorkflowOrchestrationConfig::default()
        },
    );

    let response =
        handle_api_staff_projection(State(state), staff_tenant(), staff_project_path(project_id))
            .await
            .expect("staff projection")
            .0;

    let ids = response
        .capabilities
        .iter()
        .map(|capability| capability.capability_id.as_str())
        .collect::<Vec<_>>();
    assert!(ids.contains(&"manual"));
    assert!(ids.contains(&"release_workflow"));
    let custom = response
        .capabilities
        .iter()
        .find(|capability| capability.capability_id == "release_workflow")
        .expect("custom capability");
    assert_eq!(custom.label, "Release Workflow");
    assert!(custom.requires_workflow_uuid);
}

#[test]
fn staff_project_state_conversion_preserves_orchestration_and_runtime_assignments() {
    let capability = StaffProjectCapabilityDto {
        id: "release_workflow".to_string(),
        label: "Release Workflow".to_string(),
        capability_type: StaffCapabilityKindDto::SavedWorkflow,
        built_in: false,
        input_summary: StaffCapabilityIoSummaryDto {
            description: "release input".to_string(),
            fields: vec!["workflow_uuid".to_string()],
        },
        result_summary: StaffCapabilityIoSummaryDto {
            description: "release result".to_string(),
            fields: vec!["summary".to_string()],
        },
    };
    let assignment = StaffStageAssignmentDto {
        stage_id: "planning".to_string(),
        primary_member_id: Some("staff-planner".to_string()),
        backup_member_ids: vec!["staff-reviewer".to_string()],
        manager_member_ids: vec!["staff-orchestrator".to_string()],
        autonomous: true,
    };
    let action = StaffStageActionDto {
        stage_id: "planning".to_string(),
        capability_id: "release_workflow".to_string(),
        workflow_uuid: Some("workflow-1".to_string()),
        inputs: StaffStageActionInputsDto {
            workflow_uuid: Some("workflow-2".to_string()),
            parameters: serde_json::Map::from_iter([("channel".to_string(), json!("stable"))]),
        },
    };
    let member_assignment = StaffMemberTaskAssignmentDto {
        member_id: "staff-planner".to_string(),
        task_ids: vec!["task-1".to_string(), "task-2".to_string()],
    };
    let dto = StaffProjectStateDto {
        capabilities: vec![capability.clone()],
        stage_assignments: vec![assignment.clone()],
        stage_workflow_bindings: Vec::new(),
        stage_actions: vec![action.clone()],
        member_task_assignments: vec![member_assignment.clone()],
    };

    let config = project_state_from_dto(dto.clone());
    let workflow = workflow_orchestration_from_project_state(&config);
    assert_eq!(workflow.capabilities.len(), 1);
    assert_eq!(workflow.stage_assignments.len(), 1);
    assert_eq!(workflow.stage_actions.len(), 1);

    let runtime = project_runtime_state(config.clone());
    assert!(runtime.capabilities.is_empty());
    assert!(runtime.stage_assignments.is_empty());
    assert!(runtime.stage_actions.is_empty());
    assert_eq!(runtime.member_task_assignments[0].task_ids, vec!["task-1", "task-2"]);

    assert_eq!(project_state_to_dto(config), dto);

    let workflow_dto = StaffWorkflowOrchestrationDto {
        capabilities: vec![capability],
        stage_assignments: vec![assignment],
        stage_workflow_bindings: Vec::new(),
        stage_actions: vec![action],
    };
    assert_eq!(
        workflow_orchestration_to_dto(workflow_orchestration_from_dto(workflow_dto.clone())),
        workflow_dto
    );
}

#[test]
fn staff_workflow_ids_are_trimmed_and_must_match() {
    assert_eq!(normalized_workflow_id("  dev_lifecycle  ").expect("workflow id"), "dev_lifecycle");
    assert_eq!(
        normalized_matching_workflow_id(" dev_lifecycle ", "dev_lifecycle")
            .expect("matching workflow ids"),
        "dev_lifecycle"
    );

    let empty = normalized_workflow_id(" \t ").expect_err("empty workflow id");
    assert_eq!(empty.status, StatusCode::BAD_REQUEST);
    assert_eq!(empty.message, "workflow_id cannot be empty");

    let mismatch = normalized_matching_workflow_id("dev_lifecycle", "release")
        .expect_err("mismatched workflow ids");
    assert_eq!(mismatch.status, StatusCode::BAD_REQUEST);
    assert!(mismatch.message.contains("path dev_lifecycle"));
    assert!(mismatch.message.contains("body release"));
}

#[test]
fn staff_status_and_metadata_helpers_cover_known_and_invalid_values() {
    let mappings = [
        (AgentTaskStatus::Draft, AgentTaskStatusDto::Draft),
        (AgentTaskStatus::Queued, AgentTaskStatusDto::Queued),
        (AgentTaskStatus::Assigned, AgentTaskStatusDto::Assigned),
        (AgentTaskStatus::Dispatched, AgentTaskStatusDto::Dispatched),
        (AgentTaskStatus::Acknowledged, AgentTaskStatusDto::Acknowledged),
        (AgentTaskStatus::Active, AgentTaskStatusDto::Active),
        (AgentTaskStatus::Running, AgentTaskStatusDto::Running),
        (AgentTaskStatus::Submitted, AgentTaskStatusDto::Submitted),
        (AgentTaskStatus::Revision, AgentTaskStatusDto::Revision),
        (AgentTaskStatus::Blocked, AgentTaskStatusDto::Blocked),
        (AgentTaskStatus::Paused, AgentTaskStatusDto::Paused),
        (AgentTaskStatus::Completed, AgentTaskStatusDto::Completed),
        (AgentTaskStatus::Failed, AgentTaskStatusDto::Failed),
        (AgentTaskStatus::Cancelled, AgentTaskStatusDto::Cancelled),
        (AgentTaskStatus::Archived, AgentTaskStatusDto::Archived),
    ];
    for (status, expected) in mappings {
        assert_eq!(task_status_to_dto(status), expected);
    }

    for status in [
        AgentTaskStatusDto::Assigned,
        AgentTaskStatusDto::Dispatched,
        AgentTaskStatusDto::Acknowledged,
        AgentTaskStatusDto::Active,
        AgentTaskStatusDto::Running,
    ] {
        assert!(agent_task_status_is_running(&status));
    }
    assert!(!agent_task_status_is_running(&AgentTaskStatusDto::Queued));
    assert!(!agent_task_status_is_running(&AgentTaskStatusDto::Completed));

    let roles = [
        ("backlog", TaskPoolWorkflowStatusRoleDto::Backlog),
        ("queued", TaskPoolWorkflowStatusRoleDto::Queued),
        ("planning", TaskPoolWorkflowStatusRoleDto::Planning),
        ("executing", TaskPoolWorkflowStatusRoleDto::Executing),
        ("failed", TaskPoolWorkflowStatusRoleDto::Failed),
        ("paused", TaskPoolWorkflowStatusRoleDto::Paused),
        ("review_ready", TaskPoolWorkflowStatusRoleDto::ReviewReady),
        ("review", TaskPoolWorkflowStatusRoleDto::Review),
        ("merging", TaskPoolWorkflowStatusRoleDto::Merging),
        ("done", TaskPoolWorkflowStatusRoleDto::Done),
        ("archived", TaskPoolWorkflowStatusRoleDto::Archived),
        ("custom", TaskPoolWorkflowStatusRoleDto::Custom),
    ];
    for (role, expected) in roles {
        assert_eq!(
            task_board_status_role_from_metadata(&json!({"task_board.status_role": role})),
            Some(expected)
        );
    }
    assert_eq!(
        task_board_status_role_from_metadata(&json!({"task_board.status_role": "unknown"})),
        None
    );

    let metadata = json!({
        "label": "  Ready  ",
        "blank": "  ",
        "priority": u32::MAX as u64,
        "overflow": u32::MAX as u64 + 1,
        "order": -7
    });
    assert_eq!(string_field(&metadata, "label").as_deref(), Some("Ready"));
    assert_eq!(string_field(&metadata, "blank"), None);
    assert_eq!(u32_field(&metadata, "priority"), Some(u32::MAX));
    assert_eq!(u32_field(&metadata, "overflow"), None);
    assert_eq!(i64_field(&metadata, "order"), Some(-7));

    let mut values = vec!["task-1".to_string()];
    push_unique(&mut values, "task-1".to_string());
    push_unique(&mut values, "task-2".to_string());
    assert_eq!(values, vec!["task-1", "task-2"]);
}

async fn state_reloaded_from_config(
    config_path: &std::path::Path,
    workspace_dir: std::path::PathBuf,
) -> AppState {
    let config = crate::app::agent::config::load_from_path_without_env(config_path, workspace_dir)
        .await
        .expect("reload config");
    let state = state_with_agents_ipc(false);
    *state.config.lock() = config;
    state
}

fn json_request<T: Serialize>(method: Method, uri: &str, body: &T) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(body).expect("request json")))
        .expect("request")
}

async fn response_json<T: serde::de::DeserializeOwned>(response: axum::response::Response) -> T {
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("response body");
    serde_json::from_slice(&body).expect("response json")
}

fn project_capability_to_config(
    capability: StaffProjectCapabilityDto,
) -> vw_config_types::staff::StaffCapabilityConfig {
    vw_config_types::staff::StaffCapabilityConfig {
        id: capability.id,
        label: capability.label,
        capability_type: match capability.capability_type {
            StaffCapabilityKindDto::BuiltIn => {
                vw_config_types::staff::StaffCapabilityKindConfig::BuiltIn
            }
            StaffCapabilityKindDto::SavedWorkflow => {
                vw_config_types::staff::StaffCapabilityKindConfig::SavedWorkflow
            }
        },
        built_in: capability.built_in,
        input_summary: vw_config_types::staff::StaffCapabilityIoSummaryConfig {
            description: capability.input_summary.description,
            fields: capability.input_summary.fields,
        },
        result_summary: vw_config_types::staff::StaffCapabilityIoSummaryConfig {
            description: capability.result_summary.description,
            fields: capability.result_summary.fields,
        },
    }
}

fn projection_task(id: &str, status_key: &str) -> NewAgentTask {
    NewAgentTask {
        id: TaskId::new(id).expect("task id"),
        title: format!("Projection task {id}"),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Queued,
        spec_uri: format!("shared/tasks/{id}/spec.md"),
        meta_uri: format!("shared/tasks/{id}/meta.json"),
        assigned_to: None,
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({
            "task_board.status_key": status_key,
            "priority": 1,
            "order_no": 0
        }),
        created_at_ms: TimestampMs(1_783_000_000_000),
    }
}

fn projection_member_id<'a>(
    response: &'a StaffProjectionResponse,
    task_id: &str,
) -> Option<&'a str> {
    response
        .projection
        .lanes
        .iter()
        .flat_map(|lane| lane.assignments.iter())
        .find(|assignment| assignment.task_id == task_id)
        .and_then(|assignment| assignment.member_id.as_deref())
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

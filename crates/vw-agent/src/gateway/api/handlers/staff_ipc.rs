//! 员工任务流本地 IPC 消息路由模块。

use axum::extract::{Extension, Path as AxumPath, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::Value;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::agent_scheduler::{AgentSchedulerCommandResponse, AgentTaskStatusDto};
use vw_api_types::common::TimestampMs;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_api_types::staff::{
    StaffAutoSchedulePlanRequest, StaffAutoSchedulePlanResponse, StaffCapabilityKindDto,
    StaffCommandRequest, StaffCommandResponse,
    StaffMemberTaskAssignmentDto, StaffProjectCapabilityCatalogRequest,
    StaffProjectCapabilityCatalogResponse, StaffProjectCapabilityDto,
    StaffProjectOrchestrationRequest, StaffProjectOrchestrationResponse, StaffProjectStateDto,
    StaffProjectionResponse, StaffStageActionDto, StaffStageActionInputsDto,
    StaffStageAssignmentDto, StaffStageCapabilitiesResponse, StaffWorkflowOrchestrationDto,
    StaffWorkflowOrchestrationRequest, StaffWorkflowOrchestrationResponse,
};
use vw_api_types::task::{TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusRoleDto};
use vw_config_types::staff::{
    StaffCapabilityConfig, StaffCapabilityIoSummaryConfig, StaffCapabilityKindConfig, StaffConfig,
    StaffMemberTaskAssignmentConfig, StaffProjectStateConfig, StaffStageActionConfig,
    StaffStageActionInputsConfig, StaffStageAssignmentConfig, StaffWorkflowOrchestrationConfig,
};

use crate::agent_scheduler::domain::AgentSchedulerContext;
use crate::agent_scheduler::staff::{
    StaffTaskReadModel, build_staff_auto_schedule_plan, build_staff_projection,
    scheduler_command_for_action, staff_config_for_workflow_orchestration, stage_capabilities,
    stage_capabilities_for_config, validate_staff_command, workflow_snapshot_from_staff_config,
};
use crate::agent_scheduler::state_machine::AgentTaskStatus;
use crate::agent_scheduler::store::tasks::{AgentTaskFilter, StoredAgentTask};
use crate::agent_scheduler::store::{AgentSchedulerStorageContext, AgentSchedulerStore};
use crate::agent_scheduler::task_board::task_board_status_key;
use crate::app::agent::config::schema::save_config;
use crate::app::agent::gateway::{ApiError, AppState};
use crate::gateway::api::handlers::agent_scheduler::{
    AgentSchedulerGateway, StoreBackedAgentSchedulerGateway,
};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/staff/stage-capabilities", get(handle_api_staff_stage_capabilities))
        .route(
            "/staff/projects/{project_id}/capabilities",
            get(handle_api_staff_project_capabilities)
                .put(handle_api_staff_project_capabilities_put),
        )
        .route(
            "/staff/projects/{project_id}/orchestration",
            get(handle_api_staff_project_orchestration)
                .put(handle_api_staff_project_orchestration_put),
        )
        .route(
            "/staff/projects/{project_id}/workflows/{workflow_id}/capabilities",
            get(handle_api_staff_workflow_capabilities)
                .put(handle_api_staff_workflow_capabilities_put),
        )
        .route(
            "/staff/projects/{project_id}/workflows/{workflow_id}/orchestration",
            get(handle_api_staff_workflow_orchestration)
                .put(handle_api_staff_workflow_orchestration_put),
        )
        .route("/staff/projects/{project_id}/projection", get(handle_api_staff_projection))
        .route("/staff/projects/{project_id}/commands", post(handle_api_staff_command))
        .route(
            "/staff/projects/{project_id}/auto-schedule-plan",
            post(handle_api_staff_auto_schedule_plan),
        )
}

pub async fn handle_api_staff_stage_capabilities(
    Extension(_tenant): Extension<TenantScope>,
) -> Result<Json<StaffStageCapabilitiesResponse>, ApiError> {
    Ok(Json(StaffStageCapabilitiesResponse { capabilities: stage_capabilities() }))
}

pub async fn handle_api_staff_project_capabilities(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    AxumPath(project_id): AxumPath<String>,
) -> Result<Json<StaffProjectCapabilityCatalogResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    let store = AgentSchedulerStore::new();
    let (staff_config, _) = load_staff_orchestration_config(&state, &store, &project_path)?;
    Ok(Json(project_capability_catalog_response(&staff_config)))
}

pub async fn handle_api_staff_project_capabilities_put(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    AxumPath(project_id): AxumPath<String>,
    Json(body): Json<StaffProjectCapabilityCatalogRequest>,
) -> Result<Json<StaffProjectCapabilityCatalogResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path).map_err(map_scheduler_error)?;

    let mut staff_config = state.config.lock().staff.clone();
    let workflow = store
        .load_task_board_workflow(&project_path)
        .map_err(map_scheduler_error)?
        .unwrap_or_else(|| workflow_snapshot_from_staff_config(&staff_config));
    let workflow_id = normalized_workflow_id(&workflow.id)?;
    let project_state = store
        .load_task_board_staff_project_state(&project_path)
        .map_err(map_scheduler_error)?
        .map(project_runtime_state)
        .unwrap_or_default();
    let mut orchestration =
        staff_config.workflow_orchestrations.remove(&workflow_id).unwrap_or_default();
    orchestration.capabilities =
        body.capabilities.into_iter().map(project_capability_from_dto).collect();
    staff_config.workflow_orchestrations.insert(workflow_id.clone(), orchestration);
    staff_config.project_state = project_state;
    staff_config = staff_config_for_workflow_orchestration(&staff_config, &workflow_id, &workflow)
        .normalize()
        .map_err(ApiError::bad_request)?;
    save_staff_project_orchestration_snapshot(
        &state,
        &store,
        &project_path,
        &workflow,
        &staff_config,
    )
    .await?;
    let response = project_capability_catalog_response(&staff_config);
    Ok(Json(response))
}

pub async fn handle_api_staff_project_orchestration(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    AxumPath(project_id): AxumPath<String>,
) -> Result<Json<StaffProjectOrchestrationResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    let store = AgentSchedulerStore::new();
    let (staff_config, workflow) = load_staff_orchestration_config(&state, &store, &project_path)?;
    Ok(Json(project_orchestration_response(project_id, workflow, staff_config.project_state)))
}

pub async fn handle_api_staff_project_orchestration_put(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    AxumPath(project_id): AxumPath<String>,
    Json(body): Json<StaffProjectOrchestrationRequest>,
) -> Result<Json<StaffProjectOrchestrationResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(&project_path).map_err(map_scheduler_error)?;

    let workflow_id = normalized_workflow_id(&body.workflow.id)?;
    let project_state = project_state_from_dto(body.project_state);
    let mut staff_config = state.config.lock().staff.clone();
    staff_config
        .workflow_orchestrations
        .insert(workflow_id.clone(), workflow_orchestration_from_project_state(&project_state));
    staff_config.project_state = project_runtime_state(project_state);
    staff_config =
        staff_config_for_workflow_orchestration(&staff_config, &workflow_id, &body.workflow)
            .normalize()
            .map_err(ApiError::bad_request)?;
    save_staff_project_orchestration_snapshot(
        &state,
        &store,
        &project_path,
        &body.workflow,
        &staff_config,
    )
    .await?;

    Ok(Json(project_orchestration_response(project_id, body.workflow, staff_config.project_state)))
}

pub async fn handle_api_staff_workflow_capabilities(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    AxumPath((project_id, workflow_id)): AxumPath<(String, String)>,
) -> Result<Json<StaffProjectCapabilityCatalogResponse>, ApiError> {
    let _context = resolve_project_context(tenant, &project_id).await?;
    let workflow_id = normalized_workflow_id(&workflow_id)?;
    let staff_config = state.config.lock().staff.clone();
    let staff_config = staff_workflow_orchestration_view(&staff_config, &workflow_id)?;
    Ok(Json(project_capability_catalog_response(&staff_config)))
}

pub async fn handle_api_staff_workflow_capabilities_put(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    AxumPath((project_id, workflow_id)): AxumPath<(String, String)>,
    Json(body): Json<StaffProjectCapabilityCatalogRequest>,
) -> Result<Json<StaffProjectCapabilityCatalogResponse>, ApiError> {
    let _context = resolve_project_context(tenant, &project_id).await?;
    let workflow_id = normalized_workflow_id(&workflow_id)?;
    let mut staff_config = state.config.lock().staff.clone();
    let mut orchestration =
        staff_config.workflow_orchestrations.remove(&workflow_id).unwrap_or_default();
    orchestration.capabilities =
        body.capabilities.into_iter().map(project_capability_from_dto).collect();
    staff_config.workflow_orchestrations.insert(workflow_id.clone(), orchestration);
    let staff_config =
        save_staff_workflow_orchestration_config(&state, staff_config, &workflow_id).await?;
    let staff_config = staff_workflow_orchestration_view(&staff_config, &workflow_id)?;
    Ok(Json(project_capability_catalog_response(&staff_config)))
}

pub async fn handle_api_staff_workflow_orchestration(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    AxumPath((project_id, workflow_id)): AxumPath<(String, String)>,
) -> Result<Json<StaffWorkflowOrchestrationResponse>, ApiError> {
    let _context = resolve_project_context(tenant, &project_id).await?;
    let workflow_id = normalized_workflow_id(&workflow_id)?;
    let staff_config = state.config.lock().staff.clone();
    Ok(Json(workflow_orchestration_response(&staff_config, workflow_id)))
}

pub async fn handle_api_staff_workflow_orchestration_put(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    AxumPath((project_id, workflow_id)): AxumPath<(String, String)>,
    Json(body): Json<StaffWorkflowOrchestrationRequest>,
) -> Result<Json<StaffWorkflowOrchestrationResponse>, ApiError> {
    let _context = resolve_project_context(tenant, &project_id).await?;
    let workflow_id = normalized_matching_workflow_id(&workflow_id, &body.workflow_id)?;
    let mut staff_config = state.config.lock().staff.clone();
    staff_config
        .workflow_orchestrations
        .insert(workflow_id.clone(), workflow_orchestration_from_dto(body.orchestration));
    let staff_config =
        save_staff_workflow_orchestration_config(&state, staff_config, &workflow_id).await?;
    Ok(Json(workflow_orchestration_response(&staff_config, workflow_id)))
}

pub async fn handle_api_staff_projection(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    AxumPath(project_id): AxumPath<String>,
) -> Result<Json<StaffProjectionResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    let store = AgentSchedulerStore::new();
    let (staff_config, workflow, tasks, running_task_ids) =
        load_staff_inputs(&state, &store, &project_path)?;
    let projection = build_staff_projection(&tasks, &staff_config, &workflow, &running_task_ids)
        .map_err(ApiError::bad_request)?;
    Ok(Json(StaffProjectionResponse {
        project_id: project_id.into(),
        projection,
        capabilities: stage_capabilities_for_config(&staff_config),
    }))
}

pub async fn handle_api_staff_command(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    AxumPath(project_id): AxumPath<String>,
    Json(body): Json<StaffCommandRequest>,
) -> Result<Json<StaffCommandResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    let store = AgentSchedulerStore::new();
    let (staff_config, workflow, tasks, _) = load_staff_inputs(&state, &store, &project_path)?;
    let action = validate_staff_command(&body.command, &tasks, &staff_config, &workflow)
        .map_err(ApiError::bad_request)?;
    let scheduler_command =
        scheduler_command_for_action(&action, &body.command, Some("staff command".to_string()));
    let scheduler_result = if body.dry_run {
        None
    } else {
        execute_scheduler_command(&project_id, project_path, scheduler_command.clone()).await?
    };
    Ok(Json(StaffCommandResponse {
        accepted: true,
        message: if body.dry_run {
            "staff command validated".to_string()
        } else {
            "staff command accepted".to_string()
        },
        action: Some(action),
        scheduler_command,
        scheduler_result,
    }))
}

pub async fn handle_api_staff_auto_schedule_plan(
    State(state): State<AppState>,
    Extension(tenant): Extension<TenantScope>,
    AxumPath(project_id): AxumPath<String>,
    Json(body): Json<StaffAutoSchedulePlanRequest>,
) -> Result<Json<StaffAutoSchedulePlanResponse>, ApiError> {
    let project_path = resolve_project_context(tenant, &project_id).await?;
    let store = AgentSchedulerStore::new();
    let (staff_config, workflow, tasks, mut running_task_ids) =
        load_staff_inputs(&state, &store, &project_path)?;
    for task_id in body.running_task_ids {
        push_unique(&mut running_task_ids, task_id);
    }
    let projection = build_staff_projection(&tasks, &staff_config, &workflow, &running_task_ids)
        .map_err(ApiError::bad_request)?;
    let plan = build_staff_auto_schedule_plan(
        &tasks,
        &staff_config,
        &projection,
        &workflow,
        &running_task_ids,
        body.now_ms.unwrap_or_else(current_timestamp_ms),
    )
    .map_err(ApiError::bad_request)?;
    Ok(Json(StaffAutoSchedulePlanResponse { project_id: project_id.into(), plan }))
}

fn project_capability_catalog_response(
    staff_config: &StaffConfig,
) -> StaffProjectCapabilityCatalogResponse {
    StaffProjectCapabilityCatalogResponse {
        capabilities: stage_capabilities_for_config(staff_config),
    }
}

fn project_capability_from_dto(capability: StaffProjectCapabilityDto) -> StaffCapabilityConfig {
    StaffCapabilityConfig {
        id: capability.id,
        label: capability.label,
        capability_type: capability_kind_from_dto(capability.capability_type),
        built_in: capability.built_in,
        input_summary: StaffCapabilityIoSummaryConfig {
            description: capability.input_summary.description,
            fields: capability.input_summary.fields,
        },
        result_summary: StaffCapabilityIoSummaryConfig {
            description: capability.result_summary.description,
            fields: capability.result_summary.fields,
        },
    }
}

fn project_capability_to_dto(capability: StaffCapabilityConfig) -> StaffProjectCapabilityDto {
    StaffProjectCapabilityDto {
        id: capability.id,
        label: capability.label,
        capability_type: capability_kind_to_dto(capability.capability_type),
        built_in: capability.built_in,
        input_summary: vw_api_types::staff::StaffCapabilityIoSummaryDto {
            description: capability.input_summary.description,
            fields: capability.input_summary.fields,
        },
        result_summary: vw_api_types::staff::StaffCapabilityIoSummaryDto {
            description: capability.result_summary.description,
            fields: capability.result_summary.fields,
        },
    }
}

fn capability_kind_from_dto(kind: StaffCapabilityKindDto) -> StaffCapabilityKindConfig {
    match kind {
        StaffCapabilityKindDto::BuiltIn => StaffCapabilityKindConfig::BuiltIn,
        StaffCapabilityKindDto::SavedWorkflow => StaffCapabilityKindConfig::SavedWorkflow,
    }
}

fn capability_kind_to_dto(kind: StaffCapabilityKindConfig) -> StaffCapabilityKindDto {
    match kind {
        StaffCapabilityKindConfig::BuiltIn => StaffCapabilityKindDto::BuiltIn,
        StaffCapabilityKindConfig::SavedWorkflow => StaffCapabilityKindDto::SavedWorkflow,
    }
}

fn project_state_from_dto(project_state: StaffProjectStateDto) -> StaffProjectStateConfig {
    StaffProjectStateConfig {
        capabilities: project_state
            .capabilities
            .into_iter()
            .map(project_capability_from_dto)
            .collect(),
        stage_assignments: project_state
            .stage_assignments
            .into_iter()
            .map(|assignment| StaffStageAssignmentConfig {
                stage_id: assignment.stage_id,
                primary_member_id: assignment.primary_member_id,
                backup_member_ids: assignment.backup_member_ids,
                manager_member_ids: assignment.manager_member_ids,
                autonomous: assignment.autonomous,
            })
            .collect(),
        stage_workflow_bindings: Vec::new(),
        stage_actions: project_state
            .stage_actions
            .into_iter()
            .map(|action| StaffStageActionConfig {
                stage_id: action.stage_id,
                capability_id: action.capability_id,
                workflow_uuid: action.workflow_uuid,
                inputs: StaffStageActionInputsConfig {
                    workflow_uuid: action.inputs.workflow_uuid,
                    parameters: action.inputs.parameters.into_iter().collect(),
                },
            })
            .collect(),
        member_task_assignments: project_state
            .member_task_assignments
            .into_iter()
            .map(|assignment| StaffMemberTaskAssignmentConfig {
                member_id: assignment.member_id,
                task_ids: assignment.task_ids,
            })
            .collect(),
    }
}

fn project_state_to_dto(project_state: StaffProjectStateConfig) -> StaffProjectStateDto {
    StaffProjectStateDto {
        capabilities: project_state
            .capabilities
            .into_iter()
            .map(project_capability_to_dto)
            .collect(),
        stage_assignments: project_state
            .stage_assignments
            .into_iter()
            .map(|assignment| StaffStageAssignmentDto {
                stage_id: assignment.stage_id,
                primary_member_id: assignment.primary_member_id,
                backup_member_ids: assignment.backup_member_ids,
                manager_member_ids: assignment.manager_member_ids,
                autonomous: assignment.autonomous,
            })
            .collect(),
        stage_workflow_bindings: Vec::new(),
        stage_actions: project_state
            .stage_actions
            .into_iter()
            .map(|action| StaffStageActionDto {
                stage_id: action.stage_id,
                capability_id: action.capability_id,
                workflow_uuid: action.workflow_uuid,
                inputs: StaffStageActionInputsDto {
                    workflow_uuid: action.inputs.workflow_uuid,
                    parameters: action.inputs.parameters.into_iter().collect(),
                },
            })
            .collect(),
        member_task_assignments: project_state
            .member_task_assignments
            .into_iter()
            .map(|assignment| StaffMemberTaskAssignmentDto {
                member_id: assignment.member_id,
                task_ids: assignment.task_ids,
            })
            .collect(),
    }
}

fn workflow_orchestration_from_dto(
    orchestration: StaffWorkflowOrchestrationDto,
) -> StaffWorkflowOrchestrationConfig {
    StaffWorkflowOrchestrationConfig {
        capabilities: orchestration
            .capabilities
            .into_iter()
            .map(project_capability_from_dto)
            .collect(),
        stage_assignments: orchestration
            .stage_assignments
            .into_iter()
            .map(|assignment| StaffStageAssignmentConfig {
                stage_id: assignment.stage_id,
                primary_member_id: assignment.primary_member_id,
                backup_member_ids: assignment.backup_member_ids,
                manager_member_ids: assignment.manager_member_ids,
                autonomous: assignment.autonomous,
            })
            .collect(),
        stage_workflow_bindings: Vec::new(),
        stage_actions: orchestration
            .stage_actions
            .into_iter()
            .map(|action| StaffStageActionConfig {
                stage_id: action.stage_id,
                capability_id: action.capability_id,
                workflow_uuid: action.workflow_uuid,
                inputs: StaffStageActionInputsConfig {
                    workflow_uuid: action.inputs.workflow_uuid,
                    parameters: action.inputs.parameters.into_iter().collect(),
                },
            })
            .collect(),
    }
}

fn workflow_orchestration_from_project_state(
    project_state: &StaffProjectStateConfig,
) -> StaffWorkflowOrchestrationConfig {
    StaffWorkflowOrchestrationConfig {
        capabilities: project_state.capabilities.clone(),
        stage_assignments: project_state.stage_assignments.clone(),
        stage_workflow_bindings: Vec::new(),
        stage_actions: project_state.stage_actions.clone(),
    }
}

fn project_runtime_state(project_state: StaffProjectStateConfig) -> StaffProjectStateConfig {
    StaffProjectStateConfig {
        member_task_assignments: project_state.member_task_assignments,
        ..StaffProjectStateConfig::default()
    }
}

fn workflow_orchestration_to_dto(
    orchestration: StaffWorkflowOrchestrationConfig,
) -> StaffWorkflowOrchestrationDto {
    StaffWorkflowOrchestrationDto {
        capabilities: orchestration
            .capabilities
            .into_iter()
            .map(project_capability_to_dto)
            .collect(),
        stage_assignments: orchestration
            .stage_assignments
            .into_iter()
            .map(|assignment| StaffStageAssignmentDto {
                stage_id: assignment.stage_id,
                primary_member_id: assignment.primary_member_id,
                backup_member_ids: assignment.backup_member_ids,
                manager_member_ids: assignment.manager_member_ids,
                autonomous: assignment.autonomous,
            })
            .collect(),
        stage_workflow_bindings: Vec::new(),
        stage_actions: orchestration
            .stage_actions
            .into_iter()
            .map(|action| StaffStageActionDto {
                stage_id: action.stage_id,
                capability_id: action.capability_id,
                workflow_uuid: action.workflow_uuid,
                inputs: StaffStageActionInputsDto {
                    workflow_uuid: action.inputs.workflow_uuid,
                    parameters: action.inputs.parameters.into_iter().collect(),
                },
            })
            .collect(),
    }
}

fn project_orchestration_response(
    project_id: String,
    workflow: TaskPoolWorkflowSnapshotDto,
    project_state: StaffProjectStateConfig,
) -> StaffProjectOrchestrationResponse {
    StaffProjectOrchestrationResponse {
        project_id: project_id.into(),
        workflow,
        project_state: project_state_to_dto(project_state),
    }
}

fn workflow_orchestration_response(
    staff_config: &StaffConfig,
    workflow_id: String,
) -> StaffWorkflowOrchestrationResponse {
    let orchestration =
        staff_config.workflow_orchestrations.get(&workflow_id).cloned().unwrap_or_default();
    StaffWorkflowOrchestrationResponse {
        workflow_id,
        orchestration: workflow_orchestration_to_dto(orchestration),
    }
}

async fn save_staff_project_orchestration_snapshot(
    state: &AppState,
    store: &AgentSchedulerStore,
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
    workflow: &TaskPoolWorkflowSnapshotDto,
    staff_config: &StaffConfig,
) -> Result<(), ApiError> {
    store.save_task_board_workflow(project_path, workflow).map_err(map_scheduler_error)?;
    store
        .save_task_board_staff_project_state(project_path, &staff_config.project_state)
        .map_err(map_scheduler_error)?;

    let mut updated = state.config.lock().clone();
    updated.staff = staff_config.clone();
    updated.staff.project_state = StaffProjectStateConfig::default();
    updated.staff = updated.staff.normalize().map_err(ApiError::bad_request)?;
    save_config(&updated).await.map_err(|error| {
        ApiError::internal(format!("failed to save staff project orchestration: {error}"))
    })?;
    *state.config.lock() = updated;
    Ok(())
}

async fn save_staff_workflow_orchestration_config(
    state: &AppState,
    staff_config: StaffConfig,
    workflow_id: &str,
) -> Result<StaffConfig, ApiError> {
    let mut updated = state.config.lock().clone();
    updated.staff = staff_config.normalize().map_err(ApiError::bad_request)?;
    let workflow = workflow_snapshot_from_staff_config(&updated.staff);
    staff_config_for_workflow_orchestration(&updated.staff, workflow_id, &workflow)
        .normalize()
        .map_err(ApiError::bad_request)?;
    save_config(&updated).await.map_err(|error| {
        ApiError::internal(format!("failed to save staff workflow orchestration: {error}"))
    })?;
    *state.config.lock() = updated.clone();
    Ok(updated.staff)
}

fn staff_workflow_orchestration_view(
    staff_config: &StaffConfig,
    workflow_id: &str,
) -> Result<StaffConfig, ApiError> {
    let workflow = workflow_snapshot_from_staff_config(staff_config);
    staff_config_for_workflow_orchestration(staff_config, workflow_id, &workflow)
        .normalize()
        .map_err(ApiError::bad_request)
}

fn normalized_matching_workflow_id(
    path_workflow_id: &str,
    body_workflow_id: &str,
) -> Result<String, ApiError> {
    let workflow_id = normalized_workflow_id(path_workflow_id)?;
    let body_workflow_id = normalized_workflow_id(body_workflow_id)?;
    if body_workflow_id != workflow_id {
        return Err(ApiError::bad_request(format!(
            "workflow_id mismatch: path {workflow_id} does not match body {body_workflow_id}"
        )));
    }
    Ok(workflow_id)
}

fn normalized_workflow_id(workflow_id: &str) -> Result<String, ApiError> {
    non_empty_str(workflow_id)
        .map(str::to_string)
        .ok_or_else(|| ApiError::bad_request("workflow_id cannot be empty"))
}

async fn execute_scheduler_command(
    project_id: &str,
    project_path: AgentSchedulerContext,
    command: Option<vw_api_types::agent_scheduler::AgentSchedulerCommandRequest>,
) -> Result<Option<AgentSchedulerCommandResponse>, ApiError> {
    let Some(command) = command else {
        return Ok(None);
    };
    let gateway = StoreBackedAgentSchedulerGateway::default();
    gateway.command(project_id.to_string(), project_path, command).await.map(Some)
}

fn load_staff_inputs(
    state: &AppState,
    store: &AgentSchedulerStore,
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
) -> Result<
    (
        vw_config_types::staff::StaffConfig,
        TaskPoolWorkflowSnapshotDto,
        Vec<StaffTaskReadModel>,
        Vec<String>,
    ),
    ApiError,
> {
    let (staff_config, workflow) = load_staff_orchestration_config(state, store, project_path)?;
    let tasks = store
        .list_task_board_tasks(project_path, AgentTaskFilter::default())
        .map_err(map_scheduler_error)?;
    let tasks =
        tasks.into_iter().map(|task| staff_task_read_model(task, &workflow)).collect::<Vec<_>>();
    let running_task_ids = tasks
        .iter()
        .filter(|task| task.agent_status.as_ref().is_some_and(agent_task_status_is_running))
        .map(|task| task.task_id.clone())
        .collect();
    Ok((staff_config, workflow, tasks, running_task_ids))
}

fn load_staff_orchestration_config(
    state: &AppState,
    store: &AgentSchedulerStore,
    project_path: &(impl AgentSchedulerStorageContext + ?Sized),
) -> Result<(StaffConfig, TaskPoolWorkflowSnapshotDto), ApiError> {
    store.initialize_project(project_path).map_err(map_scheduler_error)?;
    let mut staff_config = state.config.lock().staff.clone();
    let workflow = store
        .load_task_board_workflow(project_path)
        .map_err(map_scheduler_error)?
        .unwrap_or_else(|| workflow_snapshot_from_staff_config(&staff_config));
    let workflow_id = normalized_workflow_id(&workflow.id)?;
    staff_config.project_state = store
        .load_task_board_staff_project_state(project_path)
        .map_err(map_scheduler_error)?
        .map(project_runtime_state)
        .unwrap_or_default();
    let staff_config =
        staff_config_for_workflow_orchestration(&staff_config, &workflow_id, &workflow)
            .normalize()
            .map_err(ApiError::bad_request)?;
    Ok((staff_config, workflow))
}

fn staff_task_read_model(
    task: StoredAgentTask,
    workflow: &TaskPoolWorkflowSnapshotDto,
) -> StaffTaskReadModel {
    let status_key = task_board_status_key(&task);
    let status = workflow.statuses.iter().find(|status| status.key == status_key);
    let metadata = &task.metadata;
    StaffTaskReadModel {
        task_id: task.id.as_str().to_string(),
        title: task.title,
        status_key,
        status_label: string_field(metadata, "task_board.status_label")
            .or_else(|| status.map(|status| status.label.clone())),
        status_role: task_board_status_role_from_metadata(metadata)
            .or_else(|| status.map(|status| status.role)),
        agent_status: Some(task_status_to_dto(task.status)),
        priority: u32_field(metadata, "priority").unwrap_or_default(),
        order_no: i64_field(metadata, "order_no").unwrap_or_default(),
        updated_at_ms: task.updated_at_ms,
        last_error: string_field(metadata, "last_error"),
        pause_reason: string_field(metadata, "pause_reason"),
        staff_member_id: string_field(metadata, "task_board.staff.member_id"),
        assignee: task.assigned_to.as_ref().map(|worker| worker.as_str().to_string()),
        acp_agent: string_field(metadata, "acp_agent"),
    }
}

async fn resolve_project_context(
    tenant: TenantScope,
    project_id: &str,
) -> Result<AgentSchedulerContext, ApiError> {
    #[cfg(test)]
    if std::path::Path::new(project_id).is_dir() {
        use crate::agent_scheduler::store::AgentSchedulerStorageContext;

        let directory = std::path::PathBuf::from(project_id);
        let scope = directory.storage_scope().map_err(map_scheduler_error)?;
        return AgentSchedulerContext::new(scope, directory).map_err(map_scheduler_error);
    }

    let requested_scope = ProjectScope::new(tenant.clone(), ProjectId::from(project_id))
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let stored = crate::app::agent::project::get(&requested_scope)
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let project = match stored {
        Some(project) => project,
        None => {
            let directory = std::path::Path::new(project_id);
            if !directory.is_dir() {
                return Err(ApiError::not_found("project not found"));
            }
            crate::app::agent::project::from_directory(&tenant, directory)
                .await
                .map(|(project, _)| project)
                .map_err(|error| ApiError::bad_request(error.to_string()))?
        }
    };
    let project_scope = ProjectScope::new(tenant, ProjectId::from(project.id.clone()))
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    AgentSchedulerContext::new(AgentSchedulerScope::new(project_scope), project.worktree)
        .map_err(map_scheduler_error)
}

fn current_timestamp_ms() -> TimestampMs {
    TimestampMs(chrono::Utc::now().timestamp_millis().max(0))
}

fn map_scheduler_error(error: anyhow::Error) -> ApiError {
    ApiError::internal(format!("agent scheduler staff gateway error: {error}"))
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

fn agent_task_status_is_running(status: &AgentTaskStatusDto) -> bool {
    matches!(
        status,
        AgentTaskStatusDto::Assigned
            | AgentTaskStatusDto::Dispatched
            | AgentTaskStatusDto::Acknowledged
            | AgentTaskStatusDto::Active
            | AgentTaskStatusDto::Running
    )
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

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).and_then(non_empty_str).map(str::to_string)
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

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

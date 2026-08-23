//! 将员工与工作流配置编译为调度读模型，并驱动阶段分派和自动调度决策。

use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};
use vw_api_types::agent_scheduler::{
    AgentSchedulerCommandKindDto, AgentSchedulerCommandPayloadDto, AgentSchedulerCommandRequest,
    AgentSchedulerDispatchModeDto, AgentSchedulerMoveTaskPayloadDto,
    AgentSchedulerStaffCommandPayloadDto, AgentSchedulerStageActionCommandPayloadDto,
    AgentSchedulerStageActionInputsDto, AgentSchedulerStageActionOutcomeDto, AgentTaskStatusDto,
};
use vw_api_types::common::TimestampMs;
use vw_api_types::staff::{
    StaffActionAvailabilityDto, StaffAssignmentDto, StaffAutoScheduleCommandContextDto,
    StaffAutoSchedulePlanDto, StaffAutoScheduleSkipDto, StaffCommandActionDto, StaffCommandDto,
    StaffLaneDto, StaffMemberLoadDto, StaffOrchestrationContextDto, StaffProjectionDto,
    StaffStageCapabilityDto,
};
use vw_api_types::task::{
    TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusDto, TaskPoolWorkflowStatusRoleDto,
    TaskPoolWorkflowTransitionDto,
};
use vw_config_types::staff::{
    StaffConfig, StaffMemberConfig, StaffProjectStateConfig, StaffStageActionConfig,
    StaffStageAssignmentConfig, StaffStageWorkflowBindingConfig, StaffWorkflowStageConfig,
};

use crate::agent_scheduler::capabilities::{
    builtin_stage_capabilities, capability_by_id,
    stage_capabilities_for_config as merged_stage_capabilities_for_config,
};
use crate::agent_scheduler::lifecycle_bridge::{
    LifecycleBridge, LifecycleStatusProjection, status_role_key,
};
use crate::agent_scheduler::state_machine::AgentTaskStatus;
use crate::agent_scheduler::store::tasks::StoredAgentTask;
use crate::agent_scheduler::task_board::task_board_status_key;

pub use crate::agent_scheduler::capabilities::{
    ADVANCE_STATUS_CAPABILITY_ID, CODE_REVIEW_CAPABILITY_ID, EXECUTE_TASK_CAPABILITY_ID,
    GIT_MERGE_CAPABILITY_ID, MANUAL_CAPABILITY_ID, SAVED_WORKFLOW_CAPABILITY_ID,
};

const DEFAULT_WORKFLOW_ID: &str = "dev_lifecycle";
const BUILTIN_WORKFLOW_IDS: &[&str] = &[
    DEFAULT_WORKFLOW_ID,
    "content_publishing",
    "marketing_campaign",
    "customer_support",
    "recruiting_pipeline",
];

#[derive(Debug, Clone)]
pub struct StaffTaskReadModel {
    pub task_id: String,
    pub title: String,
    pub status_key: String,
    pub status_label: Option<String>,
    pub status_role: Option<TaskPoolWorkflowStatusRoleDto>,
    pub agent_status: Option<AgentTaskStatusDto>,
    pub priority: u32,
    pub order_no: i64,
    pub updated_at_ms: TimestampMs,
    pub last_error: Option<String>,
    pub pause_reason: Option<String>,
    pub staff_member_id: Option<String>,
    pub assignee: Option<String>,
    pub acp_agent: Option<String>,
}

#[derive(Debug, Clone)]
struct StaffRuntimeConfig {
    members: Vec<StaffMemberRuntime>,
    stages: Vec<StaffStageRuntime>,
    assignments: Vec<StaffStageAssignmentRuntime>,
    actions: Vec<StaffStageActionRuntime>,
}

#[derive(Debug, Clone)]
struct StaffMemberRuntime {
    id: String,
    name: String,
    role: String,
    level: String,
    status: String,
    enabled: bool,
    capacity: u32,
    delegate_agent: Option<String>,
    acp_agent: Option<String>,
    ipc_agent_id: Option<String>,
    current_task_ids: Vec<String>,
}

#[derive(Debug, Clone)]
struct StaffStageRuntime {
    stage_id: String,
    status_key: String,
    label: String,
    role: Option<String>,
    enabled: bool,
    display_only: bool,
}

#[derive(Debug, Clone)]
struct StaffStageAssignmentRuntime {
    stage_id: String,
    status_key: String,
    primary_member_id: Option<String>,
    backup_member_ids: Vec<String>,
    manager_member_ids: Vec<String>,
    autonomous: bool,
}

#[derive(Debug, Clone)]
struct StaffStageActionRuntime {
    stage_id: String,
    status_key: String,
    capability_id: String,
    workflow_uuid: Option<String>,
    inputs: AgentSchedulerStageActionInputsDto,
}

#[derive(Clone)]
struct AssignmentMemberInfo {
    id: String,
    name: String,
    role: String,
}

struct MemberRoutingState<'a> {
    member: &'a StaffMemberRuntime,
    task_ids: Vec<String>,
    running_task_ids: Vec<String>,
    blocked_task_ids: Vec<String>,
}

enum StageAssignmentChoice {
    Selected(usize),
    Unavailable(String),
}

#[derive(Clone, Copy)]
struct RoutingStage<'a> {
    stage: &'a StaffStageRuntime,
    assignment: Option<&'a StaffStageAssignmentRuntime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutoScheduleAction {
    Assign,
    Start,
    Resume,
}

impl AutoScheduleAction {
    fn key(self) -> &'static str {
        match self {
            AutoScheduleAction::Assign => "assign",
            AutoScheduleAction::Start => "start",
            AutoScheduleAction::Resume => "resume",
        }
    }

    fn target_label(self) -> &'static str {
        match self {
            AutoScheduleAction::Assign => "分配",
            AutoScheduleAction::Start => "开始",
            AutoScheduleAction::Resume => "继续",
        }
    }
}

pub fn stage_capabilities() -> Vec<StaffStageCapabilityDto> {
    builtin_stage_capabilities()
}

pub fn stage_capabilities_for_config(config: &StaffConfig) -> Vec<StaffStageCapabilityDto> {
    merged_stage_capabilities_for_config(config)
}

pub fn workflow_snapshot_from_staff_config(config: &StaffConfig) -> TaskPoolWorkflowSnapshotDto {
    let stages = config
        .workflow_stages
        .iter()
        .enumerate()
        .map(|(index, stage)| {
            let role = status_role_for_key(&stage.task_status)
                .unwrap_or(TaskPoolWorkflowStatusRoleDto::Custom);
            let next = config.workflow_stages.get(index + 1).map(|next| next.task_status.clone());
            TaskPoolWorkflowStatusDto {
                key: stage.task_status.trim().to_string(),
                label: stage.name.trim().to_string(),
                role,
                enabled: stage.enabled,
                terminal: stage.display_only,
                transitions: retry_transition_for_failed_role(role, next.as_deref()),
                next,
            }
        })
        .collect();
    TaskPoolWorkflowSnapshotDto {
        id: DEFAULT_WORKFLOW_ID.to_string(),
        label: "Default Task Workflow".to_string(),
        statuses: stages,
    }
}

pub fn build_staff_projection(
    tasks: &[StaffTaskReadModel],
    config: &StaffConfig,
    workflow: &TaskPoolWorkflowSnapshotDto,
    running_task_ids: &[String],
) -> Result<StaffProjectionDto, String> {
    let config = runtime_config(config, workflow, tasks)?;
    let workflow_index = WorkflowIndex::new(workflow);
    let running_task_set = running_task_ids.iter().map(String::as_str).collect::<HashSet<_>>();
    let task_id_set = tasks.iter().map(|task| task.task_id.as_str()).collect::<HashSet<_>>();
    let mut member_states = build_member_states(&config, &task_id_set, &running_task_set);
    let mut lanes = build_lanes(&config);
    let mut lane_index_by_status = lanes
        .iter()
        .enumerate()
        .map(|(index, lane)| (lane.status_key.clone(), index))
        .collect::<HashMap<_, _>>();
    let mut unassigned = Vec::new();

    for task in tasks {
        let stage = stage_for_status(&config, &task.status_key)?;
        let assignment =
            route_task(task, stage, &mut member_states, &running_task_set, &workflow_index);
        if assignment.member_id.is_none() && !assignment.display_only {
            unassigned.push(assignment.clone());
        }
        push_assignment(&mut lanes, &mut lane_index_by_status, stage.stage, assignment);
    }

    let blocked_task_count = lanes.iter().map(|lane| lane.blocked_count).sum();
    Ok(StaffProjectionDto {
        lanes,
        member_loads: member_states.into_iter().map(MemberRoutingState::into_load).collect(),
        unassigned,
        total_task_count: tasks.len(),
        blocked_task_count,
    })
}

pub fn staff_task_read_model_from_task(
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
        agent_status: Some(agent_task_status_to_dto(task.status)),
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

pub fn running_staff_task_ids(tasks: &[StaffTaskReadModel]) -> Vec<String> {
    tasks
        .iter()
        .filter(|task| task.agent_status.as_ref().is_some_and(agent_task_status_is_running))
        .map(|task| task.task_id.clone())
        .collect()
}

pub fn validate_staff_command(
    command: &StaffCommandDto,
    tasks: &[StaffTaskReadModel],
    config: &StaffConfig,
    workflow: &TaskPoolWorkflowSnapshotDto,
) -> Result<StaffCommandActionDto, String> {
    let config = runtime_config(config, workflow, tasks)?;
    let workflow_index = WorkflowIndex::new(workflow);
    let lifecycle_bridge = LifecycleBridge::new(workflow);
    match command {
        StaffCommandDto::AssignTask { task_id, member_id, .. }
        | StaffCommandDto::TakeOverTask { task_id, member_id, .. } => {
            let (task, member) = validate_task_member_command(command, tasks, &config)?;
            ensure_member_capacity(member, task_id)?;
            Ok(StaffCommandActionDto::AssignTask {
                task_id: task.task_id.clone(),
                member_id: member_id.clone(),
            })
        }
        StaffCommandDto::StartTask { .. } => {
            let (task, member) = validate_task_member_command(command, tasks, &config)?;
            ensure_member_capacity(member, &task.task_id)?;
            start_action(task, member, &config, &workflow_index, &lifecycle_bridge)
        }
        StaffCommandDto::PauseTask { .. } => {
            let (task, _) = validate_task_member_command(command, tasks, &config)?;
            let Some(to_status_key) = pause_target_status(task, &workflow_index) else {
                return Err(cannot_pause_status(task));
            };
            status_change_action(task, &to_status_key, &lifecycle_bridge)
        }
        StaffCommandDto::ResumeTask { .. } => {
            let (task, _) = validate_task_member_command(command, tasks, &config)?;
            let Some(to_status_key) = resume_target_status(task, &workflow_index) else {
                return Err(cannot_resume_status(task));
            };
            status_change_action(task, &to_status_key, &lifecycle_bridge)
        }
        StaffCommandDto::OpenTaskLogs { task_id, .. } => {
            let task = find_task(tasks, task_id)?;
            Ok(StaffCommandActionDto::ViewTaskLogs { task_id: task.task_id.clone() })
        }
        StaffCommandDto::FilterByMember { member_id, .. } => {
            let member = find_member(&config, member_id)?;
            ensure_member_enabled(member)?;
            Ok(StaffCommandActionDto::FilterByMember { member_id: member.id.clone() })
        }
    }
}

pub fn build_staff_auto_schedule_plan(
    tasks: &[StaffTaskReadModel],
    config: &StaffConfig,
    projection: &StaffProjectionDto,
    workflow: &TaskPoolWorkflowSnapshotDto,
    running_task_ids: &[String],
    now_ms: TimestampMs,
) -> Result<StaffAutoSchedulePlanDto, String> {
    let config = runtime_config(config, workflow, tasks)?;
    let workflow_index = WorkflowIndex::new(workflow);
    let running_task_ids = running_task_ids.iter().map(String::as_str).collect::<HashSet<_>>();
    let mut member_loads = config
        .members
        .iter()
        .map(|member| (member.id.as_str(), member.current_task_ids.len()))
        .collect::<HashMap<_, _>>();
    let mut scheduled = HashSet::new();
    let mut plan = StaffAutoSchedulePlanDto::default();

    for task in tasks {
        let Some(stage_assignment) = config
            .assignments
            .iter()
            .find(|assignment| assignment.status_key == task.status_key && assignment.autonomous)
        else {
            continue;
        };

        if task_is_ended(task, &workflow_index) {
            push_skip(&mut plan, task, "task ended");
            continue;
        }

        if running_task_ids.contains(task.task_id.as_str()) {
            push_skip(&mut plan, task, "task is already in the running list");
            continue;
        }

        let projection_assignment = find_projection_assignment(projection, &task.task_id);
        if let Some(member) = current_member_for_task(&config, &task.task_id) {
            if member_skip_reason(&config, task, member).is_none() {
                schedule_assigned_task(
                    &mut plan,
                    &mut scheduled,
                    &config,
                    stage_assignment,
                    projection_assignment,
                    task,
                    member,
                    now_ms,
                );
                continue;
            }
        }

        schedule_stage_assignment_member(
            &mut plan,
            &mut scheduled,
            &config,
            stage_assignment,
            task,
            &mut member_loads,
            now_ms,
        );
    }

    Ok(plan)
}

fn schedule_stage_assignment_member(
    plan: &mut StaffAutoSchedulePlanDto,
    scheduled: &mut HashSet<String>,
    config: &StaffRuntimeConfig,
    stage_assignment: &StaffStageAssignmentRuntime,
    task: &StaffTaskReadModel,
    member_loads: &mut HashMap<&str, usize>,
    now_ms: TimestampMs,
) {
    match select_stage_assignment_member(config, stage_assignment, task, member_loads) {
        Ok(member_id) => {
            let command = StaffCommandDto::AssignTask {
                command_id: auto_command_id(
                    AutoScheduleAction::Assign,
                    &task.task_id,
                    &member_id,
                    now_ms,
                ),
                task_id: task.task_id.clone(),
                member_id: member_id.clone(),
            };
            let orchestration =
                config.members.iter().find(|member| member.id == member_id).map(|member| {
                    orchestration_context(
                        config,
                        stage_assignment,
                        task,
                        member,
                        AutoScheduleAction::Assign,
                    )
                });
            if push_command(plan, scheduled, command, &task.status_key, orchestration)
                && let Some(load) = member_loads.get_mut(member_id.as_str())
            {
                *load = load.saturating_add(1);
            }
        }
        Err(reason) => push_skip(plan, task, reason),
    }
}

pub fn staff_auto_schedule_command_is_assignment(command: &StaffCommandDto) -> bool {
    matches!(command, StaffCommandDto::AssignTask { .. })
}

pub fn staff_auto_schedule_command_is_continuation(command: &StaffCommandDto) -> bool {
    matches!(command, StaffCommandDto::StartTask { .. } | StaffCommandDto::ResumeTask { .. })
}

pub fn scheduler_command_for_action(
    action: &StaffCommandActionDto,
    source_command: &StaffCommandDto,
    reason: Option<String>,
) -> Option<AgentSchedulerCommandRequest> {
    let command_id = command_id(source_command).to_string();
    match action {
        StaffCommandActionDto::AssignTask { task_id, member_id } => {
            Some(AgentSchedulerCommandRequest {
                task_id: task_id.clone().into(),
                command: AgentSchedulerCommandKindDto::AssignStaff,
                dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                max_concurrent: None,
                reason,
                payload: AgentSchedulerCommandPayloadDto {
                    staff_command: Some(AgentSchedulerStaffCommandPayloadDto {
                        command_id,
                        member_id: member_id.clone(),
                        stage_id: None,
                        role: None,
                        action: Some("assign_task".to_string()),
                        metadata: None,
                    }),
                    ..AgentSchedulerCommandPayloadDto::default()
                },
            })
        }
        StaffCommandActionDto::ChangeTaskStatus { task_id, to_status_key, to_status } => {
            Some(AgentSchedulerCommandRequest {
                task_id: task_id.clone().into(),
                command: AgentSchedulerCommandKindDto::Move,
                dispatch_mode: AgentSchedulerDispatchModeDto::Local,
                max_concurrent: None,
                reason,
                payload: AgentSchedulerCommandPayloadDto {
                    move_task: Some(AgentSchedulerMoveTaskPayloadDto {
                        from_status_key: None,
                        to_status_key: to_status_key.clone(),
                        to_status: to_status.clone(),
                        order_no: None,
                    }),
                    ..AgentSchedulerCommandPayloadDto::default()
                },
            })
        }
        StaffCommandActionDto::ExecuteTask { task_id } => Some(AgentSchedulerCommandRequest {
            task_id: task_id.clone().into(),
            command: AgentSchedulerCommandKindDto::Start,
            dispatch_mode: AgentSchedulerDispatchModeDto::Local,
            max_concurrent: None,
            reason,
            payload: AgentSchedulerCommandPayloadDto::default(),
        }),
        StaffCommandActionDto::RunCodeReview { task_id } => stage_action_command(
            task_id,
            CODE_REVIEW_CAPABILITY_ID,
            None,
            member_id(source_command).map(str::to_string),
            reason,
        ),
        StaffCommandActionDto::RunMerge { task_id } => Some(AgentSchedulerCommandRequest {
            task_id: task_id.clone().into(),
            command: AgentSchedulerCommandKindDto::Merge,
            dispatch_mode: AgentSchedulerDispatchModeDto::Local,
            max_concurrent: None,
            reason,
            payload: AgentSchedulerCommandPayloadDto::default(),
        }),
        StaffCommandActionDto::RunStageWorkflow {
            task_id,
            stage_id,
            workflow_uuid,
            member_id,
            ..
        } => stage_action_command(
            task_id,
            SAVED_WORKFLOW_CAPABILITY_ID,
            Some((stage_id.clone(), workflow_uuid.clone())),
            Some(member_id.clone()),
            reason,
        ),
        StaffCommandActionDto::ViewTaskLogs { .. }
        | StaffCommandActionDto::FilterByMember { .. } => None,
    }
}

pub fn validate_stage_action_payload(
    stage_id: &str,
    capability_id: &str,
    workflow_uuid: Option<&str>,
    inputs: &AgentSchedulerStageActionInputsDto,
    role: Option<TaskPoolWorkflowStatusRoleDto>,
    catalog: &[StaffStageCapabilityDto],
) -> Result<(), String> {
    let stage_id = non_empty(stage_id, "stage_id")?;
    let capability_id = non_empty(capability_id, "capability_id")?;
    let Some(capability) = capability_by_id(catalog, capability_id) else {
        return Err(format!("unsupported staff stage capability_id: {capability_id}"));
    };
    validate_stage_action_role(stage_id, capability_id, role, capability)?;
    validate_stage_action_inputs(stage_id, capability_id, workflow_uuid, inputs, capability)?;
    Ok(())
}

pub fn resolve_stage_action_outcome_target_status_key(
    workflow: &TaskPoolWorkflowSnapshotDto,
    from_status_key: &str,
    outcome: AgentSchedulerStageActionOutcomeDto,
) -> Result<String, String> {
    Ok(resolve_stage_action_outcome_target_status(workflow, from_status_key, outcome)?.status_key)
}

pub fn resolve_stage_action_outcome_target_status(
    workflow: &TaskPoolWorkflowSnapshotDto,
    from_status_key: &str,
    outcome: AgentSchedulerStageActionOutcomeDto,
) -> Result<LifecycleStatusProjection, String> {
    let from_status_key = non_empty(from_status_key, "from_status_key")?;
    let outcome_key = stage_action_outcome_key(outcome);
    LifecycleBridge::new(workflow)
        .resolve_outcome(from_status_key, outcome_key)
        .map_err(|error| stage_action_outcome_error(from_status_key, outcome_key, error))
}

fn validate_stage_action_inputs(
    stage_id: &str,
    capability_id: &str,
    workflow_uuid: Option<&str>,
    inputs: &AgentSchedulerStageActionInputsDto,
    capability: &StaffStageCapabilityDto,
) -> Result<(), String> {
    let payload_workflow_uuid = workflow_uuid.map(str::trim).filter(|value| !value.is_empty());
    let input_workflow_uuid =
        inputs.workflow_uuid.as_deref().map(str::trim).filter(|value| !value.is_empty());
    if let (Some(payload_workflow_uuid), Some(input_workflow_uuid)) =
        (payload_workflow_uuid, input_workflow_uuid)
    {
        if payload_workflow_uuid != input_workflow_uuid {
            return Err(format!(
                "staff stage action workflow_uuid for {stage_id} conflicts with inputs.workflow_uuid for capability {capability_id}"
            ));
        }
    }
    let effective_workflow_uuid = payload_workflow_uuid.or(input_workflow_uuid);
    if capability.requires_workflow_uuid && effective_workflow_uuid.is_none() {
        return Err(format!(
            "staff stage action workflow_uuid for {stage_id} capability {capability_id} cannot be empty"
        ));
    }
    if !capability.requires_workflow_uuid && effective_workflow_uuid.is_some() {
        return Err(format!(
            "staff stage action workflow_uuid for {stage_id} capability {capability_id} must be empty"
        ));
    }
    for parameter_key in inputs.parameters.keys() {
        if parameter_key.trim().is_empty() {
            return Err(format!(
                "staff stage action input parameter for {stage_id} capability {capability_id} cannot be empty"
            ));
        }
    }
    Ok(())
}

fn validate_stage_action_role(
    stage_id: &str,
    capability_id: &str,
    role: Option<TaskPoolWorkflowStatusRoleDto>,
    capability: &StaffStageCapabilityDto,
) -> Result<(), String> {
    let Some(role) = role else {
        return Err(format!(
            "staff stage action capability {capability_id} for {stage_id} requires current status role"
        ));
    };
    if capability.supported_roles.contains(&role) {
        return Ok(());
    }
    Err(format!(
        "staff stage action capability {capability_id} does not support status role {} for {stage_id}",
        status_role_key(role)
    ))
}

fn stage_action_command(
    task_id: &str,
    capability_id: &str,
    saved_workflow: Option<(String, String)>,
    member_id: Option<String>,
    reason: Option<String>,
) -> Option<AgentSchedulerCommandRequest> {
    let stage_id = saved_workflow
        .as_ref()
        .map(|(stage_id, _)| stage_id.clone())
        .unwrap_or_else(|| capability_id.to_string());
    let workflow_uuid = saved_workflow.map(|(_, workflow_uuid)| workflow_uuid);
    let inputs = AgentSchedulerStageActionInputsDto {
        workflow_uuid: workflow_uuid.clone(),
        ..AgentSchedulerStageActionInputsDto::default()
    };
    Some(AgentSchedulerCommandRequest {
        task_id: task_id.to_string().into(),
        command: AgentSchedulerCommandKindDto::RunStageAction,
        dispatch_mode: AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason,
        payload: AgentSchedulerCommandPayloadDto {
            stage_action: Some(AgentSchedulerStageActionCommandPayloadDto {
                stage_id,
                capability_id: capability_id.to_string(),
                workflow_uuid,
                inputs,
                outcome: None,
                member_id,
                metadata: Some(json!({"source": "staff_command"})),
            }),
            ..AgentSchedulerCommandPayloadDto::default()
        },
    })
}

fn runtime_config(
    config: &StaffConfig,
    workflow: &TaskPoolWorkflowSnapshotDto,
    tasks: &[StaffTaskReadModel],
) -> Result<StaffRuntimeConfig, String> {
    let config = staff_config_for_workflow(config, workflow).normalize()?;
    let mut members = config.members.iter().map(member_runtime).collect::<Vec<_>>();
    apply_member_task_assignments(&mut members, &config.project_state.member_task_assignments);
    for task in tasks {
        if let Some(member_id) = task.staff_member_id.as_deref() {
            if let Some(member) = members.iter_mut().find(|member| member.id == member_id) {
                push_unique(&mut member.current_task_ids, task.task_id.clone());
            }
        }
    }

    let stages = config.workflow_stages.iter().map(stage_runtime).collect::<Vec<_>>();
    let actionable_status_key_by_stage_id = stages
        .iter()
        .filter(|stage| stage.enabled && !stage.display_only && stage.role.is_some())
        .map(|stage| (stage.stage_id.clone(), stage.status_key.clone()))
        .collect();
    let capability_catalog = stage_capabilities_for_config(&config);
    let mut actions = stage_actions_from_config(
        &config.project_state.stage_actions,
        &config.project_state.stage_workflow_bindings,
        &actionable_status_key_by_stage_id,
        workflow,
        &capability_catalog,
    )?;
    merge_default_stage_actions(&mut actions, &stages, workflow);
    validate_stage_actions(&actions, workflow, &capability_catalog)?;
    let assignments = stage_assignments_from_config(
        &config.project_state.stage_assignments,
        &actionable_status_key_by_stage_id,
    );

    Ok(StaffRuntimeConfig { members, stages, assignments, actions })
}

pub(crate) fn staff_config_for_workflow(
    config: &StaffConfig,
    workflow: &TaskPoolWorkflowSnapshotDto,
) -> StaffConfig {
    let mut config = config.clone();
    if workflow_snapshot_from_staff_config(&config) != *workflow {
        config.workflow_stages = staff_workflow_stages_from_workflow(workflow);
        let stage_ids =
            config.workflow_stages.iter().map(|stage| stage.id.clone()).collect::<HashSet<_>>();
        config
            .project_state
            .stage_assignments
            .retain(|assignment| stage_ids.contains(assignment.stage_id.trim()));
        config
            .project_state
            .stage_actions
            .retain(|action| stage_ids.contains(action.stage_id.trim()));
    }
    config.project_state.stage_workflow_bindings.clear();
    config
}

pub(crate) fn staff_config_for_workflow_orchestration(
    config: &StaffConfig,
    workflow_id: &str,
    workflow: &TaskPoolWorkflowSnapshotDto,
) -> StaffConfig {
    let mut config = staff_config_for_workflow(config, workflow);
    let member_task_assignments = config.project_state.member_task_assignments.clone();
    config.project_state =
        StaffProjectStateConfig { member_task_assignments, ..StaffProjectStateConfig::default() };

    if let Some(orchestration) = config.workflow_orchestrations.get(workflow_id.trim()) {
        config.project_state.capabilities = orchestration.capabilities.clone();
        config.project_state.stage_assignments = orchestration.stage_assignments.clone();
        config.project_state.stage_actions = orchestration.stage_actions.clone();
    } else if BUILTIN_WORKFLOW_IDS.contains(&workflow.id.as_str()) {
        config.project_state.stage_assignments =
            default_stage_assignments_for_workflow(&config.members, workflow);
    }

    config
}

fn default_stage_assignments_for_workflow(
    members: &[StaffMemberConfig],
    workflow: &TaskPoolWorkflowSnapshotDto,
) -> Vec<StaffStageAssignmentConfig> {
    workflow
        .statuses
        .iter()
        .filter(|status| status.enabled && default_stage_should_autonomously_continue(status.role))
        .filter_map(|status| {
            let member_role = staff_role_for_status_role(status.role)?;
            let member_id = members
                .iter()
                .find(|member| member.enabled && member.role.trim() == member_role)?
                .id
                .clone();
            Some(StaffStageAssignmentConfig {
                stage_id: status.key.clone(),
                primary_member_id: Some(member_id),
                backup_member_ids: Vec::new(),
                manager_member_ids: Vec::new(),
                autonomous: true,
            })
        })
        .collect()
}

fn default_stage_should_autonomously_continue(role: TaskPoolWorkflowStatusRoleDto) -> bool {
    matches!(
        role,
        TaskPoolWorkflowStatusRoleDto::Backlog
            | TaskPoolWorkflowStatusRoleDto::Queued
            | TaskPoolWorkflowStatusRoleDto::Planning
            | TaskPoolWorkflowStatusRoleDto::ReviewReady
            | TaskPoolWorkflowStatusRoleDto::Review
            | TaskPoolWorkflowStatusRoleDto::Merging
    )
}

fn staff_workflow_stages_from_workflow(
    workflow: &TaskPoolWorkflowSnapshotDto,
) -> Vec<StaffWorkflowStageConfig> {
    workflow
        .statuses
        .iter()
        .map(|status| {
            let role = status
                .enabled
                .then(|| staff_role_for_status_role(status.role).map(ToString::to_string))
                .flatten();
            StaffWorkflowStageConfig {
                id: status.key.clone(),
                name: status.label.clone(),
                task_status: status.key.clone(),
                role: role.clone(),
                enabled: status.enabled,
                display_only: !status.enabled || role.is_none(),
            }
        })
        .collect()
}

fn staff_role_for_status_role(status_role: TaskPoolWorkflowStatusRoleDto) -> Option<&'static str> {
    match status_role {
        TaskPoolWorkflowStatusRoleDto::Backlog
        | TaskPoolWorkflowStatusRoleDto::Queued
        | TaskPoolWorkflowStatusRoleDto::Failed
        | TaskPoolWorkflowStatusRoleDto::Paused => Some("orchestrator"),
        TaskPoolWorkflowStatusRoleDto::Planning => Some("planner"),
        TaskPoolWorkflowStatusRoleDto::Executing => Some("executor"),
        TaskPoolWorkflowStatusRoleDto::ReviewReady | TaskPoolWorkflowStatusRoleDto::Review => {
            Some("reviewer")
        }
        TaskPoolWorkflowStatusRoleDto::Merging => Some("merger"),
        TaskPoolWorkflowStatusRoleDto::Done
        | TaskPoolWorkflowStatusRoleDto::Archived
        | TaskPoolWorkflowStatusRoleDto::Custom => None,
    }
}

fn member_runtime(member: &StaffMemberConfig) -> StaffMemberRuntime {
    StaffMemberRuntime {
        id: member.id.clone(),
        name: member.name.clone(),
        role: member.role.clone(),
        level: member.level.clone(),
        status: member.status.clone(),
        enabled: member.enabled,
        capacity: member.capacity,
        delegate_agent: member.delegate_agent.clone(),
        acp_agent: member.acp_agent.clone(),
        ipc_agent_id: member.ipc_agent_id.clone(),
        current_task_ids: Vec::new(),
    }
}

fn stage_runtime(stage: &StaffWorkflowStageConfig) -> StaffStageRuntime {
    StaffStageRuntime {
        stage_id: stage.id.clone(),
        status_key: stage.task_status.clone(),
        label: stage.name.clone(),
        role: stage.role.clone(),
        enabled: stage.enabled,
        display_only: stage.display_only,
    }
}

fn apply_member_task_assignments(
    members: &mut [StaffMemberRuntime],
    assignments: &[vw_config_types::staff::StaffMemberTaskAssignmentConfig],
) {
    for assignment in assignments {
        if let Some(member) = members.iter_mut().find(|member| member.id == assignment.member_id) {
            for task_id in &assignment.task_ids {
                push_unique(&mut member.current_task_ids, task_id.clone());
            }
        }
    }
}

fn stage_assignments_from_config(
    assignments: &[StaffStageAssignmentConfig],
    status_key_by_stage_id: &HashMap<String, String>,
) -> Vec<StaffStageAssignmentRuntime> {
    assignments
        .iter()
        .filter_map(|assignment| {
            let status_key = status_key_by_stage_id.get(&assignment.stage_id)?;
            Some(StaffStageAssignmentRuntime {
                stage_id: assignment.stage_id.clone(),
                status_key: status_key.clone(),
                primary_member_id: assignment.primary_member_id.clone(),
                backup_member_ids: assignment.backup_member_ids.clone(),
                manager_member_ids: assignment.manager_member_ids.clone(),
                autonomous: assignment.autonomous,
            })
        })
        .collect()
}

fn stage_action_inputs_from_config(
    action: &StaffStageActionConfig,
) -> AgentSchedulerStageActionInputsDto {
    AgentSchedulerStageActionInputsDto {
        workflow_uuid: action.inputs.workflow_uuid.clone(),
        parameters: action.inputs.parameters.clone(),
    }
}

fn stage_actions_from_config(
    actions: &[StaffStageActionConfig],
    bindings: &[StaffStageWorkflowBindingConfig],
    status_key_by_stage_id: &HashMap<String, String>,
    workflow: &TaskPoolWorkflowSnapshotDto,
    capability_catalog: &[StaffStageCapabilityDto],
) -> Result<Vec<StaffStageActionRuntime>, String> {
    let mut runtime_actions = actions
        .iter()
        .filter_map(|action| {
            let status_key = status_key_by_stage_id.get(&action.stage_id)?;
            Some(StaffStageActionRuntime {
                stage_id: action.stage_id.clone(),
                status_key: status_key.clone(),
                capability_id: action.capability_id.clone(),
                workflow_uuid: action
                    .workflow_uuid
                    .clone()
                    .or_else(|| action.inputs.workflow_uuid.clone()),
                inputs: stage_action_inputs_from_config(action),
            })
        })
        .collect::<Vec<_>>();
    let mut configured_stage_ids =
        runtime_actions.iter().map(|action| action.stage_id.clone()).collect::<HashSet<_>>();
    for binding in bindings {
        if !configured_stage_ids.insert(binding.stage_id.clone()) {
            continue;
        }
        let Some(status_key) = status_key_by_stage_id.get(&binding.stage_id) else {
            continue;
        };
        runtime_actions.push(StaffStageActionRuntime {
            stage_id: binding.stage_id.clone(),
            status_key: status_key.clone(),
            capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
            workflow_uuid: Some(binding.workflow_uuid.clone()),
            inputs: AgentSchedulerStageActionInputsDto {
                workflow_uuid: Some(binding.workflow_uuid.clone()),
                ..AgentSchedulerStageActionInputsDto::default()
            },
        });
    }
    validate_stage_actions(&runtime_actions, workflow, capability_catalog)?;
    Ok(runtime_actions)
}

fn merge_default_stage_actions(
    actions: &mut Vec<StaffStageActionRuntime>,
    stages: &[StaffStageRuntime],
    workflow: &TaskPoolWorkflowSnapshotDto,
) {
    if !BUILTIN_WORKFLOW_IDS.contains(&workflow.id.as_str()) {
        return;
    }
    let workflow_index = WorkflowIndex::new(workflow);
    let mut configured_statuses =
        actions.iter().map(|action| action.status_key.clone()).collect::<HashSet<_>>();
    for stage in stages {
        if !configured_statuses.insert(stage.status_key.clone()) {
            continue;
        }
        let Some(capability_id) = workflow_index
            .role(&stage.status_key)
            .and_then(|role| default_stage_capability_for_role(&workflow.id, role))
        else {
            continue;
        };
        actions.push(StaffStageActionRuntime {
            stage_id: stage.stage_id.clone(),
            status_key: stage.status_key.clone(),
            capability_id: capability_id.to_string(),
            workflow_uuid: None,
            inputs: AgentSchedulerStageActionInputsDto::default(),
        });
    }
}

fn default_stage_capability_for_role(
    workflow_id: &str,
    status_role: TaskPoolWorkflowStatusRoleDto,
) -> Option<&'static str> {
    match status_role {
        TaskPoolWorkflowStatusRoleDto::Backlog
        | TaskPoolWorkflowStatusRoleDto::Failed
        | TaskPoolWorkflowStatusRoleDto::ReviewReady => Some(ADVANCE_STATUS_CAPABILITY_ID),
        TaskPoolWorkflowStatusRoleDto::Queued | TaskPoolWorkflowStatusRoleDto::Planning => {
            Some(EXECUTE_TASK_CAPABILITY_ID)
        }
        TaskPoolWorkflowStatusRoleDto::Review if workflow_id == DEFAULT_WORKFLOW_ID => {
            Some(CODE_REVIEW_CAPABILITY_ID)
        }
        TaskPoolWorkflowStatusRoleDto::Merging if workflow_id == DEFAULT_WORKFLOW_ID => {
            Some(GIT_MERGE_CAPABILITY_ID)
        }
        _ => None,
    }
}

fn validate_stage_actions(
    actions: &[StaffStageActionRuntime],
    workflow: &TaskPoolWorkflowSnapshotDto,
    capability_catalog: &[StaffStageCapabilityDto],
) -> Result<(), String> {
    let workflow_index = WorkflowIndex::new(workflow);
    for action in actions {
        validate_stage_action_payload(
            &action.stage_id,
            &action.capability_id,
            action.workflow_uuid.as_deref(),
            &action.inputs,
            workflow_index.role(&action.status_key),
            capability_catalog,
        )?;
    }
    Ok(())
}

fn build_lanes(config: &StaffRuntimeConfig) -> Vec<StaffLaneDto> {
    let mut seen = HashSet::new();
    let mut lanes = Vec::new();
    for stage in &config.stages {
        if seen.insert(stage.status_key.clone()) {
            lanes.push(StaffLaneDto {
                stage_id: stage.stage_id.clone(),
                status_key: stage.status_key.clone(),
                status_label: stage.label.clone(),
                role: stage.role.clone(),
                display_only: stage.display_only,
                assignments: Vec::new(),
                task_count: 0,
                blocked_count: 0,
            });
        }
    }
    lanes
}

fn build_member_states<'a>(
    config: &'a StaffRuntimeConfig,
    task_id_set: &HashSet<&str>,
    running_task_set: &HashSet<&str>,
) -> Vec<MemberRoutingState<'a>> {
    config
        .members
        .iter()
        .map(|member| {
            let task_ids = member
                .current_task_ids
                .iter()
                .filter(|task_id| !task_id_set.contains(task_id.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            let running_task_ids = task_ids
                .iter()
                .filter(|task_id| running_task_set.contains(task_id.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            MemberRoutingState { member, task_ids, running_task_ids, blocked_task_ids: Vec::new() }
        })
        .collect()
}

fn route_task(
    task: &StaffTaskReadModel,
    stage: RoutingStage<'_>,
    member_states: &mut [MemberRoutingState<'_>],
    running_task_set: &HashSet<&str>,
    workflow: &WorkflowIndex<'_>,
) -> StaffAssignmentDto {
    if task_is_ended(task, workflow) || stage.stage.display_only {
        return build_assignment(task, stage.stage, None, true, false, None, workflow);
    }

    let mut stage_assignment_unavailable = None;
    if let Some(assignment) = stage.assignment {
        match choose_member_for_stage_assignment(member_states, assignment) {
            StageAssignmentChoice::Selected(member_index) => {
                let blocked = task_is_blocked(task, workflow);
                let block_reason = blocked.then(|| blocked_task_reason(task));
                let member_info = member_info(member_states[member_index].member);
                record_member_task(
                    &mut member_states[member_index],
                    task,
                    running_task_set,
                    blocked,
                );
                return build_assignment(
                    task,
                    stage.stage,
                    Some(member_info),
                    false,
                    blocked,
                    block_reason,
                    workflow,
                );
            }
            StageAssignmentChoice::Unavailable(reason) => {
                stage_assignment_unavailable = Some(reason);
            }
        }
    }

    if task_is_blocked(task, workflow) {
        return route_blocked_task(task, stage, member_states, running_task_set, workflow);
    }

    let Some(role) = stage.stage.role.as_deref() else {
        let reason = match stage_assignment_unavailable {
            Some(stage_reason) => {
                format!("{stage_reason}; no staff role configured for status {}", task.status_key)
            }
            None => format!("no staff role configured for status {}", task.status_key),
        };
        return build_assignment(task, stage.stage, None, false, true, Some(reason), workflow);
    };

    let preferred_index = find_original_member_index(task, member_states);
    let Some(member_index) = choose_member_for_role(member_states, role, preferred_index) else {
        let role_reason = unavailable_role_reason(role, member_states);
        let reason = match stage_assignment_unavailable {
            Some(stage_reason) => format!("{stage_reason}; {role_reason}"),
            None => role_reason,
        };
        return build_assignment(task, stage.stage, None, false, true, Some(reason), workflow);
    };

    let member_info = member_info(member_states[member_index].member);
    record_member_task(&mut member_states[member_index], task, running_task_set, false);
    build_assignment(task, stage.stage, Some(member_info), false, false, None, workflow)
}

fn choose_member_for_stage_assignment(
    member_states: &[MemberRoutingState<'_>],
    assignment: &StaffStageAssignmentRuntime,
) -> StageAssignmentChoice {
    let mut reasons = Vec::new();
    let mut has_candidate = false;

    if let Some(member_id) = assignment.primary_member_id.as_deref().map(str::trim) {
        if !member_id.is_empty() {
            has_candidate = true;
            if let Some(index) =
                choose_stage_assignment_candidate(member_states, member_id, &mut reasons)
            {
                return StageAssignmentChoice::Selected(index);
            }
        }
    }

    for member_id in &assignment.backup_member_ids {
        let member_id = member_id.trim();
        if member_id.is_empty() {
            continue;
        }
        has_candidate = true;
        if let Some(index) =
            choose_stage_assignment_candidate(member_states, member_id, &mut reasons)
        {
            return StageAssignmentChoice::Selected(index);
        }
    }

    if !has_candidate {
        return StageAssignmentChoice::Unavailable(format!(
            "no primary or backup staff member configured for stage {}",
            assignment.status_key
        ));
    }

    StageAssignmentChoice::Unavailable(format!(
        "stage assignment unavailable for {}: {}",
        assignment.status_key,
        reasons.join("; ")
    ))
}

fn choose_stage_assignment_candidate(
    member_states: &[MemberRoutingState<'_>],
    member_id: &str,
    reasons: &mut Vec<String>,
) -> Option<usize> {
    let Some(index) = member_states.iter().position(|state| state.member.id == member_id) else {
        reasons.push(format!("configured member {member_id} does not exist"));
        return None;
    };
    if member_is_available(&member_states[index]) {
        return Some(index);
    }
    reasons.push(stage_assignment_member_unavailable_reason(&member_states[index]));
    None
}

fn member_is_available(state: &MemberRoutingState<'_>) -> bool {
    state.member.enabled && state.task_ids.len() < state.member.capacity as usize
}

fn stage_assignment_member_unavailable_reason(state: &MemberRoutingState<'_>) -> String {
    if !state.member.enabled {
        return format!("configured member {} is disabled", state.member.id);
    }
    if state.task_ids.len() >= state.member.capacity as usize {
        return format!("configured member {} has no available capacity", state.member.id);
    }
    format!("configured member {} is unavailable", state.member.id)
}

fn route_blocked_task(
    task: &StaffTaskReadModel,
    stage: RoutingStage<'_>,
    member_states: &mut [MemberRoutingState<'_>],
    running_task_set: &HashSet<&str>,
    workflow: &WorkflowIndex<'_>,
) -> StaffAssignmentDto {
    let reason = blocked_task_reason(task);
    let Some(member_index) = find_original_member_index(task, member_states) else {
        return build_assignment(
            task,
            stage.stage,
            None,
            false,
            true,
            Some(format!("{reason}; original staff member is not assigned")),
            workflow,
        );
    };

    let member_info = member_info(member_states[member_index].member);
    record_member_task(&mut member_states[member_index], task, running_task_set, true);
    build_assignment(task, stage.stage, Some(member_info), false, true, Some(reason), workflow)
}

fn choose_member_for_role(
    member_states: &[MemberRoutingState<'_>],
    role: &str,
    preferred_index: Option<usize>,
) -> Option<usize> {
    if let Some(index) = preferred_index {
        if member_is_available_for_role(&member_states[index], role) {
            return Some(index);
        }
    }

    member_states
        .iter()
        .enumerate()
        .find_map(|(index, state)| member_is_available_for_role(state, role).then_some(index))
}

fn member_is_available_for_role(state: &MemberRoutingState<'_>, role: &str) -> bool {
    state.member.role == role && member_is_available(state)
}

fn find_original_member_index(
    task: &StaffTaskReadModel,
    member_states: &[MemberRoutingState<'_>],
) -> Option<usize> {
    task.staff_member_id
        .as_deref()
        .and_then(|member_id| member_states.iter().position(|state| state.member.id == member_id))
        .or_else(|| {
            member_states.iter().position(|state| {
                state.member.current_task_ids.iter().any(|id| id == &task.task_id)
            })
        })
        .or_else(|| find_member_by_task_hints(task, member_states))
}

fn find_member_by_task_hints(
    task: &StaffTaskReadModel,
    member_states: &[MemberRoutingState<'_>],
) -> Option<usize> {
    if let Some(assignee) =
        task.assignee.as_deref().map(str::trim).filter(|value| !value.is_empty())
    {
        if let Some(index) = member_states
            .iter()
            .position(|state| state.member.id == assignee || state.member.name == assignee)
        {
            return Some(index);
        }
    }

    task.acp_agent.as_deref().map(str::trim).filter(|value| !value.is_empty()).and_then(|agent| {
        member_states.iter().position(|state| {
            state
                .member
                .acp_agent
                .as_deref()
                .map(str::trim)
                .is_some_and(|member_agent| member_agent == agent)
        })
    })
}

fn record_member_task(
    state: &mut MemberRoutingState<'_>,
    task: &StaffTaskReadModel,
    running_task_set: &HashSet<&str>,
    blocked: bool,
) {
    push_unique(&mut state.task_ids, task.task_id.clone());
    if running_task_set.contains(task.task_id.as_str()) {
        push_unique(&mut state.running_task_ids, task.task_id.clone());
    }
    if blocked {
        push_unique(&mut state.blocked_task_ids, task.task_id.clone());
    }
}

fn unavailable_role_reason(role: &str, member_states: &[MemberRoutingState<'_>]) -> String {
    let mut has_same_role = false;
    let mut has_enabled_member = false;
    for state in member_states {
        if state.member.role != role {
            continue;
        }
        has_same_role = true;
        if state.member.enabled {
            has_enabled_member = true;
        }
    }

    if !has_same_role {
        return format!("no {role} staff member is configured");
    }
    if !has_enabled_member {
        return format!("no enabled {role} staff member is available");
    }
    format!("no enabled {role} staff member has available capacity")
}

fn build_assignment(
    task: &StaffTaskReadModel,
    stage: &StaffStageRuntime,
    member: Option<AssignmentMemberInfo>,
    display_only: bool,
    blocked: bool,
    block_reason: Option<String>,
    workflow: &WorkflowIndex<'_>,
) -> StaffAssignmentDto {
    let has_member = member.is_some();
    StaffAssignmentDto {
        task_id: task.task_id.clone(),
        status_key: task.status_key.clone(),
        status_label: task.status_label.clone().unwrap_or_else(|| stage.label.clone()),
        task_description: task.title.clone(),
        priority: task.priority,
        order_no: task.order_no,
        updated_at_ms: task.updated_at_ms,
        role: stage.role.clone(),
        member_id: member.as_ref().map(|member| member.id.clone()),
        member_name: member.as_ref().map(|member| member.name.clone()),
        member_role: member.as_ref().map(|member| member.role.clone()),
        display_only,
        blocked,
        block_reason,
        actions: action_availability_for_assignment(
            &task.status_key,
            workflow,
            display_only,
            blocked,
            has_member,
        ),
    }
}

fn action_availability_for_assignment(
    status_key: &str,
    workflow: &WorkflowIndex<'_>,
    display_only: bool,
    blocked: bool,
    has_member: bool,
) -> StaffActionAvailabilityDto {
    let status_role = workflow.role(status_key);
    StaffActionAvailabilityDto {
        can_assign: !display_only && !blocked,
        can_start: has_member && !display_only && is_startable_status_role(status_role, blocked),
        can_pause: has_member
            && !blocked
            && matches!(status_role, Some(TaskPoolWorkflowStatusRoleDto::Executing))
            && workflow.status_with_role(TaskPoolWorkflowStatusRoleDto::Paused).is_some(),
        can_resume: has_member
            && matches!(status_role, Some(TaskPoolWorkflowStatusRoleDto::Paused))
            && resume_target_status_key(workflow).is_some(),
        can_open_logs: true,
        can_filter_member: has_member,
    }
}

fn is_startable_status_role(
    status_role: Option<TaskPoolWorkflowStatusRoleDto>,
    blocked: bool,
) -> bool {
    match status_role {
        Some(
            TaskPoolWorkflowStatusRoleDto::Backlog
            | TaskPoolWorkflowStatusRoleDto::Queued
            | TaskPoolWorkflowStatusRoleDto::Planning,
        ) => !blocked,
        Some(TaskPoolWorkflowStatusRoleDto::Failed) => true,
        Some(
            TaskPoolWorkflowStatusRoleDto::ReviewReady
            | TaskPoolWorkflowStatusRoleDto::Review
            | TaskPoolWorkflowStatusRoleDto::Merging,
        ) => !blocked,
        Some(
            TaskPoolWorkflowStatusRoleDto::Executing
            | TaskPoolWorkflowStatusRoleDto::Paused
            | TaskPoolWorkflowStatusRoleDto::Done
            | TaskPoolWorkflowStatusRoleDto::Archived
            | TaskPoolWorkflowStatusRoleDto::Custom,
        )
        | None => false,
    }
}

fn push_assignment(
    lanes: &mut Vec<StaffLaneDto>,
    lane_index_by_status: &mut HashMap<String, usize>,
    stage: &StaffStageRuntime,
    assignment: StaffAssignmentDto,
) {
    let index = *lane_index_by_status.entry(stage.status_key.clone()).or_insert_with(|| {
        lanes.push(StaffLaneDto {
            stage_id: stage.stage_id.clone(),
            status_key: stage.status_key.clone(),
            status_label: stage.label.clone(),
            role: stage.role.clone(),
            display_only: stage.display_only,
            assignments: Vec::new(),
            task_count: 0,
            blocked_count: 0,
        });
        lanes.len() - 1
    });
    let lane = &mut lanes[index];
    if assignment.blocked {
        lane.blocked_count = lane.blocked_count.saturating_add(1);
    }
    lane.assignments.push(assignment);
    lane.task_count = lane.assignments.len();
}

fn member_info(member: &StaffMemberRuntime) -> AssignmentMemberInfo {
    AssignmentMemberInfo {
        id: member.id.clone(),
        name: member.name.clone(),
        role: member.role.clone(),
    }
}

impl MemberRoutingState<'_> {
    fn into_load(self) -> StaffMemberLoadDto {
        let load_count = self.task_ids.len();
        StaffMemberLoadDto {
            member_id: self.member.id.clone(),
            member_name: self.member.name.clone(),
            role: self.member.role.clone(),
            level: self.member.level.clone(),
            status: self.member.status.clone(),
            enabled: self.member.enabled,
            capacity: self.member.capacity,
            delegate_agent: self.member.delegate_agent.clone(),
            acp_agent: self.member.acp_agent.clone(),
            ipc_agent_id: self.member.ipc_agent_id.clone(),
            task_ids: self.task_ids,
            running_task_ids: self.running_task_ids,
            blocked_task_ids: self.blocked_task_ids,
            load_count,
            over_capacity: load_count > self.member.capacity as usize,
        }
    }
}

fn stage_for_status<'a>(
    config: &'a StaffRuntimeConfig,
    status_key: &str,
) -> Result<RoutingStage<'a>, String> {
    let stage = config
        .stages
        .iter()
        .find(|stage| stage.status_key == status_key)
        .or_else(|| config.stages.first())
        .ok_or_else(|| "staff workflow contains no stages".to_string())?;
    let assignment =
        config.assignments.iter().find(|assignment| assignment.status_key == status_key);
    Ok(RoutingStage { stage, assignment })
}

fn validate_task_member_command<'a>(
    command: &StaffCommandDto,
    tasks: &'a [StaffTaskReadModel],
    config: &'a StaffRuntimeConfig,
) -> Result<(&'a StaffTaskReadModel, &'a StaffMemberRuntime), String> {
    let task_id = task_id(command).unwrap_or_default();
    let member_id = member_id(command).unwrap_or_default();
    let task = find_task(tasks, task_id)?;
    let workflow = workflow_from_runtime_config(config);
    ensure_task_is_active(task, command_label(command), &WorkflowIndex::new(&workflow))?;
    let member = find_member(config, member_id)?;
    ensure_member_enabled(member)?;
    ensure_member_role_matches_task(config, task, member)?;
    Ok((task, member))
}

fn find_task<'a>(
    tasks: &'a [StaffTaskReadModel],
    task_id: &str,
) -> Result<&'a StaffTaskReadModel, String> {
    tasks
        .iter()
        .find(|task| task.task_id == task_id)
        .ok_or_else(|| format!("员工命令失败：任务 {task_id} 不存在"))
}

fn find_member<'a>(
    config: &'a StaffRuntimeConfig,
    member_id: &str,
) -> Result<&'a StaffMemberRuntime, String> {
    config
        .members
        .iter()
        .find(|member| member.id == member_id)
        .ok_or_else(|| format!("员工命令失败：员工 {member_id} 不存在"))
}

fn ensure_task_is_active(
    task: &StaffTaskReadModel,
    action_label: &str,
    workflow: &WorkflowIndex<'_>,
) -> Result<(), String> {
    if task_is_ended(task, workflow) {
        return Err(format!("员工命令失败：任务 {} 已结束，不能{}", task.task_id, action_label));
    }
    Ok(())
}

fn ensure_member_enabled(member: &StaffMemberRuntime) -> Result<(), String> {
    if !member.enabled {
        return Err(format!("员工命令失败：员工 {} 已停用", member.id));
    }
    Ok(())
}

fn ensure_member_role_matches_task(
    config: &StaffRuntimeConfig,
    task: &StaffTaskReadModel,
    member: &StaffMemberRuntime,
) -> Result<(), String> {
    let Some(stage) = config.stages.iter().find(|stage| stage.status_key == task.status_key) else {
        return Err(format!(
            "员工命令失败：任务 {} 的 {} 阶段没有可执行员工角色",
            task.task_id,
            task_label(task)
        ));
    };
    if !stage.enabled {
        return Err(format!(
            "员工命令失败：任务 {} 的 {} 阶段已停用",
            task.task_id,
            task_label(task)
        ));
    }
    let Some(required_role) = stage.role.as_deref() else {
        return Err(format!(
            "员工命令失败：任务 {} 的 {} 阶段没有可执行员工角色",
            task.task_id,
            task_label(task)
        ));
    };
    if member.role != required_role {
        return Err(format!(
            "员工命令失败：角色不匹配，任务 {} 的 {} 阶段需要 {}，但员工 {} 是 {}",
            task.task_id,
            task_label(task),
            required_role,
            member.id,
            member.role
        ));
    }
    Ok(())
}

fn ensure_member_capacity(member: &StaffMemberRuntime, task_id: &str) -> Result<(), String> {
    if member.current_task_ids.iter().any(|current_id| current_id == task_id) {
        return Ok(());
    }
    if member.current_task_ids.len() >= member.capacity as usize {
        return Err(format!("员工命令失败：员工 {} 容量已满", member.id));
    }
    Ok(())
}

fn start_action(
    task: &StaffTaskReadModel,
    member: &StaffMemberRuntime,
    config: &StaffRuntimeConfig,
    workflow: &WorkflowIndex<'_>,
    lifecycle_bridge: &LifecycleBridge<'_>,
) -> Result<StaffCommandActionDto, String> {
    if !workflow.status_enabled(&task.status_key) {
        return Err(format!("员工命令失败：任务 {} 当前阶段已停用", task.task_id));
    }
    if let Some(action) = stage_action(config, &task.status_key) {
        return stage_action_command_action(task, member, lifecycle_bridge, action);
    }

    match workflow.role(&task.status_key) {
        Some(TaskPoolWorkflowStatusRoleDto::Backlog | TaskPoolWorkflowStatusRoleDto::Failed) => {
            advance_status_command(task, lifecycle_bridge)
        }
        Some(TaskPoolWorkflowStatusRoleDto::Queued | TaskPoolWorkflowStatusRoleDto::Planning) => {
            Ok(StaffCommandActionDto::ExecuteTask { task_id: task.task_id.clone() })
        }
        Some(
            TaskPoolWorkflowStatusRoleDto::ReviewReady
            | TaskPoolWorkflowStatusRoleDto::Review
            | TaskPoolWorkflowStatusRoleDto::Merging,
        ) => Err(missing_stage_action(task)),
        Some(TaskPoolWorkflowStatusRoleDto::Executing) => {
            Err(format!("员工命令失败：任务 {} 已在执行中", task.task_id))
        }
        Some(TaskPoolWorkflowStatusRoleDto::Paused) => {
            Err(format!("员工命令失败：任务 {} 已暂停，请使用继续动作", task.task_id))
        }
        Some(
            TaskPoolWorkflowStatusRoleDto::Done
            | TaskPoolWorkflowStatusRoleDto::Archived
            | TaskPoolWorkflowStatusRoleDto::Custom,
        )
        | None => Err(unsupported_start_status(task)),
    }
}

fn stage_action<'a>(
    config: &'a StaffRuntimeConfig,
    status_key: &str,
) -> Option<&'a StaffStageActionRuntime> {
    config
        .actions
        .iter()
        .find(|action| action.status_key == status_key && !action.capability_id.trim().is_empty())
}

fn stage_action_command_action(
    task: &StaffTaskReadModel,
    member: &StaffMemberRuntime,
    lifecycle_bridge: &LifecycleBridge<'_>,
    action: &StaffStageActionRuntime,
) -> Result<StaffCommandActionDto, String> {
    match action.capability_id.trim() {
        ADVANCE_STATUS_CAPABILITY_ID => advance_status_command(task, lifecycle_bridge),
        EXECUTE_TASK_CAPABILITY_ID => {
            Ok(StaffCommandActionDto::ExecuteTask { task_id: task.task_id.clone() })
        }
        CODE_REVIEW_CAPABILITY_ID => {
            Ok(StaffCommandActionDto::RunCodeReview { task_id: task.task_id.clone() })
        }
        GIT_MERGE_CAPABILITY_ID => {
            Ok(StaffCommandActionDto::RunMerge { task_id: task.task_id.clone() })
        }
        SAVED_WORKFLOW_CAPABILITY_ID => {
            let Some(workflow_uuid) =
                action.workflow_uuid.as_deref().map(str::trim).filter(|value| !value.is_empty())
            else {
                return Err(format!(
                    "员工命令失败：任务 {} 的已保存 Workflow 阶段能力缺少 workflow_uuid",
                    task.task_id
                ));
            };
            Ok(StaffCommandActionDto::RunStageWorkflow {
                task_id: task.task_id.clone(),
                stage_id: action.stage_id.clone(),
                status_key: task.status_key.clone(),
                workflow_uuid: workflow_uuid.to_string(),
                member_id: member.id.clone(),
            })
        }
        MANUAL_CAPABILITY_ID => Err(format!(
            "员工命令失败：任务 {} 当前阶段配置为人工处理，不能通过开始命令自动执行",
            task.task_id
        )),
        capability_id => Err(format!(
            "员工命令失败：任务 {} 配置了不支持的阶段能力 {}",
            task.task_id, capability_id
        )),
    }
}

fn advance_status_command(
    task: &StaffTaskReadModel,
    lifecycle_bridge: &LifecycleBridge<'_>,
) -> Result<StaffCommandActionDto, String> {
    let projection =
        lifecycle_bridge.resolve_outcome(&task.status_key, "success").map_err(|_| {
            format!(
                "员工命令失败：任务 {} 当前是 {}，没有可推进的下一阶段",
                task.task_id,
                task_label(task)
            )
        })?;
    Ok(status_change_action_from_projection(task, projection))
}

fn status_change_action(
    task: &StaffTaskReadModel,
    to_status_key: &str,
    lifecycle_bridge: &LifecycleBridge<'_>,
) -> Result<StaffCommandActionDto, String> {
    let projection = lifecycle_bridge.resolve_status(to_status_key).map_err(|error| {
        format!(
            "员工命令失败：任务 {} 目标阶段 {} 无法映射运行状态：{}",
            task.task_id, to_status_key, error
        )
    })?;
    Ok(status_change_action_from_projection(task, projection))
}

fn status_change_action_from_projection(
    task: &StaffTaskReadModel,
    projection: LifecycleStatusProjection,
) -> StaffCommandActionDto {
    StaffCommandActionDto::ChangeTaskStatus {
        task_id: task.task_id.clone(),
        to_status: Some(agent_task_status_to_dto(projection.runtime_status)),
        to_status_key: projection.status_key,
    }
}

fn missing_stage_action(task: &StaffTaskReadModel) -> String {
    format!(
        "员工命令失败：任务 {} 当前是 {}，需要配置阶段能力后才能开始",
        task.task_id,
        task_label(task)
    )
}

fn unsupported_start_status(task: &StaffTaskReadModel) -> String {
    format!("员工命令失败：任务 {} 当前是 {}，不能开始", task.task_id, task_label(task))
}

fn pause_target_status(task: &StaffTaskReadModel, workflow: &WorkflowIndex<'_>) -> Option<String> {
    if !matches!(workflow.role(&task.status_key), Some(TaskPoolWorkflowStatusRoleDto::Executing)) {
        return None;
    }
    workflow.status_with_role(TaskPoolWorkflowStatusRoleDto::Paused)
}

fn resume_target_status(task: &StaffTaskReadModel, workflow: &WorkflowIndex<'_>) -> Option<String> {
    if !matches!(workflow.role(&task.status_key), Some(TaskPoolWorkflowStatusRoleDto::Paused)) {
        return None;
    }
    workflow
        .next_status(&task.status_key)
        .or_else(|| workflow.status_with_role(TaskPoolWorkflowStatusRoleDto::Queued))
        .or_else(|| workflow.status_with_role(TaskPoolWorkflowStatusRoleDto::Planning))
}

fn resume_target_status_key(workflow: &WorkflowIndex<'_>) -> Option<String> {
    workflow
        .status_with_role(TaskPoolWorkflowStatusRoleDto::Queued)
        .or_else(|| workflow.status_with_role(TaskPoolWorkflowStatusRoleDto::Planning))
}

fn cannot_pause_status(task: &StaffTaskReadModel) -> String {
    format!("员工命令失败：任务 {} 当前是 {}，不能暂停", task.task_id, task_label(task))
}

fn cannot_resume_status(task: &StaffTaskReadModel) -> String {
    format!("员工命令失败：任务 {} 当前是 {}，不能继续", task.task_id, task_label(task))
}

fn schedule_assigned_task(
    plan: &mut StaffAutoSchedulePlanDto,
    scheduled: &mut HashSet<String>,
    config: &StaffRuntimeConfig,
    stage_assignment: &StaffStageAssignmentRuntime,
    projection_assignment: Option<&StaffAssignmentDto>,
    task: &StaffTaskReadModel,
    member: &StaffMemberRuntime,
    now_ms: TimestampMs,
) {
    if let Some(reason) = member_skip_reason(config, task, member) {
        push_skip(plan, task, reason);
        return;
    }

    if task.status_key == "paused" {
        if projection_assignment.is_some_and(|assignment| assignment.actions.can_resume) {
            push_command(
                plan,
                scheduled,
                StaffCommandDto::ResumeTask {
                    command_id: auto_command_id(
                        AutoScheduleAction::Resume,
                        &task.task_id,
                        &member.id,
                        now_ms,
                    ),
                    task_id: task.task_id.clone(),
                    member_id: member.id.clone(),
                },
                &task.status_key,
                Some(orchestration_context(
                    config,
                    stage_assignment,
                    task,
                    member,
                    AutoScheduleAction::Resume,
                )),
            );
        } else {
            push_skip(plan, task, "task is paused but cannot be resumed by this stage");
        }
        return;
    }

    if projection_assignment.is_some_and(|assignment| assignment.actions.can_start) {
        push_command(
            plan,
            scheduled,
            StaffCommandDto::StartTask {
                command_id: auto_command_id(
                    AutoScheduleAction::Start,
                    &task.task_id,
                    &member.id,
                    now_ms,
                ),
                task_id: task.task_id.clone(),
                member_id: member.id.clone(),
            },
            &task.status_key,
            Some(orchestration_context(
                config,
                stage_assignment,
                task,
                member,
                AutoScheduleAction::Start,
            )),
        );
    } else {
        push_skip(plan, task, "task status is not auto-startable");
    }
}

fn select_stage_assignment_member(
    config: &StaffRuntimeConfig,
    assignment: &StaffStageAssignmentRuntime,
    task: &StaffTaskReadModel,
    member_loads: &HashMap<&str, usize>,
) -> Result<String, String> {
    let mut candidate_ids = Vec::new();
    let mut seen = HashSet::new();
    if let Some(member_id) = assignment.primary_member_id.as_deref().map(str::trim) {
        if !member_id.is_empty() && seen.insert(member_id) {
            candidate_ids.push(member_id);
        }
    }
    for member_id in &assignment.backup_member_ids {
        let member_id = member_id.trim();
        if !member_id.is_empty() && seen.insert(member_id) {
            candidate_ids.push(member_id);
        }
    }

    if candidate_ids.is_empty() {
        return Err(format!(
            "no primary or backup staff member configured for autonomous stage {}",
            assignment.status_key
        ));
    }

    let mut reasons = Vec::new();
    for member_id in candidate_ids {
        let Some(member) = config.members.iter().find(|member| member.id == member_id) else {
            reasons.push(format!("configured member {member_id} does not exist"));
            continue;
        };
        if let Some(reason) = member_skip_reason(config, task, member) {
            reasons.push(reason);
            continue;
        }
        let load = member_loads.get(member.id.as_str()).copied().unwrap_or_default();
        if load >= member.capacity as usize {
            reasons.push(format!("member {} capacity is full", member.id));
            continue;
        }
        return Ok(member.id.clone());
    }

    Err(reasons.join("; "))
}

fn member_skip_reason(
    config: &StaffRuntimeConfig,
    task: &StaffTaskReadModel,
    member: &StaffMemberRuntime,
) -> Option<String> {
    if !member.enabled {
        return Some(format!("member {} is disabled", member.id));
    }
    let Some(stage) = config.stages.iter().find(|stage| stage.status_key == task.status_key) else {
        return Some(format!("role is not configured for status {}", task.status_key));
    };
    if !stage.enabled {
        return Some(format!("status {} is disabled", task.status_key));
    }
    let Some(required_role) = stage.role.as_deref() else {
        return Some(format!("role is not configured for status {}", task.status_key));
    };
    if member.role != required_role {
        return Some(format!(
            "role mismatch for task {}: status {} requires {}, member {} is {}",
            task.task_id, task.status_key, required_role, member.id, member.role
        ));
    }
    None
}

fn current_member_for_task<'a>(
    config: &'a StaffRuntimeConfig,
    task_id: &str,
) -> Option<&'a StaffMemberRuntime> {
    config
        .members
        .iter()
        .find(|member| member.current_task_ids.iter().any(|current_id| current_id == task_id))
}

fn find_projection_assignment<'a>(
    projection: &'a StaffProjectionDto,
    task_id: &str,
) -> Option<&'a StaffAssignmentDto> {
    projection
        .lanes
        .iter()
        .flat_map(|lane| lane.assignments.iter())
        .find(|assignment| assignment.task_id == task_id)
}

fn push_command(
    plan: &mut StaffAutoSchedulePlanDto,
    scheduled: &mut HashSet<String>,
    command: StaffCommandDto,
    status_key: &str,
    orchestration: Option<StaffOrchestrationContextDto>,
) -> bool {
    let task_id = task_id(&command).unwrap_or_default().to_string();
    let member_id = member_id(&command).unwrap_or_default().to_string();
    let key = format!("{}:{task_id}:{member_id}", command_label(&command));
    if scheduled.insert(key) {
        if let Some(orchestration) = orchestration {
            plan.command_contexts.push(StaffAutoScheduleCommandContextDto {
                command_id: command_id(&command).to_string(),
                orchestration,
            });
        }
        plan.commands.push(command);
        return true;
    }

    plan.skipped.push(StaffAutoScheduleSkipDto {
        task_id,
        status_key: status_key.to_string(),
        reason: "duplicate auto schedule command".to_string(),
        orchestration,
    });
    false
}

fn push_skip(
    plan: &mut StaffAutoSchedulePlanDto,
    task: &StaffTaskReadModel,
    reason: impl Into<String>,
) {
    plan.skipped.push(StaffAutoScheduleSkipDto {
        task_id: task.task_id.clone(),
        status_key: task.status_key.clone(),
        reason: reason.into(),
        orchestration: None,
    });
}

fn auto_command_id(
    action: AutoScheduleAction,
    task_id: &str,
    member_id: &str,
    now_ms: TimestampMs,
) -> String {
    format!("staff-auto-{}-{task_id}-{member_id}-{}", action.key(), now_ms.0)
}

fn orchestration_context(
    config: &StaffRuntimeConfig,
    assignment: &StaffStageAssignmentRuntime,
    task: &StaffTaskReadModel,
    member: &StaffMemberRuntime,
    action: AutoScheduleAction,
) -> StaffOrchestrationContextDto {
    StaffOrchestrationContextDto {
        stage_id: assignment.stage_id.clone(),
        status_key: task.status_key.clone(),
        stage_label: task_label(task),
        member_level: member.level.clone(),
        member_level_label: staff_level_label(&member.level).to_string(),
        member_name: member.name.clone(),
        action_source: "自主调度".to_string(),
        target: format!("{} {} -> {}", action.target_label(), task.task_id, member.name),
        manager_member_names: manager_member_names(config, assignment),
    }
}

fn manager_member_names(
    config: &StaffRuntimeConfig,
    assignment: &StaffStageAssignmentRuntime,
) -> Vec<String> {
    assignment
        .manager_member_ids
        .iter()
        .filter_map(|member_id| {
            config
                .members
                .iter()
                .find(|member| member.id == *member_id)
                .map(|member| member.name.clone())
        })
        .collect()
}

fn staff_level_label(level: &str) -> &'static str {
    match level {
        "principal" => "主智能体",
        "manager" => "管理智能体",
        "executor" => "执行智能体",
        _ => "普通智能体",
    }
}

fn task_is_ended(task: &StaffTaskReadModel, workflow: &WorkflowIndex<'_>) -> bool {
    matches!(task.agent_status, Some(AgentTaskStatusDto::Completed | AgentTaskStatusDto::Archived))
        || matches!(
            workflow.role(&task.status_key),
            Some(TaskPoolWorkflowStatusRoleDto::Done | TaskPoolWorkflowStatusRoleDto::Archived)
        )
        || matches!(task.status_key.as_str(), "completed" | "archived")
}

fn task_is_blocked(task: &StaffTaskReadModel, workflow: &WorkflowIndex<'_>) -> bool {
    matches!(
        workflow.role(&task.status_key),
        Some(TaskPoolWorkflowStatusRoleDto::Failed | TaskPoolWorkflowStatusRoleDto::Paused)
    ) || matches!(task.status_key.as_str(), "failed" | "paused")
}

fn blocked_task_reason(task: &StaffTaskReadModel) -> String {
    if task.status_key == "failed" {
        return task
            .last_error
            .as_deref()
            .map(str::trim)
            .filter(|reason| !reason.is_empty())
            .map(|reason| format!("task failed: {reason}"))
            .unwrap_or_else(|| "task failed".to_string());
    }
    if task.status_key == "paused" {
        return task
            .pause_reason
            .as_deref()
            .map(str::trim)
            .filter(|reason| !reason.is_empty())
            .map(|reason| format!("task paused: {reason}"))
            .unwrap_or_else(|| "task paused".to_string());
    }
    "task is blocked".to_string()
}

fn workflow_from_runtime_config(config: &StaffRuntimeConfig) -> TaskPoolWorkflowSnapshotDto {
    TaskPoolWorkflowSnapshotDto {
        id: DEFAULT_WORKFLOW_ID.to_string(),
        label: "Staff Runtime Workflow".to_string(),
        statuses: config
            .stages
            .iter()
            .enumerate()
            .map(|(index, stage)| {
                let role = status_role_for_key(&stage.status_key)
                    .unwrap_or(TaskPoolWorkflowStatusRoleDto::Custom);
                let next = config.stages.get(index + 1).map(|next| next.status_key.clone());
                TaskPoolWorkflowStatusDto {
                    key: stage.status_key.clone(),
                    label: stage.label.clone(),
                    role,
                    enabled: stage.enabled,
                    terminal: stage.display_only,
                    transitions: retry_transition_for_failed_role(role, next.as_deref()),
                    next,
                }
            })
            .collect(),
    }
}

fn retry_transition_for_failed_role(
    role: TaskPoolWorkflowStatusRoleDto,
    next: Option<&str>,
) -> Vec<TaskPoolWorkflowTransitionDto> {
    match (role, next) {
        (TaskPoolWorkflowStatusRoleDto::Failed, Some(target)) => {
            vec![TaskPoolWorkflowTransitionDto {
                outcome: "retry".to_string(),
                label: Some("重试".to_string()),
                target: target.to_string(),
            }]
        }
        _ => Vec::new(),
    }
}

struct WorkflowIndex<'a> {
    workflow: &'a TaskPoolWorkflowSnapshotDto,
}

impl<'a> WorkflowIndex<'a> {
    fn new(workflow: &'a TaskPoolWorkflowSnapshotDto) -> Self {
        Self { workflow }
    }

    fn status(&self, status_key: &str) -> Option<&TaskPoolWorkflowStatusDto> {
        self.workflow.statuses.iter().find(|status| status.key == status_key)
    }

    fn role(&self, status_key: &str) -> Option<TaskPoolWorkflowStatusRoleDto> {
        match self.status(status_key) {
            Some(status) if status.enabled => Some(status.role),
            Some(_) => None,
            None => status_role_for_key(status_key),
        }
    }

    fn status_enabled(&self, status_key: &str) -> bool {
        self.status(status_key).is_some_and(|status| status.enabled)
    }

    fn next_status(&self, status_key: &str) -> Option<String> {
        let status = self.status(status_key)?;
        let target = status
            .transitions
            .iter()
            .find(|transition| matches!(transition.outcome.as_str(), "succeed" | "success"))
            .map(|transition| transition.target.clone())
            .or_else(|| status.next.clone())?;
        self.next_enabled_status(target)
    }

    fn status_with_role(&self, role: TaskPoolWorkflowStatusRoleDto) -> Option<String> {
        self.workflow
            .statuses
            .iter()
            .find(|status| status.enabled && status.role == role)
            .map(|status| status.key.clone())
    }

    fn next_enabled_status(&self, status_key: String) -> Option<String> {
        let mut current = status_key;
        let mut visited = HashSet::new();
        loop {
            let status = self.status(&current)?;
            if status.enabled {
                return Some(current);
            }
            if !visited.insert(current.clone()) {
                return None;
            }
            current = status
                .transitions
                .iter()
                .find(|transition| matches!(transition.outcome.as_str(), "succeed" | "success"))
                .map(|transition| transition.target.clone())
                .or_else(|| status.next.clone())?;
        }
    }
}

fn stage_action_outcome_key(outcome: AgentSchedulerStageActionOutcomeDto) -> &'static str {
    match outcome {
        AgentSchedulerStageActionOutcomeDto::Success => "success",
        AgentSchedulerStageActionOutcomeDto::Failure => "failure",
        AgentSchedulerStageActionOutcomeDto::Retry => "retry",
        AgentSchedulerStageActionOutcomeDto::Blocked => "blocked",
    }
}

fn stage_action_outcome_error(
    from_status_key: &str,
    outcome_key: &str,
    error: anyhow::Error,
) -> String {
    let message = error.to_string();
    if message.contains("has no transition") {
        format!(
            "stage action outcome {outcome_key} has no workflow transition from {from_status_key}"
        )
    } else if message.contains("not found in workflow") {
        format!("unknown current workflow status for stage action outcome: {from_status_key}")
    } else {
        message
    }
}

pub(crate) fn status_role_for_key(status_key: &str) -> Option<TaskPoolWorkflowStatusRoleDto> {
    match status_key.trim() {
        "pool" | "draft" | "backlog" => Some(TaskPoolWorkflowStatusRoleDto::Backlog),
        "pending" | "queued" => Some(TaskPoolWorkflowStatusRoleDto::Queued),
        "planning" | "assigned" => Some(TaskPoolWorkflowStatusRoleDto::Planning),
        "running" | "active" => Some(TaskPoolWorkflowStatusRoleDto::Executing),
        "failed" | "blocked" => Some(TaskPoolWorkflowStatusRoleDto::Failed),
        "paused" => Some(TaskPoolWorkflowStatusRoleDto::Paused),
        "code_complete" | "review_ready" | "submitted" => {
            Some(TaskPoolWorkflowStatusRoleDto::ReviewReady)
        }
        "code_review" | "review" => Some(TaskPoolWorkflowStatusRoleDto::Review),
        "pr_submitted" | "merging" => Some(TaskPoolWorkflowStatusRoleDto::Merging),
        "completed" | "done" => Some(TaskPoolWorkflowStatusRoleDto::Done),
        "archived" => Some(TaskPoolWorkflowStatusRoleDto::Archived),
        _ => None,
    }
}

fn agent_task_status_to_dto(status: AgentTaskStatus) -> AgentTaskStatusDto {
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

fn non_empty<'a>(value: &'a str, field: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    Ok(value)
}

fn task_label(task: &StaffTaskReadModel) -> String {
    task.status_label.clone().unwrap_or_else(|| task.status_key.clone())
}

fn command_id(command: &StaffCommandDto) -> &str {
    match command {
        StaffCommandDto::AssignTask { command_id, .. }
        | StaffCommandDto::TakeOverTask { command_id, .. }
        | StaffCommandDto::PauseTask { command_id, .. }
        | StaffCommandDto::ResumeTask { command_id, .. }
        | StaffCommandDto::StartTask { command_id, .. }
        | StaffCommandDto::OpenTaskLogs { command_id, .. }
        | StaffCommandDto::FilterByMember { command_id, .. } => command_id,
    }
}

fn task_id(command: &StaffCommandDto) -> Option<&str> {
    match command {
        StaffCommandDto::AssignTask { task_id, .. }
        | StaffCommandDto::TakeOverTask { task_id, .. }
        | StaffCommandDto::PauseTask { task_id, .. }
        | StaffCommandDto::ResumeTask { task_id, .. }
        | StaffCommandDto::StartTask { task_id, .. }
        | StaffCommandDto::OpenTaskLogs { task_id, .. } => Some(task_id),
        StaffCommandDto::FilterByMember { .. } => None,
    }
}

fn member_id(command: &StaffCommandDto) -> Option<&str> {
    match command {
        StaffCommandDto::AssignTask { member_id, .. }
        | StaffCommandDto::TakeOverTask { member_id, .. }
        | StaffCommandDto::PauseTask { member_id, .. }
        | StaffCommandDto::ResumeTask { member_id, .. }
        | StaffCommandDto::StartTask { member_id, .. }
        | StaffCommandDto::FilterByMember { member_id, .. } => Some(member_id),
        StaffCommandDto::OpenTaskLogs { .. } => None,
    }
}

fn command_label(command: &StaffCommandDto) -> &'static str {
    match command {
        StaffCommandDto::AssignTask { .. } => "派任务",
        StaffCommandDto::TakeOverTask { .. } => "接管任务",
        StaffCommandDto::PauseTask { .. } => "暂停任务",
        StaffCommandDto::ResumeTask { .. } => "继续任务",
        StaffCommandDto::StartTask { .. } => "开始任务",
        StaffCommandDto::OpenTaskLogs { .. } => "打开日志",
        StaffCommandDto::FilterByMember { .. } => "筛选成员",
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[cfg(test)]
#[path = "staff_tests.rs"]
mod staff_tests;

//! 任务看板员工模式的领域模型。

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use vw_config_types::staff::{
    StaffChannelBindingConfig, StaffConfig, StaffMemberTaskAssignmentConfig,
    StaffProjectStateConfig, StaffStageActionConfig, StaffStageActionInputsConfig,
    StaffStageAssignmentConfig, StaffWorkflowOrchestrationConfig, StaffWorkflowStageConfig,
};
use vw_gateway_client::vw_api_types::staff::{
    StaffCapabilityIoSummaryDto, StaffCapabilityKindDto, StaffProjectCapabilityCatalogRequest,
    StaffProjectCapabilityCatalogResponse, StaffProjectCapabilityDto, StaffStageCapabilityDto,
};
use vw_gateway_client::vw_api_types::task::TaskPoolWorkflowStatusRoleDto;

use crate::app::task::models::{TaskExecutorBackend, TaskStatus};
use crate::app::task::workflow::{
    DEFAULT_TASK_WORKFLOW_ID, StatusRole, TaskWorkflow, is_builtin_task_workflow_id,
    task_workflow_by_id,
};

use super::stage_capabilities::{
    ADVANCE_STATUS_CAPABILITY_ID, CODE_REVIEW_CAPABILITY_ID, EXECUTE_TASK_CAPABILITY_ID,
    GIT_MERGE_CAPABILITY_ID, SAVED_WORKFLOW_CAPABILITY_ID, stage_capability_registry,
};

const DEFAULT_WORKFLOW_VERSION: u32 = 1;

/// 任务看板员工岗位。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskBoardStaffRole {
    #[default]
    Orchestrator,
    Planner,
    Executor,
    Reviewer,
    Merger,
    Observer,
}

impl TaskBoardStaffRole {
    pub fn all() -> [TaskBoardStaffRole; 6] {
        [
            TaskBoardStaffRole::Orchestrator,
            TaskBoardStaffRole::Planner,
            TaskBoardStaffRole::Executor,
            TaskBoardStaffRole::Reviewer,
            TaskBoardStaffRole::Merger,
            TaskBoardStaffRole::Observer,
        ]
    }

    pub fn key(self) -> &'static str {
        match self {
            TaskBoardStaffRole::Orchestrator => "orchestrator",
            TaskBoardStaffRole::Planner => "planner",
            TaskBoardStaffRole::Executor => "executor",
            TaskBoardStaffRole::Reviewer => "reviewer",
            TaskBoardStaffRole::Merger => "merger",
            TaskBoardStaffRole::Observer => "observer",
        }
    }
}

/// 任务看板员工在阶段编排中的角色层级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskBoardStaffLevel {
    Principal,
    Manager,
    Executor,
    #[default]
    Member,
}

impl TaskBoardStaffLevel {
    pub fn for_role(role: TaskBoardStaffRole) -> Self {
        match role {
            TaskBoardStaffRole::Orchestrator => TaskBoardStaffLevel::Principal,
            TaskBoardStaffRole::Planner
            | TaskBoardStaffRole::Reviewer
            | TaskBoardStaffRole::Merger => TaskBoardStaffLevel::Manager,
            TaskBoardStaffRole::Executor => TaskBoardStaffLevel::Executor,
            TaskBoardStaffRole::Observer => TaskBoardStaffLevel::Member,
        }
    }
}

/// 任务看板员工在线状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskBoardStaffStatus {
    #[default]
    Online,
    Working,
    Blocked,
    Offline,
}

/// 任务看板员工名册成员。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskBoardStaffMember {
    pub id: String,
    pub name: String,
    pub role: TaskBoardStaffRole,
    pub level: TaskBoardStaffLevel,
    #[serde(default)]
    pub status: TaskBoardStaffStatus,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_capacity")]
    pub capacity: u32,
    #[serde(default)]
    pub executor_backend: Option<TaskExecutorBackend>,
    #[serde(default)]
    pub delegate_agent: Option<String>,
    #[serde(default)]
    pub acp_agent: Option<String>,
    #[serde(default)]
    pub ipc_agent_id: Option<String>,
    #[serde(default)]
    pub channel_bindings: Vec<StaffChannelBindingConfig>,
    #[serde(default)]
    pub current_task_ids: Vec<String>,
}

impl<'de> Deserialize<'de> for TaskBoardStaffMember {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct StaffMemberWire {
            id: String,
            name: String,
            role: TaskBoardStaffRole,
            #[serde(default)]
            level: Option<TaskBoardStaffLevel>,
            #[serde(default)]
            status: TaskBoardStaffStatus,
            #[serde(default = "default_enabled")]
            enabled: bool,
            #[serde(default = "default_capacity")]
            capacity: u32,
            #[serde(default)]
            executor_backend: Option<TaskExecutorBackend>,
            #[serde(default)]
            delegate_agent: Option<String>,
            #[serde(default)]
            acp_agent: Option<String>,
            #[serde(default)]
            ipc_agent_id: Option<String>,
            #[serde(default)]
            channel_bindings: Vec<StaffChannelBindingConfig>,
            #[serde(default)]
            current_task_ids: Vec<String>,
        }

        let wire = StaffMemberWire::deserialize(deserializer)?;
        Ok(Self {
            id: wire.id,
            name: wire.name,
            role: wire.role,
            level: wire.level.unwrap_or_else(|| TaskBoardStaffLevel::for_role(wire.role)),
            status: wire.status,
            enabled: wire.enabled,
            capacity: wire.capacity,
            executor_backend: wire.executor_backend,
            delegate_agent: wire.delegate_agent,
            acp_agent: wire.acp_agent,
            ipc_agent_id: wire.ipc_agent_id,
            channel_bindings: wire.channel_bindings,
            current_task_ids: wire.current_task_ids,
        })
    }
}

impl TaskBoardStaffMember {
    pub fn new(id: &str, name: &str, role: TaskBoardStaffRole) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            role,
            level: TaskBoardStaffLevel::for_role(role),
            status: TaskBoardStaffStatus::Online,
            enabled: true,
            capacity: 1,
            executor_backend: None,
            delegate_agent: None,
            acp_agent: None,
            ipc_agent_id: None,
            channel_bindings: Vec::new(),
            current_task_ids: Vec::new(),
        }
    }
}

/// 任务状态到显式成员编排的阶段分配。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBoardStageAssignment {
    pub status: TaskStatus,
    #[serde(default)]
    pub primary_member_id: Option<String>,
    #[serde(default)]
    pub backup_member_ids: Vec<String>,
    #[serde(default)]
    pub manager_member_ids: Vec<String>,
    #[serde(default)]
    pub autonomous: bool,
}

/// 任务状态到持久化 Workflow UUID 的绑定。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBoardStageWorkflowBinding {
    pub status: TaskStatus,
    pub workflow_uuid: String,
}

/// 任务状态到阶段能力的动作配置。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBoardStageAction {
    pub status: TaskStatus,
    pub capability_id: String,
    #[serde(default)]
    pub workflow_uuid: Option<String>,
    #[serde(default)]
    pub input_parameters: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskBoardStaffCapabilityKind {
    #[default]
    BuiltIn,
    SavedWorkflow,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBoardStaffCapabilitySummary {
    pub description: String,
    #[serde(default)]
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBoardStaffCapability {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub capability_type: TaskBoardStaffCapabilityKind,
    #[serde(default)]
    pub built_in: bool,
    #[serde(default)]
    pub supported_roles: Vec<TaskPoolWorkflowStatusRoleDto>,
    #[serde(default)]
    pub requires_workflow_uuid: bool,
    #[serde(default)]
    pub input_summary: TaskBoardStaffCapabilitySummary,
    #[serde(default)]
    pub result_summary: TaskBoardStaffCapabilitySummary,
    pub input_contract: String,
    pub result_semantics: String,
    pub permission_hint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskBoardStaffCapabilityDraftField {
    Id,
    Label,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskBoardStaffCapabilitySummaryKind {
    Input,
    Result,
}

impl TaskBoardStaffCapability {
    pub fn custom_saved_workflow(id: String, label: String) -> Self {
        Self {
            id,
            label,
            capability_type: TaskBoardStaffCapabilityKind::SavedWorkflow,
            built_in: false,
            supported_roles: Vec::new(),
            requires_workflow_uuid: true,
            input_summary: TaskBoardStaffCapabilitySummary::default(),
            result_summary: TaskBoardStaffCapabilitySummary::default(),
            input_contract: String::new(),
            result_semantics: String::new(),
            permission_hint: String::new(),
        }
    }

    pub fn editable(&self) -> bool {
        !self.built_in
    }
}

pub fn task_board_staff_capabilities_from_gateway_response(
    response: StaffProjectCapabilityCatalogResponse,
) -> Vec<TaskBoardStaffCapability> {
    response.capabilities.into_iter().map(task_board_staff_capability_from_dto).collect()
}

pub fn task_board_staff_capability_catalog_request(
    capabilities: &[TaskBoardStaffCapability],
) -> StaffProjectCapabilityCatalogRequest {
    StaffProjectCapabilityCatalogRequest {
        capabilities: capabilities
            .iter()
            .filter(|capability| capability.editable())
            .map(staff_project_capability_dto_from_task_board)
            .collect(),
    }
}

fn task_board_staff_capability_from_dto(
    capability: StaffStageCapabilityDto,
) -> TaskBoardStaffCapability {
    TaskBoardStaffCapability {
        id: capability.capability_id,
        label: capability.label,
        capability_type: task_board_capability_kind_from_dto(capability.capability_type),
        built_in: capability.built_in,
        supported_roles: capability.supported_roles,
        requires_workflow_uuid: capability.requires_workflow_uuid,
        input_summary: task_board_capability_summary_from_dto(capability.input_summary),
        result_summary: task_board_capability_summary_from_dto(capability.result_summary),
        input_contract: capability.input_contract,
        result_semantics: capability.result_semantics,
        permission_hint: capability.permission_hint,
    }
}

fn staff_project_capability_dto_from_task_board(
    capability: &TaskBoardStaffCapability,
) -> StaffProjectCapabilityDto {
    StaffProjectCapabilityDto {
        id: capability.id.trim().to_string(),
        label: capability.label.trim().to_string(),
        capability_type: capability_kind_dto_from_task_board(capability.capability_type),
        built_in: false,
        input_summary: capability_summary_dto_from_task_board(&capability.input_summary),
        result_summary: capability_summary_dto_from_task_board(&capability.result_summary),
    }
}

fn task_board_capability_kind_from_dto(
    kind: StaffCapabilityKindDto,
) -> TaskBoardStaffCapabilityKind {
    match kind {
        StaffCapabilityKindDto::BuiltIn => TaskBoardStaffCapabilityKind::BuiltIn,
        StaffCapabilityKindDto::SavedWorkflow => TaskBoardStaffCapabilityKind::SavedWorkflow,
    }
}

fn capability_kind_dto_from_task_board(
    kind: TaskBoardStaffCapabilityKind,
) -> StaffCapabilityKindDto {
    match kind {
        TaskBoardStaffCapabilityKind::BuiltIn => StaffCapabilityKindDto::BuiltIn,
        TaskBoardStaffCapabilityKind::SavedWorkflow => StaffCapabilityKindDto::SavedWorkflow,
    }
}

fn task_board_capability_summary_from_dto(
    summary: StaffCapabilityIoSummaryDto,
) -> TaskBoardStaffCapabilitySummary {
    TaskBoardStaffCapabilitySummary { description: summary.description, fields: summary.fields }
}

fn capability_summary_dto_from_task_board(
    summary: &TaskBoardStaffCapabilitySummary,
) -> StaffCapabilityIoSummaryDto {
    StaffCapabilityIoSummaryDto {
        description: summary.description.trim().to_string(),
        fields: summary
            .fields
            .iter()
            .map(|field| field.trim())
            .filter(|field| !field.is_empty())
            .map(ToString::to_string)
            .collect(),
    }
}

/// 任务状态到员工岗位的工作流阶段映射。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBoardWorkflowStage {
    pub status: TaskStatus,
    pub role: Option<TaskBoardStaffRole>,
    #[serde(default)]
    pub display_only: bool,
}

impl TaskBoardWorkflowStage {
    pub fn new(status: TaskStatus, role: TaskBoardStaffRole) -> Self {
        Self { status, role: Some(role), display_only: false }
    }

    pub fn display_only(status: TaskStatus) -> Self {
        Self { status, role: None, display_only: true }
    }
}

/// 项目级任务看板员工配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBoardStaffConfig {
    #[serde(default = "default_workflow_version")]
    pub workflow_version: u32,
    #[serde(default)]
    pub members: Vec<TaskBoardStaffMember>,
    #[serde(default)]
    pub workflow: Vec<TaskBoardWorkflowStage>,
    #[serde(default)]
    pub stage_assignments: Vec<TaskBoardStageAssignment>,
    #[serde(default)]
    pub stage_workflow_bindings: Vec<TaskBoardStageWorkflowBinding>,
    #[serde(default)]
    pub stage_actions: Vec<TaskBoardStageAction>,
}

impl Default for TaskBoardStaffConfig {
    fn default() -> Self {
        Self {
            workflow_version: DEFAULT_WORKFLOW_VERSION,
            members: default_staff_members(),
            workflow: default_workflow_stages(),
            stage_assignments: Vec::new(),
            stage_workflow_bindings: Vec::new(),
            stage_actions: Vec::new(),
        }
    }
}

impl TaskBoardStaffConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn normalize(self) -> Result<Self, String> {
        self.normalize_for_workflow_id(DEFAULT_TASK_WORKFLOW_ID)
    }

    pub fn normalize_for_workflow_id(mut self, workflow_id: &str) -> Result<Self, String> {
        let workflow = task_workflow_by_id(workflow_id.trim())
            .ok_or_else(|| format!("unknown task board workflow id: {}", workflow_id.trim()))?;
        if self.workflow_version == 0 {
            self.workflow_version = DEFAULT_WORKFLOW_VERSION;
        }
        if self.members.is_empty() {
            self.members = default_staff_members();
        }
        if self.workflow.is_empty() {
            self.workflow = workflow_stages_for_workflow(&workflow);
        }

        normalize_members(&mut self.members)?;
        normalize_workflow(&self.workflow, &workflow)?;
        let member_ids =
            self.members.iter().map(|member| member.id.clone()).collect::<HashSet<_>>();
        normalize_stage_assignments(&mut self.stage_assignments, &member_ids)?;
        normalize_stage_workflow_bindings(&mut self.stage_workflow_bindings)?;
        normalize_stage_actions(&mut self.stage_actions, &workflow)?;
        Ok(self)
    }

    pub fn retarget_for_workflow_id(mut self, workflow_id: &str) -> Result<Self, String> {
        let workflow = task_workflow_by_id(workflow_id.trim())
            .ok_or_else(|| format!("unknown task board workflow id: {}", workflow_id.trim()))?;
        self.workflow = workflow_stages_for_workflow(&workflow);
        prune_task_board_project_state_to_workflow(&mut self, &workflow);
        self.normalize_for_workflow_id(workflow.id())
    }

    pub fn role_for_status(&self, status: TaskStatus) -> Option<TaskBoardStaffRole> {
        self.workflow.iter().find(|stage| stage.status == status).and_then(|stage| stage.role)
    }
}

pub fn workflow_stages_for_workflow_id(
    workflow_id: &str,
) -> Result<Vec<TaskBoardWorkflowStage>, String> {
    let workflow = task_workflow_by_id(workflow_id.trim())
        .ok_or_else(|| format!("unknown task board workflow id: {}", workflow_id.trim()))?;
    Ok(workflow_stages_for_workflow(&workflow))
}

pub fn task_board_staff_config_from_staff_config(
    config: StaffConfig,
) -> Result<TaskBoardStaffConfig, String> {
    task_board_staff_config_from_staff_config_for_workflow_id(config, DEFAULT_TASK_WORKFLOW_ID)
}

pub fn task_board_staff_config_from_staff_config_for_workflow_id(
    mut config: StaffConfig,
    workflow_id: &str,
) -> Result<TaskBoardStaffConfig, String> {
    let selected_workflow = task_workflow_by_id(workflow_id.trim())
        .ok_or_else(|| format!("unknown task board workflow id: {}", workflow_id.trim()))?;
    config.workflow_stages = staff_workflow_stages_for_workflow(&selected_workflow);
    config.project_state =
        project_state_for_workflow_orchestration(&config, selected_workflow.id());
    let config = config.normalize()?;
    let mut stage_status_by_id = HashMap::new();
    let workflow = config
        .workflow_stages
        .iter()
        .map(|stage| {
            let status = parse_task_status_key(&stage.task_status, &stage.id)?;
            stage_status_by_id.insert(stage.id.clone(), status);
            Ok(TaskBoardWorkflowStage {
                status,
                role: parse_optional_staff_role(stage.role.as_deref(), &stage.id)?,
                display_only: stage.display_only,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let mut members = config
        .members
        .iter()
        .map(|member| {
            Ok(TaskBoardStaffMember {
                id: member.id.clone(),
                name: member.name.clone(),
                role: parse_staff_role_key(&member.role, &member.id)?,
                level: parse_staff_level_key(&member.level, &member.id)?,
                status: parse_staff_status_key(&member.status, &member.id)?,
                enabled: member.enabled,
                capacity: member.capacity,
                executor_backend: None,
                delegate_agent: member.delegate_agent.clone(),
                acp_agent: member.acp_agent.clone(),
                ipc_agent_id: member.ipc_agent_id.clone(),
                channel_bindings: member.channel_bindings.clone(),
                current_task_ids: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    apply_gateway_member_task_assignments(
        &mut members,
        &config.project_state.member_task_assignments,
    )?;

    let mut stage_assignments = config
        .project_state
        .stage_assignments
        .iter()
        .map(|assignment| {
            let status =
                stage_status_by_id.get(&assignment.stage_id).copied().ok_or_else(|| {
                    format!(
                        "unknown task board staff stage assignment stage_id: {}",
                        assignment.stage_id
                    )
                })?;
            Ok(TaskBoardStageAssignment {
                status,
                primary_member_id: assignment.primary_member_id.clone(),
                backup_member_ids: assignment.backup_member_ids.clone(),
                manager_member_ids: assignment.manager_member_ids.clone(),
                autonomous: assignment.autonomous,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut stage_actions = config
        .project_state
        .stage_actions
        .iter()
        .map(|action| {
            let status = stage_status_by_id.get(&action.stage_id).copied().ok_or_else(|| {
                format!("unknown task board stage action stage_id: {}", action.stage_id)
            })?;
            Ok(TaskBoardStageAction {
                status,
                capability_id: action.capability_id.clone(),
                workflow_uuid: action
                    .workflow_uuid
                    .clone()
                    .or_else(|| action.inputs.workflow_uuid.clone()),
                input_parameters: action.inputs.parameters.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    if !config.workflow_orchestrations.contains_key(selected_workflow.id())
        && is_builtin_task_workflow_id(selected_workflow.id())
    {
        stage_assignments = default_stage_assignments_for_workflow(&selected_workflow, &members);
        stage_actions = default_stage_actions_for_workflow(&selected_workflow);
    }
    let stage_workflow_bindings = stage_workflow_bindings_for_stage_actions(&stage_actions);

    TaskBoardStaffConfig {
        workflow_version: DEFAULT_WORKFLOW_VERSION,
        members,
        workflow,
        stage_assignments,
        stage_workflow_bindings,
        stage_actions,
    }
    .normalize_for_workflow_id(selected_workflow.id())
}

pub fn apply_task_board_staff_config_to_staff_config(
    staff_config: &mut StaffConfig,
    task_board_config: &TaskBoardStaffConfig,
) -> Result<(), String> {
    apply_task_board_staff_config_to_staff_config_for_workflow_id(
        staff_config,
        task_board_config,
        DEFAULT_TASK_WORKFLOW_ID,
    )
}

pub fn apply_task_board_staff_config_to_staff_config_for_workflow_id(
    staff_config: &mut StaffConfig,
    task_board_config: &TaskBoardStaffConfig,
    workflow_id: &str,
) -> Result<(), String> {
    let workflow = task_workflow_by_id(workflow_id.trim())
        .ok_or_else(|| format!("unknown task board workflow id: {}", workflow_id.trim()))?;
    let task_board_config = task_board_config.clone().retarget_for_workflow_id(workflow.id())?;
    let mut config = staff_config.clone();
    config.workflow_stages = staff_workflow_stages_for_workflow(&workflow);
    let capabilities = config
        .workflow_orchestrations
        .get(workflow.id())
        .map(|orchestration| orchestration.capabilities.clone())
        .unwrap_or_default();
    config.project_state =
        StaffProjectStateConfig { capabilities, ..StaffProjectStateConfig::default() };
    let mut config = config.normalize()?;
    let stage_id_by_status = gateway_stage_id_by_task_status(&config)?;

    config.project_state.stage_assignments = task_board_config
        .stage_assignments
        .iter()
        .map(|assignment| {
            let stage_id =
                stage_id_by_status.get(&assignment.status).cloned().ok_or_else(|| {
                    format!(
                        "missing gateway staff workflow stage for task status: {}",
                        assignment.status.to_string_key()
                    )
                })?;
            Ok(StaffStageAssignmentConfig {
                stage_id,
                primary_member_id: assignment.primary_member_id.clone(),
                backup_member_ids: assignment.backup_member_ids.clone(),
                manager_member_ids: assignment.manager_member_ids.clone(),
                autonomous: assignment.autonomous,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    config.project_state.member_task_assignments = task_board_config
        .members
        .iter()
        .filter(|member| !member.current_task_ids.is_empty())
        .map(|member| StaffMemberTaskAssignmentConfig {
            member_id: member.id.clone(),
            task_ids: member.current_task_ids.clone(),
        })
        .collect();
    config.project_state.stage_actions = task_board_config
        .stage_actions
        .iter()
        .map(|action| {
            let stage_id = stage_id_by_status.get(&action.status).cloned().ok_or_else(|| {
                format!(
                    "missing gateway staff stage action for task status: {}",
                    action.status.to_string_key()
                )
            })?;
            Ok(StaffStageActionConfig {
                stage_id,
                capability_id: action.capability_id.clone(),
                workflow_uuid: action.workflow_uuid.clone(),
                inputs: StaffStageActionInputsConfig {
                    workflow_uuid: action.workflow_uuid.clone(),
                    parameters: action.input_parameters.clone(),
                },
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    config.project_state.stage_workflow_bindings.clear();
    config.workflow_orchestrations.insert(
        workflow.id().to_string(),
        StaffWorkflowOrchestrationConfig {
            capabilities: config.project_state.capabilities.clone(),
            stage_assignments: config.project_state.stage_assignments.clone(),
            stage_workflow_bindings: Vec::new(),
            stage_actions: config.project_state.stage_actions.clone(),
        },
    );
    let members_by_id = task_board_config
        .members
        .iter()
        .map(|member| (member.id.as_str(), member))
        .collect::<HashMap<_, _>>();
    for member in &mut config.members {
        if let Some(task_board_member) = members_by_id.get(member.id.as_str()) {
            member.channel_bindings = task_board_member.channel_bindings.clone();
            member.ipc_agent_id = task_board_member.ipc_agent_id.clone();
        }
    }

    *staff_config = config.normalize()?;
    Ok(())
}

fn apply_gateway_member_task_assignments(
    members: &mut [TaskBoardStaffMember],
    assignments: &[StaffMemberTaskAssignmentConfig],
) -> Result<(), String> {
    for assignment in assignments {
        let member = members
            .iter_mut()
            .find(|member| member.id == assignment.member_id)
            .ok_or_else(|| {
                format!(
                    "unknown task board staff member task assignment member_id: {}",
                    assignment.member_id
                )
            })?;
        member.current_task_ids = assignment.task_ids.clone();
    }
    Ok(())
}

fn gateway_stage_id_by_task_status(
    config: &StaffConfig,
) -> Result<HashMap<TaskStatus, String>, String> {
    let mut stages = HashMap::new();
    for stage in &config.workflow_stages {
        let status = parse_task_status_key(&stage.task_status, &stage.id)?;
        stages.insert(status, stage.id.clone());
    }
    Ok(stages)
}

fn parse_task_status_key(value: &str, context: &str) -> Result<TaskStatus, String> {
    let value = value.trim();
    TaskStatus::parse_key(value).ok_or_else(|| {
        format!("unknown task board staff workflow task_status for {context}: {value}")
    })
}

fn parse_optional_staff_role(
    value: Option<&str>,
    context: &str,
) -> Result<Option<TaskBoardStaffRole>, String> {
    value.map(|value| parse_staff_role_key(value, context).map(Some)).unwrap_or(Ok(None))
}

fn parse_staff_role_key(value: &str, context: &str) -> Result<TaskBoardStaffRole, String> {
    match value.trim() {
        "orchestrator" => Ok(TaskBoardStaffRole::Orchestrator),
        "planner" => Ok(TaskBoardStaffRole::Planner),
        "executor" => Ok(TaskBoardStaffRole::Executor),
        "reviewer" => Ok(TaskBoardStaffRole::Reviewer),
        "merger" => Ok(TaskBoardStaffRole::Merger),
        "observer" | "member" => Ok(TaskBoardStaffRole::Observer),
        value => Err(format!("unknown task board staff role for {context}: {value}")),
    }
}

fn parse_staff_level_key(value: &str, context: &str) -> Result<TaskBoardStaffLevel, String> {
    match value.trim() {
        "principal" => Ok(TaskBoardStaffLevel::Principal),
        "manager" => Ok(TaskBoardStaffLevel::Manager),
        "executor" => Ok(TaskBoardStaffLevel::Executor),
        "member" => Ok(TaskBoardStaffLevel::Member),
        value => Err(format!("unknown task board staff level for {context}: {value}")),
    }
}

fn parse_staff_status_key(value: &str, context: &str) -> Result<TaskBoardStaffStatus, String> {
    match value.trim() {
        "online" | "available" => Ok(TaskBoardStaffStatus::Online),
        "working" => Ok(TaskBoardStaffStatus::Working),
        "blocked" => Ok(TaskBoardStaffStatus::Blocked),
        "offline" => Ok(TaskBoardStaffStatus::Offline),
        value => Err(format!("unknown task board staff status for {context}: {value}")),
    }
}

fn default_workflow_version() -> u32 {
    DEFAULT_WORKFLOW_VERSION
}

fn default_enabled() -> bool {
    true
}

fn default_capacity() -> u32 {
    1
}

fn default_staff_members() -> Vec<TaskBoardStaffMember> {
    vec![
        TaskBoardStaffMember::new(
            "staff-orchestrator",
            "Orchestrator",
            TaskBoardStaffRole::Orchestrator,
        ),
        TaskBoardStaffMember::new("staff-planner", "Planner", TaskBoardStaffRole::Planner),
        TaskBoardStaffMember::new("staff-executor", "Executor", TaskBoardStaffRole::Executor),
        TaskBoardStaffMember::new("staff-reviewer", "Reviewer", TaskBoardStaffRole::Reviewer),
        TaskBoardStaffMember::new("staff-merger", "Merger", TaskBoardStaffRole::Merger),
    ]
}

fn default_workflow_stages() -> Vec<TaskBoardWorkflowStage> {
    workflow_stages_for_workflow_id(DEFAULT_TASK_WORKFLOW_ID)
        .expect("default task workflow must be registered")
}

pub(crate) fn default_stage_actions_for_workflow(
    workflow: &TaskWorkflow,
) -> Vec<TaskBoardStageAction> {
    if !is_builtin_task_workflow_id(workflow.id()) {
        return Vec::new();
    }

    workflow
        .statuses()
        .iter()
        .filter_map(|definition| {
            let capability_id =
                default_stage_capability_for_role(workflow.id(), definition.role())?;
            Some(TaskBoardStageAction {
                status: definition.status,
                capability_id: capability_id.to_string(),
                workflow_uuid: None,
                input_parameters: BTreeMap::new(),
            })
        })
        .collect()
}

pub(crate) fn default_stage_assignments_for_workflow(
    workflow: &TaskWorkflow,
    members: &[TaskBoardStaffMember],
) -> Vec<TaskBoardStageAssignment> {
    if !is_builtin_task_workflow_id(workflow.id()) {
        return Vec::new();
    }

    workflow
        .statuses()
        .iter()
        .filter_map(|definition| {
            if !default_stage_should_autonomously_continue(definition.role()) {
                return None;
            }
            let role = staff_role_for_status_role(definition.role())?;
            let primary_member_id = default_member_id_for_role(members, role)?;
            Some(TaskBoardStageAssignment {
                status: definition.status,
                primary_member_id: Some(primary_member_id),
                backup_member_ids: Vec::new(),
                manager_member_ids: Vec::new(),
                autonomous: true,
            })
        })
        .collect()
}

fn default_stage_capability_for_role(
    workflow_id: &str,
    status_role: StatusRole,
) -> Option<&'static str> {
    match status_role {
        StatusRole::Backlog | StatusRole::Failed | StatusRole::ReviewReady => {
            Some(ADVANCE_STATUS_CAPABILITY_ID)
        }
        StatusRole::Queued | StatusRole::Planning => Some(EXECUTE_TASK_CAPABILITY_ID),
        StatusRole::Review if workflow_id == DEFAULT_TASK_WORKFLOW_ID => {
            Some(CODE_REVIEW_CAPABILITY_ID)
        }
        StatusRole::Merging if workflow_id == DEFAULT_TASK_WORKFLOW_ID => {
            Some(GIT_MERGE_CAPABILITY_ID)
        }
        StatusRole::Review | StatusRole::Merging => Some(ADVANCE_STATUS_CAPABILITY_ID),
        _ => None,
    }
}

fn default_stage_should_autonomously_continue(status_role: StatusRole) -> bool {
    matches!(
        status_role,
        StatusRole::Backlog
            | StatusRole::Queued
            | StatusRole::Planning
            | StatusRole::ReviewReady
            | StatusRole::Review
            | StatusRole::Merging
    )
}

fn default_member_id_for_role(
    members: &[TaskBoardStaffMember],
    role: TaskBoardStaffRole,
) -> Option<String> {
    members
        .iter()
        .find(|member| member.enabled && member.role == role)
        .or_else(|| members.iter().find(|member| member.role == role))
        .map(|member| member.id.clone())
}

fn stage_workflow_bindings_for_stage_actions(
    stage_actions: &[TaskBoardStageAction],
) -> Vec<TaskBoardStageWorkflowBinding> {
    stage_actions
        .iter()
        .filter(|action| action.capability_id == SAVED_WORKFLOW_CAPABILITY_ID)
        .filter_map(|action| {
            action.workflow_uuid.as_ref().map(|workflow_uuid| TaskBoardStageWorkflowBinding {
                status: action.status,
                workflow_uuid: workflow_uuid.clone(),
            })
        })
        .collect()
}

fn workflow_stages_for_workflow(workflow: &TaskWorkflow) -> Vec<TaskBoardWorkflowStage> {
    workflow
        .statuses()
        .iter()
        .map(|definition| workflow_stage_for_status_role(definition.status, definition.role()))
        .collect()
}

fn staff_workflow_stages_for_workflow(workflow: &TaskWorkflow) -> Vec<StaffWorkflowStageConfig> {
    workflow
        .statuses()
        .iter()
        .map(|definition| {
            let role =
                staff_role_for_status_role(definition.role()).map(|role| role.key().to_string());
            StaffWorkflowStageConfig {
                id: definition.key().to_string(),
                name: definition.label().to_string(),
                task_status: definition.key().to_string(),
                enabled: definition.enabled,
                display_only: !definition.enabled || role.is_none(),
                role,
            }
        })
        .collect()
}

fn prune_task_board_project_state_to_workflow(
    config: &mut TaskBoardStaffConfig,
    workflow: &TaskWorkflow,
) {
    let statuses =
        workflow.statuses().iter().map(|definition| definition.status).collect::<HashSet<_>>();
    config.stage_assignments.retain(|assignment| statuses.contains(&assignment.status));
    config.stage_workflow_bindings.retain(|binding| statuses.contains(&binding.status));
    config.stage_actions.retain(|action| statuses.contains(&action.status));
}

fn project_state_for_workflow_orchestration(
    config: &StaffConfig,
    workflow_id: &str,
) -> StaffProjectStateConfig {
    let member_task_assignments = config.project_state.member_task_assignments.clone();
    let mut project_state =
        StaffProjectStateConfig { member_task_assignments, ..StaffProjectStateConfig::default() };

    if let Some(orchestration) = config.workflow_orchestrations.get(workflow_id.trim()) {
        project_state.capabilities = orchestration.capabilities.clone();
        project_state.stage_assignments = orchestration.stage_assignments.clone();
        project_state.stage_actions = orchestration.stage_actions.clone();
    }

    project_state
}

fn workflow_stage_for_status_role(
    status: TaskStatus,
    status_role: StatusRole,
) -> TaskBoardWorkflowStage {
    match staff_role_for_status_role(status_role) {
        Some(role) => TaskBoardWorkflowStage::new(status, role),
        None => TaskBoardWorkflowStage::display_only(status),
    }
}

fn staff_role_for_status_role(status_role: StatusRole) -> Option<TaskBoardStaffRole> {
    match status_role {
        StatusRole::Backlog | StatusRole::Queued | StatusRole::Failed | StatusRole::Paused => {
            Some(TaskBoardStaffRole::Orchestrator)
        }
        StatusRole::Planning => Some(TaskBoardStaffRole::Planner),
        StatusRole::Executing => Some(TaskBoardStaffRole::Executor),
        StatusRole::ReviewReady | StatusRole::Review => Some(TaskBoardStaffRole::Reviewer),
        StatusRole::Merging => Some(TaskBoardStaffRole::Merger),
        StatusRole::Done | StatusRole::Archived | StatusRole::Custom => None,
    }
}

fn normalize_members(members: &mut [TaskBoardStaffMember]) -> Result<(), String> {
    let mut seen_ids = HashSet::new();
    for member in members {
        member.id = member.id.trim().to_string();
        if member.id.is_empty() {
            return Err("task board staff member id cannot be empty".to_string());
        }
        if !seen_ids.insert(member.id.clone()) {
            return Err(format!("duplicate task board staff member id: {}", member.id));
        }
        member.name = member.name.trim().to_string();
        if member.name.is_empty() {
            member.name = member.id.clone();
        }
        if member.capacity == 0 {
            return Err(format!("task board staff member {} capacity cannot be 0", member.id));
        }
        member.acp_agent = member
            .acp_agent
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        member.delegate_agent = member
            .delegate_agent
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        member.ipc_agent_id = member
            .ipc_agent_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        normalize_channel_bindings(&mut member.channel_bindings)?;
        normalize_task_ids(&mut member.current_task_ids);
    }
    Ok(())
}

fn normalize_stage_workflow_bindings(
    bindings: &mut [TaskBoardStageWorkflowBinding],
) -> Result<(), String> {
    let mut seen_statuses = HashSet::new();
    for binding in bindings {
        if !seen_statuses.insert(binding.status) {
            return Err(format!(
                "duplicate task board stage workflow binding status: {}",
                binding.status.to_string_key()
            ));
        }

        binding.workflow_uuid = binding.workflow_uuid.trim().to_string();
        if binding.workflow_uuid.is_empty() {
            return Err(format!(
                "task board stage workflow binding workflow_uuid for {} cannot be empty",
                binding.status.to_string_key()
            ));
        }
    }
    Ok(())
}

fn normalize_stage_actions(
    actions: &mut [TaskBoardStageAction],
    workflow: &TaskWorkflow,
) -> Result<(), String> {
    let workflow_statuses =
        workflow.statuses().iter().map(|definition| definition.status).collect::<HashSet<_>>();
    let mut seen_statuses = HashSet::new();
    for action in actions {
        if !seen_statuses.insert(action.status) {
            return Err(format!(
                "duplicate task board stage action status: {}",
                action.status.to_string_key()
            ));
        }
        if !workflow_statuses.contains(&action.status) {
            return Err(format!(
                "task board stage action status {} is not part of workflow {}",
                action.status.to_string_key(),
                workflow.id()
            ));
        }

        action.capability_id = action.capability_id.trim().to_string();
        if action.capability_id.is_empty() {
            return Err(format!(
                "task board stage action capability_id for {} cannot be empty",
                action.status.to_string_key()
            ));
        }
        normalize_stage_action_input_parameters(&mut action.input_parameters);
        action.workflow_uuid = action
            .workflow_uuid
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);

        if let Some(capability) = stage_capability_registry()
            .all()
            .iter()
            .find(|capability| capability.stable_id == action.capability_id)
        {
            if capability.requires_workflow_uuid {
                if action.workflow_uuid.is_none() {
                    return Err(format!(
                        "task board stage action workflow_uuid for {} cannot be empty",
                        action.status.to_string_key()
                    ));
                }
            } else if action.workflow_uuid.is_some() {
                return Err(format!(
                    "task board stage action workflow_uuid for {} must be empty",
                    action.status.to_string_key()
                ));
            }
        }
    }
    Ok(())
}

fn normalize_stage_action_input_parameters(parameters: &mut BTreeMap<String, Value>) {
    let mut normalized = BTreeMap::new();
    for (key, value) in std::mem::take(parameters) {
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = match value {
            Value::String(value) => Value::String(value.trim().to_string()),
            value => value,
        };
        normalized.insert(key.to_string(), value);
    }
    *parameters = normalized;
}

fn normalize_task_ids(task_ids: &mut Vec<String>) {
    let mut seen = HashSet::new();
    task_ids.retain_mut(|task_id| {
        *task_id = task_id.trim().to_string();
        !task_id.is_empty() && seen.insert(task_id.clone())
    });
}

fn normalize_channel_bindings(bindings: &mut [StaffChannelBindingConfig]) -> Result<(), String> {
    let mut seen_channel_types = HashSet::new();
    for binding in bindings {
        binding.channel_type = binding.channel_type.trim().to_ascii_lowercase();
        binding.instance_id = binding.instance_id.trim().to_string();
        validate_channel_binding(binding)?;
        if !seen_channel_types.insert(binding.channel_type.clone()) {
            return Err(format!(
                "duplicate task board staff channel binding type: {}",
                binding.channel_type
            ));
        }
    }
    Ok(())
}

fn validate_channel_binding(binding: &StaffChannelBindingConfig) -> Result<(), String> {
    let channel_type = binding.channel_type.trim();
    if !matches!(channel_type, "matrix" | "feishu" | "dingtalk") {
        return Err(format!("unsupported task board staff channel binding type: {channel_type}"));
    }
    let instance_id = binding.instance_id.trim();
    if instance_id.is_empty() {
        return Err(format!(
            "task board staff channel binding instance_id for {channel_type} cannot be empty"
        ));
    }
    if instance_id.contains([':', '/', '\\']) {
        return Err(format!(
            "task board staff channel binding instance_id for {channel_type} must not contain ':', '/' or '\\'"
        ));
    }
    Ok(())
}

fn normalize_workflow(
    workflow: &[TaskBoardWorkflowStage],
    task_workflow: &TaskWorkflow,
) -> Result<(), String> {
    let expected_by_status = task_workflow
        .statuses()
        .iter()
        .map(|definition| {
            (
                definition.status,
                workflow_stage_for_status_role(definition.status, definition.role()),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut seen_statuses = HashSet::new();
    for stage in workflow {
        if !seen_statuses.insert(stage.status) {
            return Err(format!(
                "duplicate task board workflow status: {}",
                stage.status.to_string_key()
            ));
        }
        if stage.display_only && stage.role.is_some() {
            return Err(format!(
                "display-only task board workflow stage cannot own status: {}",
                stage.status.to_string_key()
            ));
        }
        if !stage.display_only && stage.role.is_none() {
            return Err(format!(
                "task board workflow stage requires a staff role: {}",
                stage.status.to_string_key()
            ));
        }
        let Some(expected) = expected_by_status.get(&stage.status) else {
            return Err(format!(
                "task board workflow status {} is not part of workflow {}",
                stage.status.to_string_key(),
                task_workflow.id()
            ));
        };
        if stage.role != expected.role || stage.display_only != expected.display_only {
            return Err(format!(
                "task board workflow stage {} does not match workflow {} role mapping",
                stage.status.to_string_key(),
                task_workflow.id()
            ));
        }
    }

    for definition in task_workflow.statuses() {
        if !seen_statuses.contains(&definition.status) {
            return Err(format!("missing task board workflow status: {}", definition.key()));
        }
    }
    Ok(())
}

fn normalize_stage_assignments(
    assignments: &mut [TaskBoardStageAssignment],
    member_ids: &HashSet<String>,
) -> Result<(), String> {
    let mut seen_statuses = HashSet::new();
    for assignment in assignments {
        if !seen_statuses.insert(assignment.status) {
            return Err(format!(
                "duplicate task board stage assignment status: {}",
                assignment.status.to_string_key()
            ));
        }

        assignment.primary_member_id = assignment
            .primary_member_id
            .as_deref()
            .map(str::trim)
            .filter(|member_id| !member_id.is_empty())
            .map(ToString::to_string);
        normalize_member_id_list(&mut assignment.backup_member_ids);
        normalize_member_id_list(&mut assignment.manager_member_ids);

        let mut seen_members = HashSet::new();
        if let Some(member_id) = assignment.primary_member_id.as_deref() {
            validate_stage_assignment_member_id(member_id, member_ids, &mut seen_members)?;
        }
        for member_id in &assignment.backup_member_ids {
            validate_stage_assignment_member_id(member_id, member_ids, &mut seen_members)?;
        }
        for member_id in &assignment.manager_member_ids {
            validate_stage_assignment_member_id(member_id, member_ids, &mut seen_members)?;
        }
    }

    Ok(())
}

fn normalize_member_id_list(member_ids: &mut Vec<String>) {
    member_ids.retain_mut(|member_id| {
        *member_id = member_id.trim().to_string();
        !member_id.is_empty()
    });
}

fn validate_stage_assignment_member_id(
    member_id: &str,
    member_ids: &HashSet<String>,
    seen_members: &mut HashSet<String>,
) -> Result<(), String> {
    if !member_ids.contains(member_id) {
        return Err(format!("unknown task board stage assignment member id: {member_id}"));
    }
    if !seen_members.insert(member_id.to_string()) {
        return Err(format!("duplicate task board stage assignment member id: {member_id}"));
    }
    Ok(())
}

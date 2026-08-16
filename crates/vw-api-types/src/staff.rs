//! 员工任务流本地 IPC 消息 DTO。
//!
//! 本模块只定义桌面端调用本地网关时使用的消息边界，不承载发送逻辑。

use crate::agent_scheduler::{
    AgentSchedulerCommandRequest, AgentSchedulerCommandResponse, AgentTaskStatusDto,
};
use crate::common::TimestampMs;
use crate::id::ProjectId;
use crate::task::{TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusRoleDto};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const STAFF_IPC_MESSAGE_PAYLOAD_LIMIT: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffStageCapabilityDto {
    pub capability_id: String,
    pub label: String,
    #[serde(rename = "type", default)]
    pub capability_type: StaffCapabilityKindDto,
    #[serde(default)]
    pub built_in: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_roles: Vec<TaskPoolWorkflowStatusRoleDto>,
    #[serde(default)]
    pub requires_workflow_uuid: bool,
    #[serde(default)]
    pub input_summary: StaffCapabilityIoSummaryDto,
    #[serde(default)]
    pub result_summary: StaffCapabilityIoSummaryDto,
    pub input_contract: String,
    pub result_semantics: String,
    pub permission_hint: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaffCapabilityKindDto {
    #[default]
    BuiltIn,
    SavedWorkflow,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffCapabilityIoSummaryDto {
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffStageCapabilitiesResponse {
    pub capabilities: Vec<StaffStageCapabilityDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffProjectCapabilityDto {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub capability_type: StaffCapabilityKindDto,
    #[serde(default)]
    pub built_in: bool,
    #[serde(default)]
    pub input_summary: StaffCapabilityIoSummaryDto,
    #[serde(default)]
    pub result_summary: StaffCapabilityIoSummaryDto,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffProjectCapabilityCatalogRequest {
    #[serde(default)]
    pub capabilities: Vec<StaffProjectCapabilityDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffProjectCapabilityCatalogResponse {
    pub capabilities: Vec<StaffStageCapabilityDto>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffProjectStateDto {
    #[serde(default)]
    pub capabilities: Vec<StaffProjectCapabilityDto>,
    #[serde(default)]
    pub stage_assignments: Vec<StaffStageAssignmentDto>,
    #[serde(default)]
    pub stage_workflow_bindings: Vec<StaffStageWorkflowBindingDto>,
    #[serde(default)]
    pub stage_actions: Vec<StaffStageActionDto>,
    #[serde(default)]
    pub member_task_assignments: Vec<StaffMemberTaskAssignmentDto>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffWorkflowOrchestrationDto {
    #[serde(default)]
    pub capabilities: Vec<StaffProjectCapabilityDto>,
    #[serde(default)]
    pub stage_assignments: Vec<StaffStageAssignmentDto>,
    #[serde(default)]
    pub stage_workflow_bindings: Vec<StaffStageWorkflowBindingDto>,
    #[serde(default)]
    pub stage_actions: Vec<StaffStageActionDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffStageAssignmentDto {
    pub stage_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_member_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backup_member_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manager_member_ids: Vec<String>,
    #[serde(default)]
    pub autonomous: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffStageWorkflowBindingDto {
    pub stage_id: String,
    pub workflow_uuid: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffStageActionDto {
    pub stage_id: String,
    pub capability_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_uuid: Option<String>,
    #[serde(default, skip_serializing_if = "StaffStageActionInputsDto::is_empty")]
    pub inputs: StaffStageActionInputsDto,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffStageActionInputsDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_uuid: Option<String>,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub parameters: serde_json::Map<String, Value>,
}

impl StaffStageActionInputsDto {
    pub fn is_empty(&self) -> bool {
        self.workflow_uuid.is_none() && self.parameters.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffMemberTaskAssignmentDto {
    pub member_id: String,
    #[serde(default)]
    pub task_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffProjectOrchestrationRequest {
    pub workflow: TaskPoolWorkflowSnapshotDto,
    #[serde(default)]
    pub project_state: StaffProjectStateDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffProjectOrchestrationResponse {
    pub project_id: ProjectId,
    pub workflow: TaskPoolWorkflowSnapshotDto,
    pub project_state: StaffProjectStateDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffWorkflowOrchestrationRequest {
    pub workflow_id: String,
    #[serde(default)]
    pub orchestration: StaffWorkflowOrchestrationDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffWorkflowOrchestrationResponse {
    pub workflow_id: String,
    pub orchestration: StaffWorkflowOrchestrationDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffProjectionResponse {
    pub project_id: ProjectId,
    pub projection: StaffProjectionDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<StaffStageCapabilityDto>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffProjectionDto {
    pub lanes: Vec<StaffLaneDto>,
    pub member_loads: Vec<StaffMemberLoadDto>,
    pub unassigned: Vec<StaffAssignmentDto>,
    pub total_task_count: usize,
    pub blocked_task_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffLaneDto {
    pub stage_id: String,
    pub status_key: String,
    pub status_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default)]
    pub display_only: bool,
    pub assignments: Vec<StaffAssignmentDto>,
    pub task_count: usize,
    pub blocked_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffAssignmentDto {
    pub task_id: String,
    pub status_key: String,
    pub status_label: String,
    pub task_description: String,
    pub priority: u32,
    pub order_no: i64,
    pub updated_at_ms: TimestampMs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_role: Option<String>,
    #[serde(default)]
    pub display_only: bool,
    #[serde(default)]
    pub blocked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<String>,
    pub actions: StaffActionAvailabilityDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffMemberLoadDto {
    pub member_id: String,
    pub member_name: String,
    pub role: String,
    pub level: String,
    pub status: String,
    pub enabled: bool,
    pub capacity: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegate_agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp_agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ipc_agent_id: Option<String>,
    pub task_ids: Vec<String>,
    pub running_task_ids: Vec<String>,
    pub blocked_task_ids: Vec<String>,
    pub load_count: usize,
    pub over_capacity: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffActionAvailabilityDto {
    pub can_assign: bool,
    pub can_start: bool,
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_open_logs: bool,
    pub can_filter_member: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StaffCommandDto {
    AssignTask { command_id: String, task_id: String, member_id: String },
    TakeOverTask { command_id: String, task_id: String, member_id: String },
    PauseTask { command_id: String, task_id: String, member_id: String },
    ResumeTask { command_id: String, task_id: String, member_id: String },
    StartTask { command_id: String, task_id: String, member_id: String },
    OpenTaskLogs { command_id: String, task_id: String },
    FilterByMember { command_id: String, member_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StaffCommandActionDto {
    AssignTask {
        task_id: String,
        member_id: String,
    },
    ChangeTaskStatus {
        task_id: String,
        to_status_key: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to_status: Option<AgentTaskStatusDto>,
    },
    ExecuteTask {
        task_id: String,
    },
    RunCodeReview {
        task_id: String,
    },
    RunMerge {
        task_id: String,
    },
    RunStageWorkflow {
        task_id: String,
        stage_id: String,
        status_key: String,
        workflow_uuid: String,
        member_id: String,
    },
    ViewTaskLogs {
        task_id: String,
    },
    FilterByMember {
        member_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffCommandRequest {
    pub command: StaffCommandDto,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffCommandResponse {
    pub accepted: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<StaffCommandActionDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler_command: Option<AgentSchedulerCommandRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler_result: Option<AgentSchedulerCommandResponse>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffAutoSchedulePlanRequest {
    #[serde(default)]
    pub running_task_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub now_ms: Option<TimestampMs>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffAutoSchedulePlanResponse {
    pub project_id: ProjectId,
    pub plan: StaffAutoSchedulePlanDto,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffAutoSchedulePlanDto {
    pub commands: Vec<StaffCommandDto>,
    pub command_contexts: Vec<StaffAutoScheduleCommandContextDto>,
    pub skipped: Vec<StaffAutoScheduleSkipDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffAutoScheduleCommandContextDto {
    pub command_id: String,
    pub orchestration: StaffOrchestrationContextDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffAutoScheduleSkipDto {
    pub task_id: String,
    pub status_key: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orchestration: Option<StaffOrchestrationContextDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffOrchestrationContextDto {
    pub stage_id: String,
    pub status_key: String,
    pub stage_label: String,
    pub member_level: String,
    pub member_level_label: String,
    pub member_name: String,
    pub action_source: String,
    pub target: String,
    pub manager_member_names: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffCommandExecutionMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

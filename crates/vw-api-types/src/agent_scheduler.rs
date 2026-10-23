//! Agent Scheduler API DTO。
//!
//! 本模块只定义调度器边界上的中性序列化协议，用于表达任务、运行 lease、
//! Worker/Team 能力、资源 lease、事件与 outbox 消息；不承载执行逻辑。

use crate::chat::ReasoningEffortDto;
use crate::common::TimestampMs;
use crate::id::{IdValidationError, ProjectId, ProjectScope, TaskId};
use crate::task::{TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusRoleDto};
use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::Value;
use std::collections::BTreeMap;

fn deserialize_optional_reasoning_effort<'de, D>(
    deserializer: D,
) -> Result<Option<ReasoningEffortDto>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    value
        .map(|value| match value.as_str() {
            "none" => Ok(ReasoningEffortDto::None),
            "minimal" => Ok(ReasoningEffortDto::Minimal),
            "low" => Ok(ReasoningEffortDto::Low),
            "medium" => Ok(ReasoningEffortDto::Medium),
            "high" => Ok(ReasoningEffortDto::High),
            "xhigh" => Ok(ReasoningEffortDto::Xhigh),
            "max" => Ok(ReasoningEffortDto::Max),
            _ => Err(de::Error::custom(format!("invalid reasoning_effort value: {value}"))),
        })
        .transpose()
}

/// Agent Scheduler 按项目隔离的强类型作用域。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentSchedulerScope(ProjectScope);

/// Agent Scheduler 网关资源使用的显式项目作用域。
pub type AgentSchedulerScopeDto = AgentSchedulerScope;

impl AgentSchedulerScope {
    pub fn new(project_scope: ProjectScope) -> Self {
        Self(project_scope)
    }

    /// 使用平台默认租户构造项目调度作用域。
    pub fn for_default_tenant(project_id: ProjectId) -> Result<Self, IdValidationError> {
        ProjectScope::for_default_tenant(project_id).map(Self)
    }

    pub fn project_scope(&self) -> &ProjectScope {
        &self.0
    }
}

/// Agent Scheduler 任务类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTaskKindDto {
    /// 有明确完成状态的一次性任务。
    Finite,
    /// 按计划重复触发的长期任务。
    Infinite,
    /// 由 Team Leader 拆解并按 DAG 执行的团队任务。
    DependencyGraph,
}

/// Agent Scheduler 任务状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTaskStatusDto {
    Draft,
    Queued,
    Assigned,
    Dispatched,
    Acknowledged,
    Active,
    Running,
    Submitted,
    Revision,
    Blocked,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Archived,
}

/// Team DAG 中的子任务节点。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskDependencyNodeDto {
    pub task_id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
    pub status: AgentTaskStatusDto,
}

/// Agent Scheduler 任务快照。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentTaskDto {
    pub id: String,
    pub title: String,
    pub kind: AgentTaskKindDto,
    pub status: AgentTaskStatusDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_room_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegated_to_team: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_scheduled_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_executed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dag: Vec<TaskDependencyNodeDto>,
    pub created_at_ms: TimestampMs,
    pub updated_at_ms: TimestampMs,
    #[serde(default)]
    pub metadata: Value,
}

/// Agent Scheduler 任务创建请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSchedulerCreateTaskRequest {
    pub task_id: String,
    pub title: String,
    pub kind: AgentTaskKindDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<AgentTaskStatusDto>,
    pub spec_uri: String,
    pub meta_uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_room_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegated_to_team: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_scheduled_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_executed_at: Option<String>,
    #[serde(default)]
    pub metadata: Value,
    pub created_at_ms: TimestampMs,
}

/// Agent Scheduler 任务创建响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSchedulerCreateTaskResponse {
    pub task_id: String,
    pub task: AgentTaskDto,
}

/// Agent Scheduler 有向成功依赖边，从前置任务指向后继任务。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskGraphEdgeDto {
    pub predecessor_task_id: String,
    pub successor_task_id: String,
}

/// Agent Scheduler 项目任务图的完整替换请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerReplaceTaskGraphRequest {
    pub edges: Vec<AgentSchedulerTaskGraphEdgeDto>,
}

/// 任务在当前成功依赖图中的派生就绪原因。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSchedulerTaskGraphReadinessDto {
    Ready,
    WaitingDependencies,
    BlockedByDependency,
    NotQueued,
}

/// 项目任务图快照中的单个任务及其派生就绪性。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskGraphTaskDto {
    pub task_id: String,
    pub status: AgentTaskStatusDto,
    pub readiness: AgentSchedulerTaskGraphReadinessDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waiting_task_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocking_task_ids: Vec<String>,
}

/// Agent Scheduler 项目任务图及实时就绪性快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskGraphResponse {
    pub project_id: ProjectId,
    pub edges: Vec<AgentSchedulerTaskGraphEdgeDto>,
    pub tasks: Vec<AgentSchedulerTaskGraphTaskDto>,
}

/// Agent Scheduler 任务运行状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunStatusDto {
    Pending,
    Leased,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Stale,
}

/// Agent Scheduler 任务运行与执行 lease。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRunDto {
    pub id: String,
    pub task_id: String,
    pub status: AgentRunStatusDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_expires_at_ms: Option<TimestampMs>,
    pub attempt: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<TimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at_ms: Option<TimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_heartbeat_at_ms: Option<TimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Agent 能力类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCapabilityKindDto {
    Skill,
    RemoteSkill,
    McpServer,
    Model,
    Runtime,
    Tool,
}

/// Agent 能力声明。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentCapabilityDto {
    pub kind: AgentCapabilityKindDto,
    pub name: String,
    #[serde(default)]
    pub metadata: Value,
}

/// 资源 lease 状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentResourceLeaseStatusDto {
    Active,
    Released,
    Expired,
    Revoked,
}

/// Agent Scheduler 资源 lease。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentResourceLeaseDto {
    pub id: String,
    pub resource_kind: String,
    pub resource_id: String,
    pub holder_id: String,
    pub status: AgentResourceLeaseStatusDto,
    pub acquired_at_ms: TimestampMs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_ms: Option<TimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released_at_ms: Option<TimestampMs>,
}

/// Agent Scheduler 事件类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventKindDto {
    TaskCreated,
    ImportedLegacyTask,
    TaskAssigned,
    WorkerSelected,
    WorkerAcknowledged,
    TaskStarted,
    TaskCompleted,
    TaskFailed,
    TaskBlocked,
    TaskPaused,
    TaskResumed,
    TaskCancelled,
    TaskArchived,
    RunLeased,
    RunHeartbeat,
    RunStarted,
    RunSubmitted,
    RunCompleted,
    RunFailed,
    RunRecovered,
    InfiniteScheduled,
    InfiniteExecuted,
    TeamNodeQueued,
    TeamNodeDispatched,
    TeamNodeCompleted,
    ResourceLeased,
    ResourceReleased,
    OutboxQueued,
    OutboxDelivered,
    OutboxFailed,
    HumanIntervened,
}

/// Agent Scheduler 事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentEventDto {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub kind: AgentEventKindDto,
    pub message: String,
    #[serde(default)]
    pub payload: Value,
    pub created_at_ms: TimestampMs,
}

/// Outbox 消息状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentOutboxMessageStatusDto {
    Pending,
    Sending,
    Delivered,
    Failed,
    Cancelled,
}

/// Agent Scheduler outbox 消息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentOutboxMessageDto {
    pub id: String,
    pub target_kind: String,
    pub target_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient_id: Option<String>,
    pub body: String,
    pub status: AgentOutboxMessageStatusDto,
    pub idempotency_key: String,
    pub attempts: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_attempt_at_ms: Option<TimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivered_at_ms: Option<TimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default)]
    pub payload: Value,
    pub created_at_ms: TimestampMs,
}

/// Agent Scheduler 桌面任务板命令类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSchedulerCommandKindDto {
    Start,
    Retry,
    Pause,
    Resume,
    Cancel,
    Archive,
    Promote,
    Merge,
    Move,
    UpdateMetadata,
    SoftDelete,
    Restore,
    ImportLegacyTask,
}

/// Agent Scheduler 分发模式。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSchedulerDispatchModeDto {
    /// 本地 gateway runner 直接执行任务。
    #[default]
    Local,
    /// 由本地运行时自动选择执行时机。
    Auto,
}

/// Agent Scheduler 桌面任务板命令请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerCommandRequest {
    pub task_id: TaskId,
    pub command: AgentSchedulerCommandKindDto,
    #[serde(default, skip_serializing_if = "is_default_dispatch_mode")]
    pub dispatch_mode: AgentSchedulerDispatchModeDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrent: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "AgentSchedulerCommandPayloadDto::is_empty")]
    pub payload: AgentSchedulerCommandPayloadDto,
}

/// Agent Scheduler 桌面任务板命令的可选载荷。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerCommandPayloadDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_task: Option<AgentSchedulerMoveTaskPayloadDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_update: Option<AgentSchedulerTaskMetadataPatchDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_import: Option<AgentSchedulerLegacyTaskImportDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<Value>,
}

impl AgentSchedulerCommandPayloadDto {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// 移动任务到目标任务板状态的 command 载荷。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerMoveTaskPayloadDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_status_key: Option<String>,
    pub to_status_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_status: Option<AgentTaskStatusDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_no: Option<i64>,
}

/// 任务元数据与任务板字段 patch。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskMetadataPatchDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<AgentTaskStatusDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_no: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp_agent: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_reasoning_effort"
    )]
    pub reasoning_effort: Option<ReasoningEffortDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pause_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_source_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_target_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_worktree_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_worktree_slot_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_worktree_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_role: Option<TaskPoolWorkflowStatusRoleDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_merge: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metadata_remove_keys: Vec<String>,
}

/// 旧桌面任务导入项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerLegacyTaskImportDto {
    pub task_id: TaskId,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<AgentTaskStatusDto>,
    #[serde(default)]
    pub priority: u32,
    #[serde(default)]
    pub order_no: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp_agent: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_reasoning_effort"
    )]
    pub reasoning_effort: Option<ReasoningEffortDto>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub metadata: Value,
    pub created_at_ms: TimestampMs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at_ms: Option<TimestampMs>,
}

/// Structured inputs for a stage action command or projection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerStageActionInputsDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_uuid: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, Value>,
}

impl AgentSchedulerStageActionInputsDto {
    pub fn is_empty(&self) -> bool {
        self.workflow_uuid.is_none() && self.parameters.is_empty()
    }
}

/// Stage action result kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSchedulerStageActionOutcomeDto {
    Success,
    Failure,
    Retry,
    Blocked,
}

/// Agent Scheduler 任务工作区上下文。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerWorkspaceContextDto {
    pub slot_id: String,
    pub path: String,
    pub branch: String,
}

/// Agent Scheduler 桌面任务板任务快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskBoardTaskDto {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub status_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<AgentTaskStatusDto>,
    pub priority: u32,
    pub order_no: i64,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp_agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffortDto>,
    pub prompt: String,
    pub retry_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_stage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<AgentSchedulerWorkspaceContextDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pause_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_source_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_target_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_worktree_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_worktree_slot_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_worktree_branch: Option<String>,
    #[serde(default, skip_serializing_if = "AgentSchedulerTaskBoardTaskReadModelDto::is_empty")]
    pub read_model: AgentSchedulerTaskBoardTaskReadModelDto,
    pub updated_at_ms: TimestampMs,
}

/// 单个任务面向薄客户端的展示投影。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskBoardTaskReadModelDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_role: Option<TaskPoolWorkflowStatusRoleDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_label: Option<String>,
}

impl AgentSchedulerTaskBoardTaskReadModelDto {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// Agent Scheduler 桌面任务板状态响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskBoardStatusResponse {
    pub project_id: ProjectId,
    #[serde(
        default,
        skip_serializing_if = "AgentSchedulerTaskBoardSnapshotReadModelDto::is_empty"
    )]
    pub read_model: AgentSchedulerTaskBoardSnapshotReadModelDto,
    pub tasks: Vec<AgentSchedulerTaskBoardTaskDto>,
}

/// 任务板快照级展示投影。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskBoardSnapshotReadModelDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<TaskPoolWorkflowSnapshotDto>,
}

impl AgentSchedulerTaskBoardSnapshotReadModelDto {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// Agent Scheduler 项目 workflow 快照响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskBoardWorkflowResponse {
    pub project_id: ProjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<TaskPoolWorkflowSnapshotDto>,
}

/// Agent Scheduler 桌面任务板事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskBoardEventDto {
    pub cursor: i64,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_stage: Option<String>,
    pub kind: AgentEventKindDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_status_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_status: Option<AgentTaskStatusDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_status_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_status: Option<AgentTaskStatusDto>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<AgentSchedulerWorkspaceContextDto>,
    #[serde(default)]
    pub payload: Value,
    pub created_at_ms: TimestampMs,
}

/// Agent Scheduler 桌面任务板事件响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskBoardEventsResponse {
    pub cursor: i64,
    pub events: Vec<AgentSchedulerTaskBoardEventDto>,
}

/// Agent Scheduler 桌面任务板日志条目。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskLogDto {
    pub task_id: TaskId,
    pub seq_no: i64,
    pub timestamp_ms: TimestampMs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_status_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_from: Option<AgentTaskStatusDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_status_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_to: Option<AgentTaskStatusDto>,
    pub message: String,
}

/// Agent Scheduler 桌面任务板日志响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskLogsResponse {
    pub task_id: TaskId,
    pub logs: Vec<AgentSchedulerTaskLogDto>,
}

/// Agent Scheduler 桌面任务板产物。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskArtifactDto {
    pub task_id: TaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub kind: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default)]
    pub payload: Value,
    pub created_at_ms: TimestampMs,
}

/// Agent Scheduler 桌面任务板产物响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskArtifactsResponse {
    pub task_id: TaskId,
    pub artifacts: Vec<AgentSchedulerTaskArtifactDto>,
}

/// Agent Scheduler 桌面任务板命令响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerCommandResponse {
    pub accepted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<AgentSchedulerTaskBoardTaskDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<AgentSchedulerTaskBoardEventDto>,
}

/// Agent Scheduler 任务板批量命令请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerBulkCommandRequest {
    pub commands: Vec<AgentSchedulerCommandRequest>,
    #[serde(default)]
    pub stop_on_error: bool,
}

/// Agent Scheduler 任务板批量命令响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerBulkCommandResponse {
    pub accepted: u32,
    pub rejected: u32,
    pub results: Vec<AgentSchedulerCommandResponse>,
}

/// Agent Scheduler 任务元数据更新请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskMetadataUpdateRequest {
    pub patch: AgentSchedulerTaskMetadataPatchDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Agent Scheduler 任务元数据更新响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskMetadataUpdateResponse {
    pub task: AgentSchedulerTaskBoardTaskDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<AgentSchedulerTaskBoardEventDto>,
}

/// Agent Scheduler 旧任务导入请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerLegacyImportRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<TaskPoolWorkflowSnapshotDto>,
    pub tasks: Vec<AgentSchedulerLegacyTaskImportDto>,
    #[serde(default)]
    pub dry_run: bool,
}

/// Agent Scheduler 旧任务导入响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerLegacyImportResponse {
    pub imported: u32,
    pub skipped: u32,
    #[serde(default)]
    pub tasks: Vec<AgentSchedulerTaskBoardTaskDto>,
    #[serde(default)]
    pub events: Vec<AgentSchedulerTaskBoardEventDto>,
}

/// Agent Scheduler 唤醒请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerWakeRequest {
    pub now_ms: TimestampMs,
    #[serde(default, skip_serializing_if = "is_default_dispatch_mode")]
    pub dispatch_mode: AgentSchedulerDispatchModeDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrent: Option<u32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub auto_promote_pool_tasks: bool,
}

fn is_default_dispatch_mode(mode: &AgentSchedulerDispatchModeDto) -> bool {
    *mode == AgentSchedulerDispatchModeDto::default()
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Agent Scheduler 唤醒响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerWakeResponse {
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

/// Agent Scheduler outbox drain 请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerOutboxDrainRequest {
    pub now_ms: TimestampMs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// Agent Scheduler outbox drain 响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSchedulerOutboxDrainResponse {
    pub claimed: usize,
    pub messages: Vec<AgentOutboxMessageDto>,
}

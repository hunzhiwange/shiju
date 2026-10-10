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
    TeamDag,
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
pub struct TeamTaskDagNodeDto {
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
    pub dag: Vec<TeamTaskDagNodeDto>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<String>,
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

/// Worker 运行状态或调度可用性。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentWorkerStatusDto {
    Pending,
    Idle,
    Busy,
    Running,
    Sleeping,
    Stopped,
    Failed,
    Unavailable,
}

/// Agent Worker 快照。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentWorkerDto {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployment: Option<String>,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_provider: Option<String>,
    pub status: AgentWorkerStatusDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desired_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matrix_user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idle_since_ms: Option<TimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_at_ms: Option<TimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_heartbeat_at: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<AgentCapabilityDto>,
    pub updated_at_ms: TimestampMs,
}

/// Agent Worker 写入/更新请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSchedulerUpsertWorkerRequest {
    pub worker_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<AgentWorkerStatusDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desired_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matrix_user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idle_since_ms: Option<TimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_at_ms: Option<TimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_heartbeat_at: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<AgentCapabilityDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at_ms: Option<TimestampMs>,
}

/// Agent Worker 写入/更新响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSchedulerUpsertWorkerResponse {
    pub worker: AgentWorkerDto,
}

/// Agent Worker 列表响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSchedulerWorkersResponse {
    pub workers: Vec<AgentWorkerDto>,
}

/// Matrix/remote Worker 对一次 run 的生命周期上报类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSchedulerWorkerRunEventKindDto {
    Acknowledged,
    Started,
    Heartbeat,
    Submitted,
    Completed,
    Failed,
}

/// Matrix/remote Worker 对一次 run 的生命周期上报。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSchedulerWorkerRunEventRequest {
    pub platform_id: String,
    pub company_id: String,
    pub project_id: String,
    pub worker_id: String,
    pub event: AgentSchedulerWorkerRunEventKindDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<AgentSchedulerWorkspaceContextDto>,
    #[serde(default)]
    pub payload: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub now_ms: Option<TimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_expires_at_ms: Option<TimestampMs>,
}

/// Matrix/remote Worker 生命周期上报响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSchedulerWorkerRunEventResponse {
    pub accepted: bool,
    pub task: AgentSchedulerTaskBoardTaskDto,
    pub run: AgentRunDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<AgentSchedulerTaskBoardEventDto>,
}

/// Team 状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTeamStatusDto {
    Pending,
    Active,
    Degraded,
    Failed,
}

/// Team 成员状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTeamMemberDto {
    pub worker_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_name: Option<String>,
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matrix_user_id: Option<String>,
    pub ready: bool,
}

/// Agent Team 快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTeamDto {
    pub id: String,
    pub name: String,
    pub status: AgentTeamStatusDto,
    pub leader_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_room_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leader_dm_room_id: Option<String>,
    pub leader_ready: bool,
    pub ready_workers: u32,
    pub total_workers: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<AgentTeamMemberDto>,
    pub updated_at_ms: TimestampMs,
}

/// Agent Team 写入/更新请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerUpsertTeamRequest {
    pub team_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<AgentTeamStatusDto>,
    pub leader_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_room_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leader_dm_room_id: Option<String>,
    pub leader_ready: bool,
    pub ready_workers: u32,
    pub total_workers: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<AgentTeamMemberDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at_ms: Option<TimestampMs>,
}

/// Agent Team 写入/更新响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerUpsertTeamResponse {
    pub team: AgentTeamDto,
}

/// Agent Team 列表响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTeamsResponse {
    pub teams: Vec<AgentTeamDto>,
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
    MatrixDispatched,
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

/// Agent Scheduler Matrix 派发事件的 Matrix room event type。
pub const AGENT_SCHEDULER_MATRIX_DISPATCH_EVENT_TYPE: &str =
    "com.vibewindow.agent_scheduler.dispatch.v1";

const AGENT_SCHEDULER_MATRIX_TEXT_LIMIT: usize = 1024;

/// Agent Scheduler Matrix worker 执行一次 run 所需的结构化派发事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerMatrixDispatchEventDto {
    pub event_version: u8,
    pub platform_id: String,
    pub company_id: String,
    pub project_id: String,
    pub task_id: String,
    pub run_id: String,
    pub worker_id: String,
    pub worker_role: String,
    pub room_id: String,
    pub matrix_user_id: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp_agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffortDto>,
    pub spec_path: String,
    pub metadata_path: String,
    pub created_at_ms: TimestampMs,
}

impl AgentSchedulerMatrixDispatchEventDto {
    /// 校验事件 content 是否可以作为 Matrix room event 进入调度派发流。
    pub fn validate_for_matrix_room_event(&self) -> Result<(), String> {
        if self.event_version != 1 {
            return Err("event_version must be 1".to_string());
        }
        validate_agent_scheduler_matrix_required_text("platform_id", &self.platform_id)?;
        validate_agent_scheduler_matrix_required_text("company_id", &self.company_id)?;
        validate_agent_scheduler_matrix_required_text("project_id", &self.project_id)?;
        validate_agent_scheduler_matrix_required_text("task_id", &self.task_id)?;
        validate_agent_scheduler_matrix_required_text("run_id", &self.run_id)?;
        validate_agent_scheduler_matrix_required_text("worker_id", &self.worker_id)?;
        validate_agent_scheduler_matrix_required_text("worker_role", &self.worker_role)?;
        validate_agent_scheduler_matrix_required_text("room_id", &self.room_id)?;
        validate_agent_scheduler_matrix_required_text("matrix_user_id", &self.matrix_user_id)?;
        validate_agent_scheduler_matrix_required_text("model", &self.model)?;
        validate_agent_scheduler_matrix_optional_text("model_provider", &self.model_provider)?;
        validate_agent_scheduler_matrix_optional_text("agent", &self.agent)?;
        validate_agent_scheduler_matrix_optional_text("acp_agent", &self.acp_agent)?;
        validate_agent_scheduler_matrix_required_text("spec_path", &self.spec_path)?;
        validate_agent_scheduler_matrix_required_text("metadata_path", &self.metadata_path)?;
        Ok(())
    }
}

fn validate_agent_scheduler_matrix_required_text(field: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    validate_agent_scheduler_matrix_text_limit(field, value)
}

fn validate_agent_scheduler_matrix_optional_text(
    field: &str,
    value: &Option<String>,
) -> Result<(), String> {
    if let Some(value) = value {
        validate_agent_scheduler_matrix_required_text(field, value)?;
    }
    Ok(())
}

fn validate_agent_scheduler_matrix_text_limit(field: &str, value: &str) -> Result<(), String> {
    if value.chars().count() > AGENT_SCHEDULER_MATRIX_TEXT_LIMIT {
        return Err(format!(
            "{field} must be at most {AGENT_SCHEDULER_MATRIX_TEXT_LIMIT} characters"
        ));
    }
    Ok(())
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
    /// 必须通过 Matrix worker 房间分发。
    Matrix,
    /// 兼容旧分发逻辑：有 Matrix worker 地址时走 Matrix，否则走本地目标。
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

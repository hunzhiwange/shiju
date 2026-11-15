use vw_api_transport::{common::TimestampMs, id::{IdValidationError, ProjectId, ProjectScope}};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

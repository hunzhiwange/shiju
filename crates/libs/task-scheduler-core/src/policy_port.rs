//! 策略所需的存储和生命周期动作；实现负责绑定并保留原项目作用域。
use anyhow::Result;
use serde_json::Value;
use vw_api_transport::common::TimestampMs;
use vw_api_task::{TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusRoleDto};
use crate::domain::{AssigneeId, TeamId, RunId, TaskId};
use crate::events::AgentEventKind;
use crate::state_machine::AgentTaskStatus;
use crate::task_records::StoredAgentTask;

#[derive(Debug, Clone)]
pub struct PolicyLogRequest {
    pub task_id: TaskId,
    pub run_id: Option<RunId>,
    pub timestamp_ms: TimestampMs,
    pub from_status: Option<AgentTaskStatus>,
    pub to_status: Option<AgentTaskStatus>,
    pub from_status_key: Option<String>,
    pub to_status_key: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct PolicyEventRequest {
    pub task_id: Option<TaskId>,
    pub run_id: Option<RunId>,
    pub worker_id: Option<AssigneeId>,
    pub team_id: Option<TeamId>,
    pub kind: AgentEventKind,
    pub from_status: Option<AgentTaskStatus>,
    pub to_status: Option<AgentTaskStatus>,
    pub from_status_key: Option<String>,
    pub to_status_key: Option<String>,
    pub message: String,
    pub payload: Value,
    pub created_at_ms: TimestampMs,
}

/// 单次策略读取的运行投影；不拥有持久化运行状态。
pub struct PolicyRunSnapshot {
    pub id: RunId,
    pub attempt: u32,
    pub lease_acquired_at_ms: Option<TimestampMs>,
    pub started_at_ms: Option<TimestampMs>,
    pub finished_at_ms: Option<TimestampMs>,
    pub last_heartbeat_at_ms: Option<TimestampMs>,
    pub error_message: Option<String>,
}

/// 生命周期宿主返回的目标投影；状态解析仍由真实生命周期实现负责。
pub struct PolicyStatusProjection {
    pub status_key: String,
    pub status_role: TaskPoolWorkflowStatusRoleDto,
    pub runtime_status: AgentTaskStatus,
}

pub struct PolicyLogSnapshot {
    pub timestamp_ms: TimestampMs,
    pub message: String,
}

/// 绑定单一项目与租户作用域的策略动作。
///
/// 实现必须保留原存储校验与错误，不跨作用域回退；两个工作流加载入口
/// 分别保留可选已保存工作流和派发默认工作流的原有语义。
pub trait PolicyHost {
    fn load_task_board_workflow(&self) -> Result<Option<TaskPoolWorkflowSnapshotDto>>;
    fn load_dispatch_workflow(&self) -> Result<TaskPoolWorkflowSnapshotDto>;
    fn list_active_tasks(&self) -> Result<Vec<StoredAgentTask>>;
    fn task_board_status_key(&self, task: &StoredAgentTask) -> String;
    fn resolve_outcome(&self, workflow: &TaskPoolWorkflowSnapshotDto, from: &str, outcome: &str) -> Result<PolicyStatusProjection>;
    fn transition_task_status_with_metadata(&self, task_id: &TaskId, status: AgentTaskStatus, metadata: Value, now: TimestampMs) -> Result<Option<StoredAgentTask>>;
    fn load_latest_run_for_task(&self, task_id: &TaskId) -> Result<Option<PolicyRunSnapshot>>;
    fn list_task_logs(&self, task_id: &TaskId) -> Result<Vec<PolicyLogSnapshot>>;
    fn finish_timed_out_run(&self, run_id: &RunId, now: TimestampMs, error: Option<String>) -> Result<()>;
    fn append_task_log(&self, request: PolicyLogRequest) -> Result<()>;
    fn append_event(&self, request: PolicyEventRequest) -> Result<()>;
}

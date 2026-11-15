//! 任务存储与产物共享的数据契约。

use crate::domain::{AssigneeId, TaskId, TeamId};
use crate::state_machine::{AgentTaskKind, AgentTaskStatus};
use serde_json::Value;
use vw_api_transport::common::TimestampMs;

#[derive(Debug, Clone, PartialEq)]
/// 创建任务所需的完整领域字段；写入前会校验种类与调度字段的一致性。
pub struct NewAgentTask {
    pub id: TaskId,
    pub title: String,
    pub kind: AgentTaskKind,
    pub status: AgentTaskStatus,
    pub spec_uri: String,
    pub meta_uri: String,
    pub assigned_to: Option<AssigneeId>,
    pub team_id: Option<TeamId>,
    pub room_id: Option<String>,
    pub project_room_id: Option<String>,
    pub delegated_to_team: Option<TeamId>,
    pub schedule: Option<String>,
    pub timezone: Option<String>,
    pub next_scheduled_at: Option<String>,
    pub last_executed_at: Option<String>,
    pub dag: Vec<AgentTaskDagNode>,
    pub metadata: Value,
    pub created_at_ms: TimestampMs,
}

#[derive(Debug, Clone, PartialEq)]
/// 从存储读取的任务快照，包括持久化时间和看板元数据。
pub struct StoredAgentTask {
    pub id: TaskId,
    pub title: String,
    pub kind: AgentTaskKind,
    pub status: AgentTaskStatus,
    pub spec_uri: String,
    pub meta_uri: String,
    pub assigned_to: Option<AssigneeId>,
    pub team_id: Option<TeamId>,
    pub room_id: Option<String>,
    pub project_room_id: Option<String>,
    pub delegated_to_team: Option<TeamId>,
    pub schedule: Option<String>,
    pub timezone: Option<String>,
    pub next_scheduled_at: Option<String>,
    pub last_executed_at: Option<String>,
    pub dag: Vec<AgentTaskDagNode>,
    pub metadata: Value,
    pub created_at_ms: TimestampMs,
    pub updated_at_ms: TimestampMs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 存储层使用的团队任务依赖图节点。
pub struct AgentTaskDagNode {
    pub task_id: TaskId,
    pub title: String,
    pub depends_on: Vec<TaskId>,
    pub assigned_to: Option<AssigneeId>,
    pub status: AgentTaskStatus,
}

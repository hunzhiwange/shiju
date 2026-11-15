//! 任务池状态、工作流与 worktree 契约。

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use vw_api_transport::{ProjectId, TaskId};

/// 任务看板任务池状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPoolStatus {
    Pool,
    Pending,
    Planning,
    Running,
    Failed,
    Paused,
    CodeComplete,
    CodeReview,
    PrSubmitted,
    Completed,
    Archived,
}

impl TaskPoolStatus {
    /// 返回内置任务池状态的稳定 key。
    pub fn status_key(self) -> &'static str {
        match self {
            TaskPoolStatus::Pool => "pool",
            TaskPoolStatus::Pending => "pending",
            TaskPoolStatus::Planning => "planning",
            TaskPoolStatus::Running => "running",
            TaskPoolStatus::Failed => "failed",
            TaskPoolStatus::Paused => "paused",
            TaskPoolStatus::CodeComplete => "code_complete",
            TaskPoolStatus::CodeReview => "code_review",
            TaskPoolStatus::PrSubmitted => "pr_submitted",
            TaskPoolStatus::Completed => "completed",
            TaskPoolStatus::Archived => "archived",
        }
    }

    /// 从内置状态 key 解析兼容枚举；自定义状态 key 返回 `None`。
    pub fn from_status_key(value: &str) -> Option<Self> {
        match value {
            "pool" => Some(TaskPoolStatus::Pool),
            "pending" => Some(TaskPoolStatus::Pending),
            "planning" => Some(TaskPoolStatus::Planning),
            "running" => Some(TaskPoolStatus::Running),
            "failed" => Some(TaskPoolStatus::Failed),
            "paused" => Some(TaskPoolStatus::Paused),
            "code_complete" => Some(TaskPoolStatus::CodeComplete),
            "code_review" => Some(TaskPoolStatus::CodeReview),
            "pr_submitted" => Some(TaskPoolStatus::PrSubmitted),
            "completed" => Some(TaskPoolStatus::Completed),
            "archived" => Some(TaskPoolStatus::Archived),
            _ => None,
        }
    }
}

/// 任务池工作流状态的引擎语义角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPoolWorkflowStatusRoleDto {
    Backlog,
    Queued,
    Planning,
    Executing,
    Failed,
    Paused,
    ReviewReady,
    Review,
    Merging,
    Done,
    Archived,
    Custom,
}

/// 任务池 workflow 状态转移分支。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorkflowTransitionDto {
    pub outcome: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub target: String,
}

/// 任务池 workflow 状态快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorkflowStatusDto {
    pub key: String,
    pub label: String,
    pub role: TaskPoolWorkflowStatusRoleDto,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
    #[serde(default)]
    pub terminal: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transitions: Vec<TaskPoolWorkflowTransitionDto>,
}

/// 任务池调度使用的 workflow 快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorkflowSnapshotDto {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub statuses: Vec<TaskPoolWorkflowStatusDto>,
}

impl TaskPoolWorkflowSnapshotDto {
    /// 校验调度所需的最小 workflow 结构约束。
    pub fn validate_for_schedule(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("workflow id must not be empty".to_string());
        }

        let mut status_keys = HashSet::new();
        for status in &self.statuses {
            if status.key.trim().is_empty() {
                return Err("workflow status key must not be empty".to_string());
            }
            if !status_keys.insert(status.key.as_str()) {
                return Err(format!("workflow status key must be unique: {}", status.key));
            }
        }

        for status in &self.statuses {
            if let Some(next) = status.next.as_deref() {
                validate_workflow_target(&status_keys, "next", next)?;
            }
            for transition in &status.transitions {
                if transition.outcome.trim().is_empty() {
                    return Err(format!(
                        "workflow transition outcome must not be empty: {}",
                        status.key
                    ));
                }
                validate_workflow_target(&status_keys, "transition", &transition.target)?;
            }
        }

        Ok(())
    }
}

fn validate_workflow_target(
    status_keys: &HashSet<&str>,
    target_kind: &str,
    target: &str,
) -> Result<(), String> {
    if target.trim().is_empty() {
        return Err(format!("workflow {target_kind} target must not be empty"));
    }
    if !status_keys.contains(target) {
        return Err(format!("unknown workflow {target_kind} target: {target}"));
    }
    Ok(())
}

/// 任务池 worktree 槽位状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPoolWorktreeSlotStateDto {
    Idle,
    Busy,
    Tainted,
    Recycling,
    Dead,
}

/// 任务池 worktree 槽位快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeSlotDto {
    /// 现有智能体定义及独立运行的投射，不创建员工账户。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub id: String,
    pub path: String,
    pub branch: String,
    pub state: TaskPoolWorktreeSlotStateDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leased_task_id: Option<TaskId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taint_reason: Option<String>,
}

/// 任务池 worktree 合并目标锁快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeMergeLockDto {
    pub target_branch: String,
    pub task_id: TaskId,
    /// 合并锁自获取以来的持有时长（毫秒）。
    #[serde(default)]
    pub age_ms: u64,
}

/// 读取任务池 worktree 快照请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeSnapshotRequest {
    pub project_id: ProjectId,
}

/// 任务池 worktree 快照响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeSnapshotResponse {
    pub project_id: ProjectId,
    pub repo_root: String,
    pub pool_root: String,
    pub base_branch: String,
    #[serde(default)]
    pub idle_count: u32,
    #[serde(default)]
    pub busy_count: u32,
    #[serde(default)]
    pub tainted_count: u32,
    #[serde(default)]
    pub recycling_count: u32,
    #[serde(default)]
    pub dead_count: u32,
    #[serde(default)]
    pub merge_target_locks: Vec<TaskPoolWorktreeMergeLockDto>,
    #[serde(default)]
    pub slots: Vec<TaskPoolWorktreeSlotDto>,
}

/// 任务池 worktree 维护请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeMaintenanceRequest {
    pub project_id: ProjectId,
    pub desired_capacity: u32,
}

/// 任务池 worktree 维护响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeMaintenanceResponse {
    #[serde(default)]
    pub idle_count: u32,
    #[serde(default)]
    pub target_idle_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<TaskPoolWorktreeSnapshotResponse>,
}

/// 获取任务池 worktree 槽位请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeAcquireRequest {
    pub project_id: ProjectId,
    pub task_id: TaskId,
}

/// 获取任务池 worktree 槽位响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeAcquireResponse {
    #[serde(default)]
    pub leased: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<TaskPoolWorktreeSlotDto>,
}

/// 释放任务池 worktree 租约请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeReleaseRequest {
    pub project_id: ProjectId,
    pub task_id: TaskId,
}

/// 释放任务池 worktree 租约响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeReleaseResponse {
    pub released: bool,
}

/// 回收任务池 worktree 租约请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeRecycleRequest {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taint_reason: Option<String>,
}

/// 回收任务池 worktree 租约响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeRecycleResponse {
    pub recycled: bool,
}

/// 回收指定任务池 worktree 槽位请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeSlotRecycleRequest {
    pub project_id: ProjectId,
    pub slot_id: String,
}

/// 回收指定任务池 worktree 槽位响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeSlotRecycleResponse {
    pub recycled: bool,
}

/// 获取任务池 worktree 合并目标锁请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeMergeLockRequest {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub target_branch: String,
}

/// 获取任务池 worktree 合并目标锁响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeMergeLockResponse {
    pub locked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holder_task_id: Option<TaskId>,
}

/// 释放任务池 worktree 合并目标锁请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeMergeUnlockRequest {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub target_branch: String,
}

/// 释放任务池 worktree 合并目标锁响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolWorktreeMergeUnlockResponse {
    pub released: bool,
}

fn is_true(value: &bool) -> bool {
    *value
}

fn default_true() -> bool {
    true
}

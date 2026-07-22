//! 后台任务、任务流事件与执行请求相关类型。
//!
//! 本模块建模后台任务系统的 API 边界，主要用于：
//! - 创建命令任务
//! - 追踪任务状态迁移
//! - 订阅 stdout、stderr 与完成事件
//! - 列出或取消已有任务
//!
//! 当前任务模型以命令执行为主，但协议设计上保留了任务种类枚举，便于后续扩展。

use crate::common::{OperationAck, StringMap, TimestampMs};
use crate::id::{ProjectId, TaskId, WorktreeId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 任务种类。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// 命令执行任务。
    Command,
}

/// 任务执行状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// 已入队，尚未开始执行。
    Queued,
    /// 正在执行。
    Running,
    /// 已成功完成。
    Completed,
    /// 执行失败。
    Failed,
    /// 被主动取消。
    Cancelled,
}

/// 命令任务规格。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandTaskSpecDto {
    /// 命令及参数数组。
    pub argv: Vec<String>,
    /// 命令执行目录。
    pub cwd: String,
    /// 命令环境变量。
    #[serde(default)]
    pub env: StringMap,
}

/// 任务详情。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskDto {
    /// 任务 ID。
    pub id: TaskId,
    /// 任务种类。
    pub kind: TaskKind,
    /// 当前状态。
    pub status: TaskStatus,
    /// 创建时间。
    pub created_at_ms: TimestampMs,
    /// 启动时间。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<TimestampMs>,
    /// 完成时间。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at_ms: Option<TimestampMs>,
    /// 进程退出码。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

/// 创建任务请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    /// 目标项目 ID。
    pub project_id: ProjectId,
    /// 可选工作树 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
    /// 任务类型。
    pub kind: TaskKind,
    /// 命令任务参数；仅命令任务使用。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<CommandTaskSpecDto>,
    /// 是否启用流式输出。
    #[serde(default)]
    pub stream: bool,
}

/// 创建任务响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateTaskResponse {
    /// 新建任务详情。
    pub task: TaskDto,
}

/// 获取单个任务响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetTaskResponse {
    /// 任务详情。
    pub task: TaskDto,
}

/// 列出任务请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ListTasksRequest {
    /// 按项目过滤。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    /// 按状态过滤。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<TaskStatus>,
}

/// 列出任务响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListTasksResponse {
    /// 任务列表。
    pub items: Vec<TaskDto>,
}

/// 任务启动事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskStartedEvent {
    pub task_id: TaskId,
}

/// 任务标准输出事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskStdoutEvent {
    pub task_id: TaskId,
    pub chunk: String,
}

/// 任务标准错误输出事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskStderrEvent {
    pub task_id: TaskId,
    pub chunk: String,
}

/// 任务完成事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskCompletedEvent {
    pub task_id: TaskId,
    pub exit_code: i32,
    pub status: TaskStatus,
}

/// 任务错误事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskErrorEvent {
    pub task_id: TaskId,
    pub message: String,
}

/// 统一任务事件枚举。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum TaskEvent {
    Started(TaskStartedEvent),
    Stdout(TaskStdoutEvent),
    Stderr(TaskStderrEvent),
    Completed(TaskCompletedEvent),
    Error(TaskErrorEvent),
}

/// 取消任务响应沿用通用确认结构。
pub type CancelTaskResponse = OperationAck;

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

/// 任务池 Matrix 协同事件的 Matrix room event type。
pub const TASK_POOL_MATRIX_EVENT_TYPE: &str = "com.vibewindow.task_pool.event.v1";

const TASK_POOL_MATRIX_TEXT_LIMIT: usize = 1024;

/// 任务池 Matrix 协同事件的发起者。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolMatrixActorDto {
    pub matrix_user_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// 任务池 Matrix 协同事件 envelope。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolMatrixEventDto {
    pub version: u8,
    pub event_id: String,
    pub task_id: String,
    pub actor: TaskPoolMatrixActorDto,
    pub occurred_at_ms: u64,
    pub kind: TaskPoolMatrixEventKindDto,
}

impl TaskPoolMatrixEventDto {
    /// 校验事件 content 是否可以作为 Matrix room event 进入任务池协同流。
    pub fn validate_for_matrix_room_event(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("version must be 1".to_string());
        }
        validate_required_text("event_id", &self.event_id)?;
        validate_required_text("task_id", &self.task_id)?;
        validate_required_text("actor.matrix_user_id", &self.actor.matrix_user_id)?;
        if let Some(display_name) = &self.actor.display_name {
            validate_text_limit("actor.display_name", display_name)?;
        }

        match &self.kind {
            TaskPoolMatrixEventKindDto::TaskStatusChanged { .. } => {}
            TaskPoolMatrixEventKindDto::TaskClaimed { assignee } => {
                validate_text_limit("kind.assignee", assignee)?;
            }
            TaskPoolMatrixEventKindDto::TaskNote { message } => {
                validate_text_limit("kind.message", message)?;
            }
            TaskPoolMatrixEventKindDto::TaskBlocked { reason } => {
                validate_text_limit("kind.reason", reason)?;
            }
        }

        Ok(())
    }
}

/// 任务池 Matrix 协同事件 kind。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskPoolMatrixEventKindDto {
    TaskStatusChanged {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from_status: Option<TaskPoolStatus>,
        to_status: TaskPoolStatus,
    },
    TaskClaimed {
        assignee: String,
    },
    TaskNote {
        message: String,
    },
    TaskBlocked {
        reason: String,
    },
}

/// 发布任务池 Matrix 协同事件请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolMatrixPublishRequest {
    pub event: TaskPoolMatrixEventDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matrix_instance_id: Option<String>,
}

/// 发布任务池 Matrix 协同事件响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolMatrixPublishResponse {
    pub accepted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matrix_event_id: Option<String>,
}

/// 拉取任务池 Matrix 协同事件响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPoolMatrixEventsResponse {
    pub cursor: u64,
    pub events: Vec<TaskPoolMatrixEventDto>,
}

/// 聊天 workflow 模式触发的计划执行请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPlanRunRequest {
    pub query: String,
    pub root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp_agent: Option<String>,
}

/// 计划执行 SSE 事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskPlanRunStreamEventDto {
    PlanningStarted {
        task_id: String,
        plan_dir: String,
    },
    PlanCreated {
        task_id: String,
        plan_dir: String,
        plan_md_path: String,
        prompt_md_path: String,
        tasks_json_path: String,
        subtask_count: usize,
    },
    BatchStarted {
        task_id: String,
        batch: u32,
        subtask_ids: Vec<String>,
    },
    SubtaskStarted {
        task_id: String,
        batch: u32,
        subtask_id: String,
        title: String,
        task_file: String,
    },
    SubtaskFinished {
        task_id: String,
        batch: u32,
        subtask_id: String,
        title: String,
        task_file: String,
        success: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    Blocked {
        task_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        batch: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subtask_id: Option<String>,
        reason: String,
    },
    Finished {
        task_id: String,
        plan_dir: String,
        status: String,
        output: String,
    },
    Error {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task_id: Option<String>,
        message: String,
    },
}

fn validate_required_text(field: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    validate_text_limit(field, value)
}

fn validate_text_limit(field: &str, value: &str) -> Result<(), String> {
    if value.chars().count() > TASK_POOL_MATRIX_TEXT_LIMIT {
        return Err(format!("{field} must be at most {TASK_POOL_MATRIX_TEXT_LIMIT} characters"));
    }
    Ok(())
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

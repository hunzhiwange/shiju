//! 后台任务、任务流事件与执行请求相关类型。
//!
//! 本模块建模后台任务系统的 API 边界，主要用于：
//! - 创建命令任务
//! - 追踪任务状态迁移
//! - 订阅 stdout、stderr 与完成事件
//! - 列出或取消已有任务
//!
//! 当前任务模型以命令执行为主，但协议设计上保留了任务种类枚举，便于后续扩展。

use vw_api_transport::common::{OperationAck, StringMap, TimestampMs};
use vw_api_transport::id::{ProjectId, TaskId, WorktreeId};
use serde::{Deserialize, Serialize};

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

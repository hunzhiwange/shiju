//! 可见任务会话状态契约；协调器及持久化由宿主提供。
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 单次任务会话等待允许的最长时间。
pub const MAX_TASK_SESSION_WAIT: Duration = Duration::from_secs(60);

/// 可见任务会话的运行状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskSessionState {
    Queued,
    Running,
    Succeeded,
    Failed,
}

impl TaskSessionState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed)
    }
}

/// 可见任务会话的稳定状态快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSessionSnapshot {
    pub parent_session_id: String,
    pub task_session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub state: TaskSessionState,
    pub error: Option<String>,
    pub created_ms: u64,
    pub updated_ms: u64,
}

/// 等待任一或全部目标进入终态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSessionWaitCondition {
    Any,
    All,
}

/// 一次有界等待的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSessionWaitOutcome {
    pub tasks: Vec<TaskSessionSnapshot>,
    pub timed_out: bool,
}

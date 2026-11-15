//! 计划执行契约。

use serde::{Deserialize, Serialize};

/// 聊天工作流触发的计划执行请求。
///
/// 网关侧始终按目标模式执行：独立验收未通过时自动续轮，直到达成或真实阻塞。
/// 旧客户端若仍发送 `mode` 字段会被忽略。
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
    /// 发起 plan-run 的主会话 ID，供子会话 `parent_id` 绑定。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<String>,
}

/// 后台计划运行的可查询生命周期状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPlanRunStatus {
    Running,
    Waiting,
    Completed,
    Blocked,
    Failed,
    Cancelled,
}

impl TaskPlanRunStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Blocked | Self::Failed | Self::Cancelled)
    }
}

/// 计划运行控制面的稳定状态快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPlanRunStatusResponse {
    pub task_id: String,
    pub status: TaskPlanRunStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_event: Option<TaskPlanRunStreamEventDto>,
}

/// 计划执行 SSE 事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskPlanRunStreamEventDto {
    PlanningStarted {
        task_id: String,
        plan_dir: String,
        /// 规划子会话 ID（`{sanitize(task_id)}-plan`）。
        session_id: String,
    },
    /// 规划或执行子会话产生的普通聊天增量。
    SessionDelta {
        task_id: String,
        session_id: String,
        delta: String,
    },
    /// 会话主体已返回，运行器正在等待其 AgentTool 递归后代进入终态。
    SessionWaiting {
        task_id: String,
        session_id: String,
        active_descendant_session_ids: Vec<String>,
    },
    PlanCreated {
        task_id: String,
        plan_dir: String,
        plan_md_path: String,
        prompt_md_path: String,
        tasks_json_path: String,
        subtask_count: usize,
        /// 规划子会话 ID（`{sanitize(task_id)}-plan`）。
        session_id: String,
    },
    BatchStarted {
        task_id: String,
        batch: u32,
        subtask_ids: Vec<String>,
    },
    BatchFinished {
        task_id: String,
        batch: u32,
        subtask_ids: Vec<String>,
        success: bool,
    },
    /// 目标模式开始一次自动续轮。
    GoalTurnStarted {
        task_id: String,
        turn: u32,
        session_id: String,
        reason: String,
    },
    /// 独立目标评估器完成一次判定。
    GoalEvaluated {
        task_id: String,
        turn: u32,
        achieved: bool,
        reason: String,
        session_id: String,
    },
    SubtaskStarted {
        task_id: String,
        batch: u32,
        subtask_id: String,
        title: String,
        task_file: String,
        /// 子任务会话 ID（`{sanitize(task_id)}-{sanitize(subtask_id)}`）。
        session_id: String,
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
        /// 子任务会话 ID（`{sanitize(task_id)}-{sanitize(subtask_id)}`）。
        session_id: String,
    },
    Blocked {
        task_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        batch: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subtask_id: Option<String>,
        reason: String,
        /// 对应规划或失败子任务的会话 ID。
        session_id: String,
    },
    Finished {
        task_id: String,
        plan_dir: String,
        status: String,
        output: String,
        /// 规划子会话 ID（可选透出，便于回到规划上下文）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_id: Option<String>,
    },
    Error {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task_id: Option<String>,
        message: String,
    },
    /// 用户通过控制面停止了后台运行。
    Cancelled {
        task_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_id: Option<String>,
    },
}

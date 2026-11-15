use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use super::{now_ms, SubTask, TaskExecutorBackend, TaskSplitMode, TASK_MODEL_AUTO};

/// TaskStatus 枚举描述该模块支持的 TaskStatus 取值集合。
///
/// 每个变体代表一个明确分支，调用方应通过显式匹配处理新增状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TaskStatus {
    #[default]
    Pool,
    Pending,
    Running,
    Failed,
    Paused,
    CodeComplete,
    CodeReview,
    PrSubmitted,
    Completed,
    Archived,
}

impl TaskStatus {
    /// 提供 all 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn all() -> [TaskStatus; 10] {
        [
            TaskStatus::Pool,
            TaskStatus::Pending,
            TaskStatus::Running,
            TaskStatus::Failed,
            TaskStatus::Paused,
            TaskStatus::CodeComplete,
            TaskStatus::CodeReview,
            TaskStatus::PrSubmitted,
            TaskStatus::Completed,
            TaskStatus::Archived,
        ]
    }

    /// 提供 label 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn label(&self) -> &'static str {
        match self {
            TaskStatus::Pool => "任务池",
            TaskStatus::Pending => "待执行",
            TaskStatus::Running => "执行中",
            TaskStatus::Failed => "执行失败",
            TaskStatus::Paused => "暂停中",
            TaskStatus::CodeComplete => "代码完成",
            TaskStatus::CodeReview => "代码审核",
            TaskStatus::PrSubmitted => "合并代码",
            TaskStatus::Completed => "任务完成",
            TaskStatus::Archived => "任务归档",
        }
    }

    /// 提供 next 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn next(&self) -> Option<TaskStatus> {
        match self {
            TaskStatus::Pool => Some(TaskStatus::Pending),
            TaskStatus::Pending => Some(TaskStatus::Running),
            TaskStatus::Running => Some(TaskStatus::CodeComplete),
            TaskStatus::Failed => Some(TaskStatus::Pending),
            TaskStatus::Paused => None,
            TaskStatus::CodeComplete => Some(TaskStatus::CodeReview),
            TaskStatus::CodeReview => Some(TaskStatus::PrSubmitted),
            TaskStatus::PrSubmitted => Some(TaskStatus::Completed),
            TaskStatus::Completed => Some(TaskStatus::Archived),
            TaskStatus::Archived => None,
        }
    }

    /// 解析 key 输入。
    ///
    /// 返回结构化结果；无法识别的输入会显式返回错误，便于调用方定位补丁格式问题。
    pub fn parse_key(s: &str) -> Option<TaskStatus> {
        match s {
            "pool" | "Pool" => Some(TaskStatus::Pool),
            "pending" | "Pending" => Some(TaskStatus::Pending),
            "running" | "Running" => Some(TaskStatus::Running),
            "failed" | "Failed" => Some(TaskStatus::Failed),
            "paused" | "Paused" => Some(TaskStatus::Paused),
            "code_complete" | "CodeComplete" => Some(TaskStatus::CodeComplete),
            "pr_submitted" | "PrSubmitted" => Some(TaskStatus::PrSubmitted),
            "code_review" | "CodeReview" => Some(TaskStatus::CodeReview),
            "completed" | "Completed" => Some(TaskStatus::Completed),
            "archived" | "Archived" => Some(TaskStatus::Archived),
            _ => None,
        }
    }

    /// 提供 to string key 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn to_string_key(&self) -> &'static str {
        match self {
            TaskStatus::Pool => "pool",
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Failed => "failed",
            TaskStatus::Paused => "paused",
            TaskStatus::CodeComplete => "code_complete",
            TaskStatus::PrSubmitted => "pr_submitted",
            TaskStatus::CodeReview => "code_review",
            TaskStatus::Completed => "completed",
            TaskStatus::Archived => "archived",
        }
    }
}

fn format_task_date_from_ms(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let dt = OffsetDateTime::from_unix_timestamp(secs).unwrap_or(OffsetDateTime::UNIX_EPOCH);
    let month: u8 = dt.month().into();
    format!("{:04}{:02}{:02}", dt.year(), month, dt.day())
}

fn generate_task_id() -> String {
    let ms = now_ms();
    let date = format_task_date_from_ms(ms);
    let seq = (ms % 10000) as u32;
    format!("T{}.{:04}", date, seq)
}

/// TaskLogEntry 数据结构承载该模块对外传递的 TaskLogEntry 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLogEntry {
    pub timestamp_ms: u64,
    pub status_from: Option<TaskStatus>,
    pub status_to: Option<TaskStatus>,
    pub message: String,
}

impl TaskLogEntry {
    /// 提供 new status change 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn new_status_change(from: TaskStatus, to: TaskStatus) -> Self {
        Self {
            timestamp_ms: now_ms(),
            status_from: Some(from),
            status_to: Some(to),
            message: format!("状态变更: {} → {}", from.label(), to.label()),
        }
    }

    /// 提供 new message 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn new_message(message: String) -> Self {
        Self { timestamp_ms: now_ms(), status_from: None, status_to: None, message }
    }
}

/// Task 数据结构承载该模块对外传递的 Task 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub priority: u32,
    pub assignee: String,
    pub model: String,
    pub executor: TaskExecutorBackend,
    pub description: String,
    pub prompt: String,
    pub status: TaskStatus,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub logs: Vec<TaskLogEntry>,
    pub order: u32,
    pub deleted: bool,
    pub archived: bool,
    pub subtasks: Vec<SubTask>,
    #[serde(default)]
    pub task_split_mode: TaskSplitMode,
    pub auto_promote_delay_ms: Option<u64>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub pause_reason: Option<String>,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default = "now_ms")]
    pub last_active_at_ms: u64,
    #[serde(default)]
    pub execution_started_at_ms: Option<u64>,
    #[serde(default)]
    pub last_execution_duration_ms: Option<u64>,
    #[serde(default)]
    pub merge_source_branch: Option<String>,
    #[serde(default)]
    pub merge_target_branch: Option<String>,
    #[serde(default)]
    pub selected_worktree_path: Option<String>,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            id: generate_task_id(),
            priority: 999,
            assignee: "VibeWindow".to_string(),
            model: TASK_MODEL_AUTO.to_string(),
            executor: TaskExecutorBackend::default(),
            description: String::new(),
            prompt: String::new(),
            status: TaskStatus::Pool,
            created_at_ms: now_ms(),
            updated_at_ms: now_ms(),
            logs: Vec::new(),
            order: 0,
            deleted: false,
            archived: false,
            subtasks: Vec::new(),
            task_split_mode: TaskSplitMode::FollowSystem,
            auto_promote_delay_ms: None,
            last_error: None,
            pause_reason: None,
            retry_count: 0,
            last_active_at_ms: now_ms(),
            execution_started_at_ms: None,
            last_execution_duration_ms: None,
            merge_source_branch: None,
            merge_target_branch: None,
            selected_worktree_path: None,
        }
    }
}

impl Task {
    /// 提供 new 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn new(priority: u32) -> Self {
        let now = now_ms();
        Self {
            id: generate_task_id(),
            priority,
            assignee: "VibeWindow".to_string(),
            model: TASK_MODEL_AUTO.to_string(),
            executor: TaskExecutorBackend::default(),
            description: String::new(),
            prompt: String::new(),
            status: TaskStatus::Pool,
            created_at_ms: now,
            updated_at_ms: now,
            logs: vec![TaskLogEntry::new_message("任务创建".to_string())],
            order: 0,
            deleted: false,
            archived: false,
            subtasks: Vec::new(),
            task_split_mode: TaskSplitMode::FollowSystem,
            auto_promote_delay_ms: None,
            last_error: None,
            pause_reason: None,
            retry_count: 0,
            last_active_at_ms: now,
            execution_started_at_ms: None,
            last_execution_duration_ms: None,
            merge_source_branch: None,
            merge_target_branch: None,
            selected_worktree_path: None,
        }
    }

    /// 提供 set status 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn set_status(&mut self, new_status: TaskStatus) {
        let old_status = self.status;
        let now = now_ms();
        self.status = new_status;
        self.updated_at_ms = now;
        self.last_active_at_ms = now;
        if new_status != TaskStatus::Failed {
            self.last_error = None;
        }
        if new_status != TaskStatus::Paused {
            self.pause_reason = None;
        }
        self.logs.push(TaskLogEntry::new_status_change(old_status, new_status));
    }

    /// 提供 add log 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn add_log(&mut self, message: String) {
        let now = now_ms();
        self.logs.push(TaskLogEntry::new_message(message));
        self.updated_at_ms = now;
        self.last_active_at_ms = now;
    }

    /// 提供 start execution 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn start_execution(&mut self, trigger_log: String) {
        if self.last_error.is_some() {
            self.retry_count = self.retry_count.saturating_add(1);
            self.add_log(format!("进入重试，第 {} 次", self.retry_count));
        }
        self.pause_reason = None;
        self.merge_source_branch = None;
        self.merge_target_branch = None;
        self.selected_worktree_path = None;
        self.execution_started_at_ms = Some(now_ms());
        self.last_execution_duration_ms = None;
        self.set_status(TaskStatus::Running);
        self.add_log(trigger_log);
    }

    /// 提供 start merge execution 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn start_merge_execution(&mut self, trigger_log: String) {
        let now = now_ms();
        self.pause_reason = None;
        self.execution_started_at_ms = Some(now);
        self.last_execution_duration_ms = None;
        self.updated_at_ms = now;
        self.last_active_at_ms = now;
        self.add_log(trigger_log);
    }

    /// 提供 should auto merge 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn should_auto_merge(&self) -> bool {
        self.merge_source_branch.as_deref().is_some_and(|value| !value.trim().is_empty())
            && self.merge_target_branch.as_deref().is_some_and(|value| !value.trim().is_empty())
    }

    /// 提供 mark execution failed 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn mark_execution_failed(&mut self, error: String) {
        self.last_error = Some(error.clone());
        if let Some(started_at) = self.execution_started_at_ms {
            self.last_execution_duration_ms = Some(now_ms().saturating_sub(started_at));
        }
        self.execution_started_at_ms = None;
        self.set_status(TaskStatus::Failed);
        self.add_log(format!("失败原因: {}", error));
    }

    /// 提供 mark execution succeeded 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn mark_execution_succeeded(&mut self) {
        self.last_error = None;
        self.pause_reason = None;
        if let Some(started_at) = self.execution_started_at_ms {
            self.last_execution_duration_ms = Some(now_ms().saturating_sub(started_at));
        }
        self.execution_started_at_ms = None;
    }

    /// 提供 mark paused 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn mark_paused(&mut self, reason: String) {
        self.pause_reason = Some(reason.clone());
        if let Some(started_at) = self.execution_started_at_ms {
            self.last_execution_duration_ms = Some(now_ms().saturating_sub(started_at));
        }
        self.execution_started_at_ms = None;
        self.set_status(TaskStatus::Paused);
        self.add_log(format!("暂停原因: {}", reason));
    }

    /// 提供 running duration ms 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn running_duration_ms(&self, now_ms: u64) -> Option<u64> {
        if self.status != TaskStatus::Running {
            return None;
        }
        let started_at = self.execution_started_at_ms.unwrap_or(self.last_active_at_ms);
        Some(now_ms.saturating_sub(started_at))
    }

    /// 提供 display execution duration ms 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn display_execution_duration_ms(&self, now_ms: u64) -> Option<u64> {
        self.running_duration_ms(now_ms).or(self.last_execution_duration_ms)
    }
}

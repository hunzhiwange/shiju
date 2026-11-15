use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{TaskStatus, TaskExecutorBackend, TaskSplitMode, TASK_MODEL_AUTO};

/// TaskIndex 数据结构承载该模块对外传递的 TaskIndex 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskIndex {
    pub tasks: HashMap<String, String>,
    pub order_by_status: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub last_task_date: Option<String>,
    #[serde(default)]
    pub last_task_seq: u32,
}

impl TaskIndex {
    /// 提供 new 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn new() -> Self {
        let mut order_by_status = HashMap::new();
        for status in TaskStatus::all() {
            order_by_status.insert(status.to_string_key().to_string(), Vec::new());
        }
        Self { tasks: HashMap::new(), order_by_status, last_task_date: None, last_task_seq: 0 }
    }
}

/// TaskBoardSettings 数据结构承载该模块对外传递的 TaskBoardSettings 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBoardSettings {
    pub auto_execute: bool,
    #[serde(default = "default_code_review_enabled")]
    pub code_review_enabled: bool,
    #[serde(default = "default_task_split_enabled")]
    pub task_split_enabled: bool,
    pub max_concurrent: u32,
    pub default_priority: u32,
    pub auto_promote_pool_tasks: bool,
    pub auto_promote_delay_seconds: u64,
    #[serde(default = "default_task_board_auto_refresh")]
    pub auto_refresh: bool,
    #[serde(default = "default_task_board_refresh_interval_seconds")]
    pub refresh_interval_seconds: u64,
    #[serde(default = "default_scheduler_tick_interval_seconds")]
    pub scheduler_tick_interval_seconds: u64,
    #[serde(default = "default_auto_promote_tick_interval_seconds")]
    pub auto_promote_tick_interval_seconds: u64,
    #[serde(default = "default_failed_retry_minutes")]
    pub failed_retry_minutes: u32,
    #[serde(default = "default_failed_retry_max_attempts")]
    pub failed_retry_max_attempts: u32,
    #[serde(default = "default_running_timeout_minutes")]
    pub running_timeout_minutes: u32,
    #[serde(default = "default_recycle_worktree_on_task_finish")]
    pub recycle_worktree_on_task_finish: bool,
    #[serde(default = "default_pr_submitted_stall_timeout_seconds")]
    pub pr_submitted_stall_timeout_seconds: u32,
}

impl TaskBoardSettings {
    /// 提供 new 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn new() -> Self {
        Self {
            auto_execute: true,
            code_review_enabled: false,
            task_split_enabled: false,
            max_concurrent: 3,
            default_priority: 999,
            auto_promote_pool_tasks: true,
            auto_promote_delay_seconds: 30,
            auto_refresh: default_task_board_auto_refresh(),
            refresh_interval_seconds: default_task_board_refresh_interval_seconds(),
            scheduler_tick_interval_seconds: default_scheduler_tick_interval_seconds(),
            auto_promote_tick_interval_seconds: default_auto_promote_tick_interval_seconds(),
            failed_retry_minutes: default_failed_retry_minutes(),
            failed_retry_max_attempts: default_failed_retry_max_attempts(),
            running_timeout_minutes: default_running_timeout_minutes(),
            recycle_worktree_on_task_finish: default_recycle_worktree_on_task_finish(),
            pr_submitted_stall_timeout_seconds: default_pr_submitted_stall_timeout_seconds(),
        }
    }

    /// 提供 sanitized 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn sanitized(mut self) -> Self {
        self.max_concurrent = self.max_concurrent.clamp(1, 10);
        self.refresh_interval_seconds = self.refresh_interval_seconds.clamp(1, 3600);
        self.scheduler_tick_interval_seconds = self.scheduler_tick_interval_seconds.clamp(1, 60);
        self.auto_promote_tick_interval_seconds =
            self.auto_promote_tick_interval_seconds.clamp(1, 3600);
        self.failed_retry_minutes = self.failed_retry_minutes.clamp(1, 1440);
        self.failed_retry_max_attempts = self.failed_retry_max_attempts.clamp(0, 10);
        self.running_timeout_minutes = self.running_timeout_minutes.clamp(1, 1440);
        self.pr_submitted_stall_timeout_seconds =
            self.pr_submitted_stall_timeout_seconds.clamp(5, 3600);
        self
    }
}

impl Default for TaskBoardSettings {
    fn default() -> Self {
        Self::new()
    }
}

const fn default_code_review_enabled() -> bool {
    false
}

const fn default_task_split_enabled() -> bool {
    false
}

const fn default_task_board_auto_refresh() -> bool {
    true
}

const fn default_task_board_refresh_interval_seconds() -> u64 {
    60
}

const fn default_scheduler_tick_interval_seconds() -> u64 {
    1
}

const fn default_auto_promote_tick_interval_seconds() -> u64 {
    30
}

const fn default_failed_retry_minutes() -> u32 {
    20
}

const fn default_failed_retry_max_attempts() -> u32 {
    10
}

const fn default_running_timeout_minutes() -> u32 {
    20
}

const fn default_recycle_worktree_on_task_finish() -> bool {
    false
}

const fn default_pr_submitted_stall_timeout_seconds() -> u32 {
    30
}

/// TaskDraft 数据结构承载该模块对外传递的 TaskDraft 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Debug, Clone)]
pub struct TaskDraft {
    pub priority: String,
    pub assignee: String,
    pub model: String,
    pub executor: TaskExecutorBackend,
    pub description: String,
    pub prompt: String,
    pub subtasks: Vec<String>,
    pub task_split_mode: TaskSplitMode,
    pub auto_promote_delay_seconds: String,
}

impl Default for TaskDraft {
    fn default() -> Self {
        Self {
            priority: "999".to_string(),
            assignee: "VibeWindow".to_string(),
            model: TASK_MODEL_AUTO.to_string(),
            executor: TaskExecutorBackend::default(),
            description: String::new(),
            prompt: String::new(),
            subtasks: vec![String::new(), String::new(), String::new()],
            task_split_mode: TaskSplitMode::FollowSystem,
            auto_promote_delay_seconds: "0".to_string(),
        }
    }
}

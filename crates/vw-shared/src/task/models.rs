//! 任务模型模块，定义任务、子任务、日志、看板设置和执行器选项的持久化结构。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering};
use time::OffsetDateTime;

/// TASK_MODEL_AUTO 提供跨模块复用的稳定常量。
pub const TASK_MODEL_AUTO: &str = "auto";
/// CLAUDE_DEFAULT_MODEL_ALIAS 提供跨模块复用的稳定常量。
pub const CLAUDE_DEFAULT_MODEL_ALIAS: &str = "default";
/// CLAUDE_SUPPORTED_MODEL_ALIASES 提供跨模块复用的稳定常量。
pub const CLAUDE_SUPPORTED_MODEL_ALIASES: &[&str] =
    &[CLAUDE_DEFAULT_MODEL_ALIAS, "sonnet", "opus", "haiku"];

static SUBTASK_ID_SEQUENCE: AtomicU32 = AtomicU32::new(0);

/// 规范化 task model input 输入。
pub fn normalize_task_model_input(model: &str) -> String {
    let trimmed = model.trim();
    if trimmed.is_empty() { TASK_MODEL_AUTO.to_string() } else { trimmed.to_string() }
}

/// 解析 Claude 模型别名。
///
/// 返回值为稳定别名；未知模型返回空值，让调用方保留原始输入或报错。
pub fn claude_model_alias(model: &str) -> Option<&'static str> {
    match model.trim().to_ascii_lowercase().as_str() {
        "" | TASK_MODEL_AUTO | CLAUDE_DEFAULT_MODEL_ALIAS => Some(CLAUDE_DEFAULT_MODEL_ALIAS),
        "sonnet" => Some("sonnet"),
        "opus" => Some("opus"),
        "haiku" => Some("haiku"),
        _ => None,
    }
}

/// TaskExecutorBackend 枚举描述该模块支持的 TaskExecutorBackend 取值集合。
///
/// 每个变体代表一个明确分支，调用方应通过显式匹配处理新增状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TaskExecutorBackend {
    #[default]
    Internal,
    OpenCode,
    Claude,
    Codex,
}

impl TaskExecutorBackend {
    /// 提供 all 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn all() -> [TaskExecutorBackend; 4] {
        [
            TaskExecutorBackend::Internal,
            TaskExecutorBackend::OpenCode,
            TaskExecutorBackend::Claude,
            TaskExecutorBackend::Codex,
        ]
    }

    /// 提供 label 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn label(&self) -> &'static str {
        match self {
            TaskExecutorBackend::Internal => "内置执行器",
            TaskExecutorBackend::OpenCode => "OpenCode",
            TaskExecutorBackend::Claude => "Claude Code",
            TaskExecutorBackend::Codex => "Codex CLI",
        }
    }

    /// 提供 id 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn id(&self) -> &'static str {
        match self {
            TaskExecutorBackend::Internal => "internal",
            TaskExecutorBackend::OpenCode => "opencode",
            TaskExecutorBackend::Claude => "claude",
            TaskExecutorBackend::Codex => "codex",
        }
    }

    /// 提供 from id 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn from_id(s: &str) -> Option<TaskExecutorBackend> {
        match s {
            "internal" | "Internal" | "内置执行器" => Some(TaskExecutorBackend::Internal),
            "opencode" | "OpenCode" => Some(TaskExecutorBackend::OpenCode),
            "claude" | "Claude" | "Claude Code" => Some(TaskExecutorBackend::Claude),
            "codex" | "Codex" | "Codex CLI" => Some(TaskExecutorBackend::Codex),
            _ => None,
        }
    }
}

/// TaskSplitMode 枚举描述任务拆分开关如何覆盖系统设置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskSplitMode {
    #[default]
    FollowSystem,
    Enabled,
    Disabled,
}

impl TaskSplitMode {
    /// 返回所有可选模式，供 UI 和 CLI 复用。
    pub fn all() -> [TaskSplitMode; 3] {
        [TaskSplitMode::FollowSystem, TaskSplitMode::Enabled, TaskSplitMode::Disabled]
    }

    /// 返回面向用户的稳定标签。
    pub fn label(self) -> &'static str {
        match self {
            TaskSplitMode::FollowSystem => "遵循系统",
            TaskSplitMode::Enabled => "开启",
            TaskSplitMode::Disabled => "关闭",
        }
    }

    /// 返回持久化和 CLI 使用的稳定 key。
    pub fn to_string_key(self) -> &'static str {
        match self {
            TaskSplitMode::FollowSystem => "follow_system",
            TaskSplitMode::Enabled => "enabled",
            TaskSplitMode::Disabled => "disabled",
        }
    }

    /// 解析外部输入。
    ///
    /// 返回结构化结果；无法识别的输入显式返回空值。
    pub fn parse_key(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "follow_system" | "follow-system" | "system" | "default" | "inherit" => {
                Some(TaskSplitMode::FollowSystem)
            }
            "enabled" | "enable" | "on" | "true" | "yes" | "1" => Some(TaskSplitMode::Enabled),
            "disabled" | "disable" | "off" | "false" | "no" | "0" => Some(TaskSplitMode::Disabled),
            _ => None,
        }
    }

    /// 结合系统开关计算最终是否执行任务拆分。
    pub fn enabled_with_system(self, system_enabled: bool) -> bool {
        match self {
            TaskSplitMode::FollowSystem => system_enabled,
            TaskSplitMode::Enabled => true,
            TaskSplitMode::Disabled => false,
        }
    }
}

impl fmt::Display for TaskSplitMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// TaskImportPromptFormat 枚举描述该模块支持的 TaskImportPromptFormat 取值集合。
///
/// 每个变体代表一个明确分支，调用方应通过显式匹配处理新增状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TaskImportPromptFormat {
    #[default]
    Json,
    Csv,
    Tsv,
}

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

fn now_ms() -> u64 {
    crate::time::now_ms()
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

/// SubTask 数据结构承载该模块对外传递的 SubTask 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: String,
    pub content: String,
    #[serde(default)]
    pub boundary: String,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub target_files: Vec<String>,
    pub created_at_ms: u64,
    pub order: u32,
    pub completed: bool,
    #[serde(default = "default_subtask_status")]
    pub status: SubTaskStatus,
    #[serde(default)]
    pub execution_started_at_ms: Option<u64>,
    #[serde(default)]
    pub last_execution_duration_ms: Option<u64>,
}

/// 子任务执行状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SubTaskStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
}

fn default_subtask_status() -> SubTaskStatus {
    SubTaskStatus::Pending
}

impl SubTask {
    /// 提供 new 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub fn new(content: String) -> Self {
        let ms = now_ms();
        let seq = SUBTASK_ID_SEQUENCE.fetch_add(1, Ordering::Relaxed) % 100000;
        Self {
            id: format!("SUB-{ms}.{seq:05}"),
            content,
            boundary: String::new(),
            acceptance_criteria: Vec::new(),
            target_files: Vec::new(),
            created_at_ms: ms,
            order: 0,
            completed: false,
            status: SubTaskStatus::Pending,
            execution_started_at_ms: None,
            last_execution_duration_ms: None,
        }
    }

    pub fn start_execution(&mut self) {
        self.status = SubTaskStatus::Running;
        self.completed = false;
        self.execution_started_at_ms = Some(now_ms());
        self.last_execution_duration_ms = None;
    }

    pub fn mark_completed(&mut self) {
        if let Some(started_at) = self.execution_started_at_ms {
            self.last_execution_duration_ms = Some(now_ms().saturating_sub(started_at));
        }
        self.execution_started_at_ms = None;
        self.status = SubTaskStatus::Completed;
        self.completed = true;
    }

    pub fn mark_failed(&mut self) {
        if let Some(started_at) = self.execution_started_at_ms {
            self.last_execution_duration_ms = Some(now_ms().saturating_sub(started_at));
        }
        self.execution_started_at_ms = None;
        self.status = SubTaskStatus::Failed;
        self.completed = false;
    }

    pub fn display_execution_duration_ms(&self, now_ms: u64) -> Option<u64> {
        if self.status == SubTaskStatus::Running {
            let started_at = self.execution_started_at_ms.unwrap_or(self.created_at_ms);
            return Some(now_ms.saturating_sub(started_at));
        }
        self.last_execution_duration_ms
    }
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

#[cfg(test)]
#[path = "models_tests.rs"]
mod models_tests;

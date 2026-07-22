//! 任务领域模型与任务输入规范化逻辑。
//!
//! 该模块定义桌面端任务流转使用的数据结构、状态枚举和兼容旧配置的转换函数，是执行器、存储和界面之间的契约层。

use crate::app::task::workflow::{TaskStateMachine, TaskTransitionError, TaskTransitionIntent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use time::OffsetDateTime;
use vw_gateway_client::vw_api_types::ReasoningEffortDto;
use vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerDispatchModeDto;

/// 公开的 TASK_MODEL_AUTO 常量，集中保存该模块复用的稳定取值。
pub const TASK_MODEL_AUTO: &str = "auto";
pub const TASK_AGENT_MAIN: &str = "main";
/// 公开的 CLAUDE_DEFAULT_MODEL_ALIAS 常量，集中保存该模块复用的稳定取值。
pub const CLAUDE_DEFAULT_MODEL_ALIAS: &str = "default";
/// 公开的 CLAUDE_SUPPORTED_MODEL_ALIASES 常量，集中保存该模块复用的稳定取值。
pub const CLAUDE_SUPPORTED_MODEL_ALIASES: &[&str] =
    &[CLAUDE_DEFAULT_MODEL_ALIAS, "sonnet", "opus", "haiku"];

static SUBTASK_ID_SEQUENCE: AtomicU32 = AtomicU32::new(0);
static LAST_TASK_ID_MILLIS: AtomicU64 = AtomicU64::new(0);

/// 公开的 normalize_task_model_input 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn normalize_task_model_input(model: &str) -> String {
    let trimmed = model.trim();
    if trimmed.is_empty() { TASK_MODEL_AUTO.to_string() } else { trimmed.to_string() }
}

/// 公开的 claude_model_alias 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn claude_model_alias(model: &str) -> Option<&'static str> {
    match model.trim().to_ascii_lowercase().as_str() {
        "" | TASK_MODEL_AUTO | CLAUDE_DEFAULT_MODEL_ALIAS => Some(CLAUDE_DEFAULT_MODEL_ALIAS),
        "sonnet" => Some("sonnet"),
        "opus" => Some("opus"),
        "haiku" => Some("haiku"),
        _ => None,
    }
}

/// 公开的 normalize_task_acp_agent_input 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn normalize_task_acp_agent_input(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    match trimmed.to_ascii_lowercase().as_str() {
        "default" | "acp" | "acp 智能体" | "acp 网关" | "内置执行器" | "internal" => None,
        _ => Some(trimmed.to_string()),
    }
}

/// 公开的 legacy_executor_to_task_acp_agent 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn legacy_executor_to_task_acp_agent(executor: TaskExecutorBackend) -> Option<String> {
    match executor {
        TaskExecutorBackend::Internal => None,
        TaskExecutorBackend::OpenCode => Some("opencode".to_string()),
        TaskExecutorBackend::Claude => Some("claude".to_string()),
        TaskExecutorBackend::Codex => Some("codex".to_string()),
    }
}

/// 公开的 TaskExecutorBackend 枚举，描述该模块支持的一组离散状态或事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TaskExecutorBackend {
    #[default]
    Internal,
    OpenCode,
    Claude,
    Codex,
}

impl TaskExecutorBackend {
    /// 公开的 all 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn all() -> [TaskExecutorBackend; 4] {
        [
            TaskExecutorBackend::Internal,
            TaskExecutorBackend::OpenCode,
            TaskExecutorBackend::Claude,
            TaskExecutorBackend::Codex,
        ]
    }

    /// 公开的 label 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn label(&self) -> &'static str {
        match self {
            TaskExecutorBackend::Internal => "ACP 智能体",
            TaskExecutorBackend::OpenCode => "OpenCode",
            TaskExecutorBackend::Claude => "Claude Code",
            TaskExecutorBackend::Codex => "Codex CLI",
        }
    }

    /// 公开的 id 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn id(&self) -> &'static str {
        match self {
            TaskExecutorBackend::Internal => "internal",
            TaskExecutorBackend::OpenCode => "opencode",
            TaskExecutorBackend::Claude => "claude",
            TaskExecutorBackend::Codex => "codex",
        }
    }

    /// 公开的 from_id 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn from_id(s: &str) -> Option<TaskExecutorBackend> {
        match s {
            "internal" | "Internal" | "内置执行器" | "ACP 智能体" | "ACP 网关" | "acp_gateway" => {
                Some(TaskExecutorBackend::Internal)
            }
            "opencode" | "OpenCode" => Some(TaskExecutorBackend::OpenCode),
            "claude" | "Claude" | "Claude Code" => Some(TaskExecutorBackend::Claude),
            "codex" | "Codex" | "Codex CLI" => Some(TaskExecutorBackend::Codex),
            _ => None,
        }
    }
}

/// 公开的 TaskImportPromptFormat 枚举，描述该模块支持的一组离散状态或事件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TaskImportPromptFormat {
    #[default]
    Json,
    Csv,
    Tsv,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskImportRecord {
    pub priority: Option<u32>,
    pub prompt: String,
    pub model: Option<String>,
    pub agent: Option<String>,
    pub acp_agent: Option<String>,
    pub reasoning_effort: Option<ReasoningEffortDto>,
    pub task_split_mode: Option<TaskSplitMode>,
}

fn parse_import_reasoning_effort(
    value: Option<&str>,
) -> Result<Option<ReasoningEffortDto>, String> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    serde_json::from_value(serde_json::Value::String(value.to_ascii_lowercase()))
        .map(Some)
        .map_err(|_| format!("非法 reasoning_effort 值: {value}"))
}

fn parse_import_reasoning_effort_value(
    value: Option<&serde_json::Value>,
) -> Result<Option<ReasoningEffortDto>, String> {
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(value)) => parse_import_reasoning_effort(Some(value)),
        Some(_) => Err("非法 reasoning_effort 值: 必须是字符串或 null".to_string()),
    }
}

pub fn parse_task_import_content(content: &str) -> Result<Vec<TaskImportRecord>, String> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
        let items = value.as_array().ok_or_else(|| "JSON 导入内容必须是数组".to_string())?;
        return items
            .iter()
            .map(|item| {
                let object = item
                    .as_object()
                    .ok_or_else(|| "JSON 导入数组的每一项必须是对象".to_string())?;
                let string = |key: &str| {
                    object
                        .get(key)
                        .and_then(serde_json::Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string)
                };
                Ok(TaskImportRecord {
                    priority: object
                        .get("priority")
                        .and_then(serde_json::Value::as_u64)
                        .and_then(|value| u32::try_from(value).ok()),
                    prompt: string("prompt").unwrap_or_default(),
                    model: string("model"),
                    agent: string("agent"),
                    acp_agent: string("acp_agent"),
                    reasoning_effort: parse_import_reasoning_effort_value(
                        object.get("reasoning_effort"),
                    )?,
                    task_split_mode: object
                        .get("task_split_mode")
                        .or_else(|| object.get("task_split"))
                        .and_then(serde_json::Value::as_str)
                        .and_then(TaskSplitMode::parse_key),
                })
            })
            .collect();
    }

    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| "导入内容为空".to_string())?;
    let delimiter = if header.contains('\t') { '\t' } else { ',' };
    let headers = header
        .split(delimiter)
        .map(|value| value.trim().trim_matches('"').to_ascii_lowercase())
        .collect::<Vec<_>>();
    let column =
        |names: &[&str]| headers.iter().position(|header| names.contains(&header.as_str()));
    let prompt_idx =
        column(&["prompt", "提示词"]).ok_or_else(|| "导入内容缺少 prompt 字段".to_string())?;
    let priority_idx = column(&["priority", "优先级"]);
    let model_idx = column(&["model", "模型"]);
    let agent_idx = column(&["agent", "代理"]);
    let acp_agent_idx = column(&["acp_agent", "智能体", "acp智能体"]);
    let reasoning_effort_idx = column(&["reasoning_effort", "推理强度"]);
    let task_split_idx = column(&["task_split_mode", "task_split", "任务拆分"]);
    lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let cells = line
                .split(delimiter)
                .map(|value| value.trim().trim_matches('"'))
                .collect::<Vec<_>>();
            let cell = |index: Option<usize>| index.and_then(|index| cells.get(index).copied());
            Ok(TaskImportRecord {
                priority: cell(priority_idx).and_then(|value| value.parse().ok()),
                prompt: cell(Some(prompt_idx)).unwrap_or_default().to_string(),
                model: cell(model_idx)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                agent: cell(agent_idx)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                acp_agent: cell(acp_agent_idx)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                reasoning_effort: parse_import_reasoning_effort(cell(reasoning_effort_idx))?,
                task_split_mode: cell(task_split_idx).and_then(TaskSplitMode::parse_key),
            })
        })
        .collect()
}

/// 公开的 TaskSplitMode 枚举，描述任务拆分开关如何覆盖系统设置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskSplitMode {
    #[default]
    FollowSystem,
    Enabled,
    Disabled,
}

impl TaskSplitMode {
    pub fn all() -> [TaskSplitMode; 3] {
        [TaskSplitMode::FollowSystem, TaskSplitMode::Enabled, TaskSplitMode::Disabled]
    }

    pub fn label(self) -> &'static str {
        match self {
            TaskSplitMode::FollowSystem => "遵循系统",
            TaskSplitMode::Enabled => "开启",
            TaskSplitMode::Disabled => "关闭",
        }
    }

    pub fn to_string_key(self) -> &'static str {
        match self {
            TaskSplitMode::FollowSystem => "follow_system",
            TaskSplitMode::Enabled => "enabled",
            TaskSplitMode::Disabled => "disabled",
        }
    }

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

/// 进程级自定义状态 key 驻留表。
///
/// 把任意稳定 key 去重为唯一的 `&'static str`，使 `TaskStatus::Custom` 能以 `Copy`
/// 载荷承载自定义状态，避免破坏 `TaskStatus: Copy` 契约与全链路按值传递的既有代码。
/// 驻留集合受用户自定义状态的有限基数约束，重复 key 直接复用同一引用，不会无界泄漏。
fn intern_status_key(key: &str) -> &'static str {
    use std::collections::HashSet;
    use std::sync::{LazyLock, Mutex};

    static INTERNED: LazyLock<Mutex<HashSet<&'static str>>> =
        LazyLock::new(|| Mutex::new(HashSet::new()));

    let mut set = INTERNED.lock().expect("status key interner mutex poisoned");
    if let Some(existing) = set.get(key) {
        return existing;
    }
    let leaked: &'static str = Box::leak(key.to_owned().into_boxed_str());
    set.insert(leaked);
    leaked
}

/// 公开的 TaskStatus 枚举，描述该模块支持的一组离散状态或事件。
///
/// 11 个内置变体是引擎规范词汇；`Custom` 承载用户自定义状态的稳定 key，
/// 以驻留得到的 `&'static str` 保持整体 `Copy`。序列化对内置变体保持历史
/// PascalCase 线格式（旧 JSON 存档兼容），对 `Custom` 直接写出其原始 key 字符串。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TaskStatus {
    #[default]
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
    /// 用户自定义状态，载荷为驻留后的稳定 key。
    Custom(&'static str),
}

impl TaskStatus {
    /// 公开的 all 函数。
    ///
    /// 委托到默认工作流元数据，返回有序的状态集合，保持外部签名与顺序不变。
    pub fn all() -> [TaskStatus; 11] {
        let definitions = crate::app::task::workflow::default_task_workflow_statuses();
        let mut statuses = [TaskStatus::Pool; 11];
        for (slot, definition) in statuses.iter_mut().zip(definitions.iter()) {
            *slot = definition.status;
        }
        statuses
    }

    /// 按工作流 id 返回有序状态集合；未知 id 回退默认工作流。
    pub fn all_for_workflow_id(workflow_id: &str) -> Vec<TaskStatus> {
        crate::app::task::workflow::task_workflow_by_id(workflow_id)
            .or_else(|| {
                crate::app::task::workflow::task_workflow_by_id(
                    crate::app::task::workflow::DEFAULT_TASK_WORKFLOW_ID,
                )
            })
            .map(|workflow| {
                workflow.statuses.into_iter().map(|definition| definition.status).collect()
            })
            .unwrap_or_else(|| Self::all().to_vec())
    }

    /// 公开的 label 函数。
    ///
    /// 委托到默认工作流元数据返回展示标签；自定义状态默认工作流无定义，回退到自身 key。
    pub fn label(&self) -> &'static str {
        if let TaskStatus::Custom(key) = self {
            return key;
        }
        crate::app::task::workflow::default_task_workflow()
            .status_definition(*self)
            .map(|definition| definition.label)
            .unwrap_or_default()
    }

    /// 按工作流 id 返回展示标签；未知工作流或工作流未定义该状态时回退默认标签。
    pub fn label_for_workflow_id(&self, workflow_id: &str) -> String {
        crate::app::task::workflow::task_workflow_by_id(workflow_id)
            .or_else(|| {
                crate::app::task::workflow::task_workflow_by_id(
                    crate::app::task::workflow::DEFAULT_TASK_WORKFLOW_ID,
                )
            })
            .and_then(|workflow| {
                workflow.status_definition(*self).map(|definition| definition.label.clone())
            })
            .unwrap_or_else(|| self.label().to_string())
    }

    /// 公开的 next 函数。
    ///
    /// 委托到默认工作流元数据返回默认下一状态。
    pub fn next(&self) -> Option<TaskStatus> {
        crate::app::task::workflow::default_task_workflow().next_status(*self)
    }

    /// 按工作流 id 返回默认下一状态；未知工作流或工作流未定义该状态时回退默认工作流。
    pub fn next_for_workflow_id(&self, workflow_id: &str) -> Option<TaskStatus> {
        if let Some(workflow) = crate::app::task::workflow::task_workflow_by_id(workflow_id)
            .or_else(|| {
                crate::app::task::workflow::task_workflow_by_id(
                    crate::app::task::workflow::DEFAULT_TASK_WORKFLOW_ID,
                )
            })
        {
            if workflow.status_definition(*self).is_some() {
                return workflow.next_status(*self);
            }
        }
        self.next()
    }

    /// 公开的 parse_key 函数。
    ///
    /// 把稳定 key 反解为状态：命中内置变体返回对应枚举；非空的未知 key 驻留为
    /// `Custom` 并保留原始 key；空字符串无法表达任何状态，返回 `None`。
    pub fn parse_key(s: &str) -> Option<TaskStatus> {
        match s {
            "pool" | "Pool" => Some(TaskStatus::Pool),
            "pending" | "Pending" => Some(TaskStatus::Pending),
            "planning" | "Planning" => Some(TaskStatus::Planning),
            "running" | "Running" => Some(TaskStatus::Running),
            "failed" | "Failed" => Some(TaskStatus::Failed),
            "paused" | "Paused" => Some(TaskStatus::Paused),
            "code_complete" | "CodeComplete" => Some(TaskStatus::CodeComplete),
            "pr_submitted" | "PrSubmitted" => Some(TaskStatus::PrSubmitted),
            "code_review" | "CodeReview" => Some(TaskStatus::CodeReview),
            "completed" | "Completed" => Some(TaskStatus::Completed),
            "archived" | "Archived" => Some(TaskStatus::Archived),
            "" => None,
            other => Some(TaskStatus::Custom(intern_status_key(other))),
        }
    }

    /// 公开的 to_string_key 函数。
    ///
    /// 委托到默认工作流元数据返回稳定序列化 key；自定义状态直接返回其驻留 key。
    pub fn to_string_key(&self) -> &'static str {
        if let TaskStatus::Custom(key) = self {
            return key;
        }
        crate::app::task::workflow::default_task_workflow()
            .status_definition(*self)
            .map(|definition| definition.key)
            .unwrap_or_default()
    }

    /// 返回 serde 线格式所用的字符串：内置变体为历史 PascalCase 变体名（保持旧存档逐字节兼容），
    /// 自定义状态为其原始 key。
    fn serde_name(&self) -> &'static str {
        match self {
            TaskStatus::Pool => "Pool",
            TaskStatus::Pending => "Pending",
            TaskStatus::Planning => "Planning",
            TaskStatus::Running => "Running",
            TaskStatus::Failed => "Failed",
            TaskStatus::Paused => "Paused",
            TaskStatus::CodeComplete => "CodeComplete",
            TaskStatus::CodeReview => "CodeReview",
            TaskStatus::PrSubmitted => "PrSubmitted",
            TaskStatus::Completed => "Completed",
            TaskStatus::Archived => "Archived",
            TaskStatus::Custom(key) => key,
        }
    }
}

impl Serialize for TaskStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.serde_name())
    }
}

impl<'de> Deserialize<'de> for TaskStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TaskStatusVisitor;

        impl serde::de::Visitor<'_> for TaskStatusVisitor {
            type Value = TaskStatus;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a task status key string")
            }

            fn visit_str<E>(self, value: &str) -> Result<TaskStatus, E>
            where
                E: serde::de::Error,
            {
                // 内置变体（含历史 PascalCase 与 snake_case 两种写法）映射回枚举，
                // 非空未知 key 驻留为 Custom；空字符串不是合法状态。
                TaskStatus::parse_key(value)
                    .ok_or_else(|| E::custom("empty task status key is not allowed"))
            }
        }

        deserializer.deserialize_str(TaskStatusVisitor)
    }
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

const fn default_subtask_batch() -> u32 {
    1
}

fn now_ms() -> u64 {
    crate::app::time::now_ms()
}

fn format_task_date_from_ms(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let dt = OffsetDateTime::from_unix_timestamp(secs).unwrap_or(OffsetDateTime::UNIX_EPOCH);
    let month: u8 = dt.month().into();
    format!("{:04}{:02}{:02}", dt.year(), month, dt.day())
}

fn generate_task_id() -> String {
    let current_ms = now_ms();
    let ms = LAST_TASK_ID_MILLIS
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |previous| {
            Some(current_ms.max(previous.saturating_add(1)))
        })
        .map(|previous| current_ms.max(previous.saturating_add(1)))
        .unwrap_or(current_ms);
    let date = format_task_date_from_ms(ms);
    let seq = (ms % 10000) as u32;
    format!("T{}.{:04}", date, seq)
}

/// 公开的 TaskLogEntry 结构体，承载该模块边界内传递的结构化状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLogEntry {
    pub timestamp_ms: u64,
    pub status_from: Option<TaskStatus>,
    pub status_to: Option<TaskStatus>,
    pub message: String,
}

/// 公开的 SubTask 结构体，承载该模块边界内传递的结构化状态。
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
    #[serde(default = "default_subtask_batch")]
    pub batch: u32,
    #[serde(default)]
    pub depends_on_batches: Vec<u32>,
    #[serde(default = "default_subtask_status")]
    pub status: SubTaskStatus,
    #[serde(default)]
    pub execution_started_at_ms: Option<u64>,
    #[serde(default)]
    pub last_execution_duration_ms: Option<u64>,
}

impl SubTask {
    /// 公开的 new 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
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
            batch: 1,
            depends_on_batches: Vec::new(),
            status: SubTaskStatus::Pending,
            execution_started_at_ms: None,
            last_execution_duration_ms: None,
        }
    }

    pub fn start_execution_at(&mut self, started_at_ms: u64) {
        self.status = SubTaskStatus::Running;
        self.completed = false;
        self.execution_started_at_ms = Some(started_at_ms);
    }

    pub fn start_execution(&mut self) {
        self.start_execution_at(now_ms());
    }

    pub fn mark_completed_at(&mut self, completed_at_ms: u64) {
        if let Some(started_at) = self.execution_started_at_ms {
            self.last_execution_duration_ms = Some(completed_at_ms.saturating_sub(started_at));
        }
        self.execution_started_at_ms = None;
        self.status = SubTaskStatus::Completed;
        self.completed = true;
    }

    pub fn mark_completed(&mut self) {
        self.mark_completed_at(now_ms());
    }

    pub fn mark_failed_at(&mut self, failed_at_ms: u64) {
        if let Some(started_at) = self.execution_started_at_ms {
            self.last_execution_duration_ms = Some(failed_at_ms.saturating_sub(started_at));
        }
        self.execution_started_at_ms = None;
        self.status = SubTaskStatus::Failed;
        self.completed = false;
    }

    pub fn mark_failed(&mut self) {
        self.mark_failed_at(now_ms());
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
    /// 公开的 new_status_change 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn new_status_change(from: TaskStatus, to: TaskStatus) -> Self {
        Self {
            timestamp_ms: now_ms(),
            status_from: Some(from),
            status_to: Some(to),
            message: format!("状态变更: {} → {}", from.label(), to.label()),
        }
    }

    /// 公开的 new_message 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn new_message(message: String) -> Self {
        Self { timestamp_ms: now_ms(), status_from: None, status_to: None, message }
    }
}

/// 公开的 Task 结构体，承载该模块边界内传递的结构化状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub priority: u32,
    pub assignee: String,
    pub model: String,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub acp_agent: Option<String>,
    #[serde(default)]
    pub reasoning_effort: Option<ReasoningEffortDto>,
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
    pub selected_worktree_slot_id: Option<String>,
    #[serde(default)]
    pub selected_worktree_path: Option<String>,
    #[serde(default)]
    pub selected_worktree_branch: Option<String>,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            id: generate_task_id(),
            priority: 999,
            assignee: "VibeWindow".to_string(),
            model: TASK_MODEL_AUTO.to_string(),
            agent: Some(TASK_AGENT_MAIN.to_string()),
            acp_agent: None,
            reasoning_effort: None,
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
            selected_worktree_slot_id: None,
            selected_worktree_path: None,
            selected_worktree_branch: None,
        }
    }
}

impl Task {
    /// 公开的 new 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn new(priority: u32) -> Self {
        let now = now_ms();
        Self {
            id: generate_task_id(),
            priority,
            assignee: "VibeWindow".to_string(),
            model: TASK_MODEL_AUTO.to_string(),
            agent: Some(TASK_AGENT_MAIN.to_string()),
            acp_agent: None,
            reasoning_effort: None,
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
            selected_worktree_slot_id: None,
            selected_worktree_path: None,
            selected_worktree_branch: None,
        }
    }

    /// 公开的 set_status 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
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

    /// 按转移意图经默认工作流状态机校验后写入状态。
    ///
    /// 校验通过才调用 `set_status` 落状态；被拒绝时不改状态、写一条清晰拒绝日志，
    /// 并返回携带 from、to、intent 与工作流 id 的错误，交由调用方按安全默认处理，
    /// 不在执行器结果路径伪装成功。
    pub fn apply_task_transition(
        &mut self,
        new_status: TaskStatus,
        intent: TaskTransitionIntent,
    ) -> Result<(), TaskTransitionError> {
        let machine = TaskStateMachine::default_lifecycle();
        if let Err(error) = machine.validate_transition(self.status, new_status, intent) {
            self.add_log(format!("状态转移被状态机拒绝: {error}"));
            return Err(error);
        }
        self.set_status(new_status);
        Ok(())
    }

    /// 公开的 add_log 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn add_log(&mut self, message: String) {
        let now = now_ms();
        self.logs.push(TaskLogEntry::new_message(message));
        self.updated_at_ms = now;
        self.last_active_at_ms = now;
    }

    /// 公开的 start_execution 函数。
    ///
    /// 先经状态机校验 `StartExecution` 来源状态（仅 Pending/Planning/Failed 合法），
    /// 通过后才推进到 Running 并维护重试、分支与执行起始时间字段；来源非法时不改状态、
    /// 不消耗重试次数，写拒绝日志并返回错误，由调用方决定安全默认，不伪装已开始执行。
    pub fn start_execution(&mut self, trigger_log: String) -> Result<(), TaskTransitionError> {
        let machine = TaskStateMachine::default_lifecycle();
        if let Err(error) = machine.validate_transition(
            self.status,
            TaskStatus::Running,
            TaskTransitionIntent::StartExecution,
        ) {
            self.add_log(format!("拒绝开始执行: {error}"));
            return Err(error);
        }
        if self.last_error.is_some() {
            self.retry_count = self.retry_count.saturating_add(1);
            self.add_log(format!("进入重试，第 {} 次", self.retry_count));
        }
        self.pause_reason = None;
        self.merge_source_branch = None;
        self.execution_started_at_ms = Some(now_ms());
        self.set_status(TaskStatus::Running);
        self.add_log(trigger_log);
        Ok(())
    }

    /// 公开的 start_merge_execution 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn start_merge_execution(&mut self, trigger_log: String) {
        let now = now_ms();
        self.pause_reason = None;
        self.execution_started_at_ms = Some(now);
        self.updated_at_ms = now;
        self.last_active_at_ms = now;
        self.add_log(trigger_log);
    }

    /// 公开的 should_auto_merge 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn should_auto_merge(&self) -> bool {
        self.merge_source_branch.as_deref().is_some_and(|value| !value.trim().is_empty())
            && self.merge_target_branch.as_deref().is_some_and(|value| !value.trim().is_empty())
    }

    /// 公开的 mark_execution_failed 函数。
    ///
    /// 失败是执行器结果路径的安全默认落点（Failed 可经 `Failed -> Running` 重试），
    /// 不走前进边校验：无论来源状态如何都直接落 Failed，避免把执行前预分配失败等
    /// 非 Running 来源的失败困死在原状态。
    pub fn mark_execution_failed(&mut self, error: String) {
        self.last_error = Some(error.clone());
        if let Some(started_at) = self.execution_started_at_ms {
            self.last_execution_duration_ms = Some(now_ms().saturating_sub(started_at));
        }
        self.execution_started_at_ms = None;
        self.set_status(TaskStatus::Failed);
        self.add_log(format!("失败原因: {}", error));
    }

    /// 公开的 mark_execution_succeeded 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn mark_execution_succeeded(&mut self) {
        self.last_error = None;
        self.pause_reason = None;
        if let Some(started_at) = self.execution_started_at_ms {
            self.last_execution_duration_ms = Some(now_ms().saturating_sub(started_at));
        }
        self.execution_started_at_ms = None;
    }

    /// 公开的 mark_paused 函数。
    ///
    /// 暂停是状态机拒绝或审核/合并校验失败时的安全默认落点，不走前进边校验：
    /// 任意来源都直接落 Paused，保证拒绝路径有确定终点，不伪装成功继续推进。
    pub fn mark_paused(&mut self, reason: String) {
        self.pause_reason = Some(reason.clone());
        if let Some(started_at) = self.execution_started_at_ms {
            self.last_execution_duration_ms = Some(now_ms().saturating_sub(started_at));
        }
        self.execution_started_at_ms = None;
        self.set_status(TaskStatus::Paused);
        self.add_log(format!("暂停原因: {}", reason));
    }

    /// 公开的 running_duration_ms 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn running_duration_ms(&self, now_ms: u64) -> Option<u64> {
        if self.status != TaskStatus::Running {
            return None;
        }
        let started_at = self.execution_started_at_ms.unwrap_or(self.last_active_at_ms);
        Some(now_ms.saturating_sub(started_at))
    }

    /// 公开的 display_execution_duration_ms 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn display_execution_duration_ms(&self, now_ms: u64) -> Option<u64> {
        self.running_duration_ms(now_ms).or(self.last_execution_duration_ms)
    }
}

/// 公开的 TaskIndex 结构体，承载该模块边界内传递的结构化状态。
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
    /// 公开的 new 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn new() -> Self {
        let mut order_by_status = HashMap::new();
        for status in TaskStatus::all() {
            order_by_status.insert(status.to_string_key().to_string(), Vec::new());
        }
        Self { tasks: HashMap::new(), order_by_status, last_task_date: None, last_task_seq: 0 }
    }
}

/// 公开的 TaskBoardSettings 结构体，承载该模块边界内传递的结构化状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBoardSettings {
    pub max_concurrent: u32,
    pub default_priority: u32,
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
    #[serde(default)]
    pub agent_scheduler_dispatch_mode: AgentSchedulerDispatchModeDto,
    /// 项目选用的任务工作流 id，默认 `dev_lifecycle`；旧配置缺字段时落默认。
    #[serde(default = "default_task_workflow_id")]
    pub workflow_id: String,
}

impl TaskBoardSettings {
    /// 公开的 new 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
    pub fn new() -> Self {
        Self {
            max_concurrent: 3,
            default_priority: 999,
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
            agent_scheduler_dispatch_mode: AgentSchedulerDispatchModeDto::Local,
            workflow_id: default_task_workflow_id(),
        }
    }

    /// 公开的 sanitized 函数。
    ///
    /// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
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
        if crate::app::task::workflow::task_workflow_by_id(&self.workflow_id).is_none() {
            self.workflow_id = default_task_workflow_id();
        }
        self
    }
}

impl Default for TaskBoardSettings {
    fn default() -> Self {
        Self::new()
    }
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

fn default_task_workflow_id() -> String {
    crate::app::task::workflow::DEFAULT_TASK_WORKFLOW_ID.to_string()
}

/// 公开的 TaskDraft 结构体，承载该模块边界内传递的结构化状态。
#[derive(Debug, Clone)]
pub struct TaskDraft {
    pub priority: String,
    pub assignee: String,
    pub model: String,
    pub agent: Option<String>,
    pub acp_agent: Option<String>,
    pub reasoning_effort: Option<ReasoningEffortDto>,
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
            agent: Some(TASK_AGENT_MAIN.to_string()),
            acp_agent: None,
            reasoning_effort: None,
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

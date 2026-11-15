use serde::{Deserialize, Serialize};
use std::fmt;

/// TASK_MODEL_AUTO 提供跨模块复用的稳定常量。
pub const TASK_MODEL_AUTO: &str = "auto";
/// CLAUDE_DEFAULT_MODEL_ALIAS 提供跨模块复用的稳定常量。
pub const CLAUDE_DEFAULT_MODEL_ALIAS: &str = "default";
/// CLAUDE_SUPPORTED_MODEL_ALIASES 提供跨模块复用的稳定常量。
pub const CLAUDE_SUPPORTED_MODEL_ALIASES: &[&str] =
    &[CLAUDE_DEFAULT_MODEL_ALIAS, "sonnet", "opus", "haiku"];

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

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

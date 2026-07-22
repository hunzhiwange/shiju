//! 任务面板共享模块。
//!
//! 提供任务看板、子任务、执行后端和持久化访问接口，供桌面端与代理端共享。

pub mod models;
pub mod store;

/// 任务领域模型及辅助常量。
pub use models::{
    CLAUDE_DEFAULT_MODEL_ALIAS, CLAUDE_SUPPORTED_MODEL_ALIASES, SubTask, SubTaskStatus,
    TASK_MODEL_AUTO, Task, TaskBoardSettings, TaskDraft, TaskExecutorBackend,
    TaskImportPromptFormat, TaskIndex, TaskLogEntry, TaskSplitMode, TaskStatus, claude_model_alias,
    normalize_task_model_input,
};
/// 任务持久化读写接口。
pub use store::*;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

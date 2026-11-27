//! 任务看板消息处理模块
//!
//! 本模块负责处理任务看板的所有 UI 消息和后台调度逻辑，包括：
//! - 任务的创建、编辑、删除和归档
//! - 任务执行引擎的调度（自动执行、超时处理、并发控制）
//! - Git 合并流程（分支合并、锁管理、冲突处理）
//! - Worktree 池管理（工作区快照、维护、回收）
//! - 任务导入（JSON/CSV/TSV 格式解析）
//!
//! # 架构概述
//!
//! 任务看板模块继续保留 `TaskBoardMessage` 与 `update` 作为外部入口，
//! 但将实现按职责拆分为独立子文件：
//! - `helpers`：辅助函数、常量、调度与日志工具
//! - `message`：消息枚举定义
//! - `update`：消息分发与状态更新实现

use crate::app::components::text_editor_context_menu::{
    focus_editor_task, paste_action, paste_task, selection_copy_task, selection_cut_task,
    selection_delete_task,
};
use crate::app::task::{
    Task, TaskStatus,
};

mod advanced_filter;
mod helpers;
mod message;
mod update;
mod work_view_prefs;

pub(crate) use helpers::{
    next_agent_scheduler_status_generation, resolve_task_board_scheduler_scope, sync_task_logs_editor,
};
pub use message::TaskBoardMessage;
pub(crate) use message::WorktreePoolMaintenanceOutcome;
pub(crate) use update::{activate_tab, close_tab};
pub(in crate::app) use work_view_prefs::filter as work_filter_draft;
pub use update::update;

pub(crate) const TAB_ID: &str = "task_board";

pub(in crate::app) fn attachment_link(input: &str) -> Option<String> {
    let value = input.trim();
    if value.len() > 2048 || value.chars().any(char::is_control) {
        return None;
    }
    let url = reqwest::Url::parse(value).ok()?;
    (matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none())
    .then(|| url.to_string())
}

#[cfg(test)]
#[path = "task_board_tests.rs"]
mod task_board_tests;

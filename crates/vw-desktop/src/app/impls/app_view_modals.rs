//! 渲染应用视图中的模态窗口。
//! 本模块只描述模态内容和交互消息，不直接承担持久化策略。

use super::message;
use super::{App, Message};

mod account_details;
mod chat_fork;
mod chat_reset;
mod git_diff;
mod permission_modal;
mod project_edit;
mod question_modal;
mod recent_project_remove;
mod rename;

#[allow(dead_code)]
pub(super) const PRESET_COLORS: [(&str, &str); 10] = [
    ("Blue", "#3b82f6"),
    ("Purple", "#8b5cf6"),
    ("Green", "#22c55e"),
    ("Red", "#ef4444"),
    ("Orange", "#f97316"),
    ("Pink", "#ec4899"),
    ("Cyan", "#06b6d4"),
    ("Yellow", "#eab308"),
    ("Teal", "#14b8a6"),
    ("Indigo", "#6366f1"),
];

pub(crate) use account_details::with_account_details;
pub(crate) use chat_fork::with_chat_fork_dialog;
pub(crate) use chat_reset::with_chat_reset_dialog;
pub(crate) use git_diff::with_git_diff_overlays;
pub(crate) use permission_modal::with_permission_modal;
pub(crate) use project_edit::with_project_edit;
pub(crate) use question_modal::with_question_modal;
pub(crate) use recent_project_remove::with_recent_project_remove_confirm;
pub(crate) use rename::{with_file_tree_rename, with_session_rename};
#[cfg(test)]
#[path = "app_view_modals_tests.rs"]
mod app_view_modals_tests;

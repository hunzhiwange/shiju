//! Git 消息处理模块
//!
//! 本模块负责处理所有与 Git 相关的 UI 消息和操作，包括：
//! - 变更文件的刷新与展示
//! - Diff 视图的交互（选择、高亮、滚动、评论）
//! - 文件暂存与提交
//! - 代码丢弃与恢复
//! - 自定义 Diff 对比
//! - 复制模式与聊天集成
//!
//! 该模块是 VibeWindow 应用中 Git 面板功能的核心消息处理器。

use crate::app::{App, Message};
use iced::Task;

mod effects;
mod repository;
mod shared;
pub(crate) use shared::git_context_path_for_app;
mod stage_commit;

pub use desktop_git_ui::message_git::messages::{ExpandDirection, GitMessage};

pub fn update(app: &mut App, message: GitMessage) -> Task<Message> {
    match message {
        message @ (GitMessage::RemoteAnimationTick
        | GitMessage::ShowHistory(_)
        | GitMessage::RefreshRepository
        | GitMessage::LoadMoreHistory
        | GitMessage::RepositoryReady { .. }
        | GitMessage::SelectHistoryCommit(_)
        | GitMessage::HistoryDetailReady { .. }
        | GitMessage::OpenHistoryFileMenu(_)
        | GitMessage::CloseHistoryFileMenu
        | GitMessage::PreviewHistoryFile(_)
        | GitMessage::CopyHistoryFile(_)
        | GitMessage::SelectHistoryFile(_)
        | GitMessage::RunRemote(_)
        | GitMessage::RemoteFinished { .. }) => repository::update(app, message),
        message => effects::update(app, message, desktop_git_ui::message_git::controller::update),
    }
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod diff_tests;

#[cfg(test)]
#[path = "modal_tests.rs"]
mod modal_tests;

#[cfg(test)]
#[path = "refresh_tests.rs"]
mod refresh_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod repository_tests;

#[cfg(test)]
mod diff;

#[cfg(test)]
mod modal;

#[cfg(test)]
mod refresh;

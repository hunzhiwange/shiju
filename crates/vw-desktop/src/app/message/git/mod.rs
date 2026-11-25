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

#[derive(Clone, Copy)]
pub(crate) enum Source {
    Inline,
    #[cfg(not(target_arch = "wasm32"))]
    Workbench(u64),
}

impl Source {
    fn is_inline(self) -> bool {
        matches!(self, Self::Inline)
    }

    pub(crate) fn message(self, message: GitMessage) -> Message {
        match self {
            Self::Inline => Message::Git(message),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Workbench(epoch) => Message::GitWorkbench(
                crate::app::git_workbench::Event::Git(epoch, Box::new(message)),
            ),
        }
    }

    fn state_mut(self, app: &mut App) -> &mut desktop_git_ui::state::State {
        match self {
            Self::Inline => &mut app.git,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Workbench(_) => &mut app.git_workbench.git,
        }
    }

    fn project_path(self, app: &App) -> Option<String> {
        match self {
            Self::Inline => app.navigation_tabs.project_path.clone(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Workbench(_) => app.git_workbench.project_directory().map(str::to_owned),
        }
    }

    fn repo_path(self, app: &App) -> Option<String> {
        match self {
            Self::Inline => crate::app::components::git_panel::git_repo_path_for_app(app),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Workbench(_) => app.git_workbench.repo_path(),
        }
    }

    fn context_path(self, app: &App) -> Option<String> {
        match self {
            Self::Inline => shared::git_context_path_for_app(app),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Workbench(_) => self.repo_path(app),
        }
    }

    fn client(self, app: &App) -> Result<vw_gateway_client::GatewayClient, String> {
        match self {
            Self::Inline => crate::app::gateway_client(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Workbench(_) => app.git_workbench.client(),
        }
    }

    fn after_mutation(self, app: &mut App) -> Task<Message> {
        desktop_git_ui::message_git::controller::after_mutation(self.state_mut(app))
            .map(move |message| self.message(message))
    }
}

pub fn update(app: &mut App, message: GitMessage) -> Task<Message> {
    update_from(app, Source::Inline, message)
}

pub(crate) fn update_from(app: &mut App, source: Source, message: GitMessage) -> Task<Message> {
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
        | GitMessage::RemoteFinished { .. }) => repository::update(app, source, message),
        message => effects::update(app, source, message, desktop_git_ui::message_git::controller::update),
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

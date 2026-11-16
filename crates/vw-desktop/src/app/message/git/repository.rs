//! Git 历史与同步的宿主消息及跨域效果映射。
use super::{GitMessage, shared};
use crate::app::{App, Message};
use desktop_git_ui::message_git::repository::{self, Effect, RepositoryMessage as Local};
use iced::Task;

pub(super) fn update(app: &mut App, message: GitMessage) -> Task<Message> {
    let message = match message {
        GitMessage::RemoteAnimationTick => Local::RemoteAnimationTick,
        GitMessage::ShowHistory(value) => Local::ShowHistory(value),
        GitMessage::RefreshRepository => Local::RefreshRepository,
        GitMessage::LoadMoreHistory => Local::LoadMoreHistory,
        GitMessage::RepositoryReady { path, generation, result } => {
            Local::RepositoryReady { path, generation, result }
        }
        GitMessage::SelectHistoryCommit(value) => Local::SelectHistoryCommit(value),
        GitMessage::HistoryDetailReady { path, generation, sha, result } => {
            Local::HistoryDetailReady { path, generation, sha, result }
        }
        GitMessage::SelectHistoryFile(value) => Local::SelectHistoryFile(value),
        GitMessage::OpenHistoryFileMenu(value) => Local::OpenHistoryFileMenu(value),
        GitMessage::CloseHistoryFileMenu => Local::CloseHistoryFileMenu,
        GitMessage::PreviewHistoryFile(value) => Local::PreviewHistoryFile(value),
        GitMessage::CopyHistoryFile(value) => Local::CopyHistoryFile(value),
        GitMessage::RunRemote(value) => Local::RunRemote(value),
        GitMessage::RemoteFinished { path, action, result } => {
            Local::RemoteFinished { path, action, result }
        }
        _ => unreachable!("unexpected repository message"),
    };
    let path = crate::app::components::git_panel::git_repo_path_for_app(app);
    let gateway = crate::app::gateway_client()
        .map(|client| (client, crate::app::session_gateway::active_tenant_scope()));
    let mut effects = Vec::new();
    let task = repository::update(
        &mut app.git.git_repository,
        &mut app.git.selected_branch,
        path,
        app.git.git_commit_in_progress,
        gateway,
        &mut effects,
        message,
    )
    .map(|message| {
        Message::Git(match message {
            Local::RemoteAnimationTick => GitMessage::RemoteAnimationTick,
            Local::ShowHistory(value) => GitMessage::ShowHistory(value),
            Local::RefreshRepository => GitMessage::RefreshRepository,
            Local::LoadMoreHistory => GitMessage::LoadMoreHistory,
            Local::RepositoryReady { path, generation, result } => {
                GitMessage::RepositoryReady { path, generation, result }
            }
            Local::SelectHistoryCommit(value) => GitMessage::SelectHistoryCommit(value),
            Local::HistoryDetailReady { path, generation, sha, result } => {
                GitMessage::HistoryDetailReady { path, generation, sha, result }
            }
            Local::SelectHistoryFile(value) => GitMessage::SelectHistoryFile(value),
            Local::OpenHistoryFileMenu(value) => GitMessage::OpenHistoryFileMenu(value),
            Local::CloseHistoryFileMenu => GitMessage::CloseHistoryFileMenu,
            Local::PreviewHistoryFile(value) => GitMessage::PreviewHistoryFile(value),
            Local::CopyHistoryFile(value) => GitMessage::CopyHistoryFile(value),
            Local::RunRemote(value) => GitMessage::RunRemote(value),
            Local::RemoteFinished { path, action, result } => {
                GitMessage::RemoteFinished { path, action, result }
            }
        })
    });
    let mut tasks = vec![task];
    for effect in effects {
        tasks.push(match effect {
            Effect::CopyCode(content) => Task::done(Message::CopyCode(content)),
            Effect::PreviewSnapshot { key, path, title, content } => {
                Task::done(Message::Preview(crate::app::message::PreviewMessage::OpenSnapshot {
                    key,
                    path,
                    title,
                    content,
                }))
            }
            Effect::Toast { success, message } => app.show_toast(
                if success {
                    crate::app::state::ToastKind::Success
                } else {
                    crate::app::state::ToastKind::Error
                },
                message,
            ),
            Effect::RefreshAfterMutation => {
                shared::refresh_git_panel_data_after_repo_mutation_task(app)
            }
            Effect::ScheduleAnimationTick => crate::app::message::after(
                std::time::Duration::from_millis(
                    crate::app::components::status_animation::ACTIVITY_ANIMATION_INTERVAL_MS,
                ),
                Message::Git(GitMessage::RemoteAnimationTick),
            ),
        });
    }
    Task::batch(tasks)
}

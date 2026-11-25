//! Git 历史与同步的宿主消息及跨域效果映射。
use super::{GitMessage, Source};
use crate::app::{App, Message};
use desktop_git_ui::message_git::repository::{self, Effect, RepositoryMessage as Local};
use iced::Task;

pub(super) fn update(app: &mut App, source: Source, message: GitMessage) -> Task<Message> {
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
    let path = source.repo_path(app);
    let gateway = source.client(app).map(|client| {
        let scope = client.endpoint().tenant_scope().clone();
        (client, scope)
    }).map_err(Into::into);
    let mut effects = Vec::new();
    let git = source.state_mut(app);
    let task = repository::update(
        &mut git.git_repository,
        &mut git.selected_branch,
        path,
        git.git_commit_in_progress,
        gateway,
        &mut effects,
        message,
    )
    .map(move |message| {
        source.message(match message {
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
        let task = match effect {
            Effect::CopyCode(content) => Task::done(Message::CopyCode(content)),
            Effect::PreviewSnapshot { key, path, title, content } => {
                #[cfg(not(target_arch = "wasm32"))]
                if !source.is_inline() {
                    tasks.push(crate::app::git_workbench::preview_snapshot(app, title, content));
                    continue;
                }
                Task::done(Message::Preview(crate::app::message::PreviewMessage::OpenSnapshot {
                    key,
                    path,
                    title,
                    content,
                }))
            }
            Effect::Toast { success, message } => app.show_message_toast(
                if success {
                    crate::app::state::ToastKind::Success
                } else {
                    crate::app::state::ToastKind::Error
                },
                message.message(),
            ),
            Effect::RefreshAfterMutation => {
                source.after_mutation(app)
            }
            Effect::ScheduleAnimationTick => crate::app::message::after(
                std::time::Duration::from_millis(
                    crate::app::components::status_animation::ACTIVITY_ANIMATION_INTERVAL_MS,
                ),
                source.message(GitMessage::RemoteAnimationTick),
            ),
        };
        tasks.push(task);
    }
    Task::batch(tasks)
}

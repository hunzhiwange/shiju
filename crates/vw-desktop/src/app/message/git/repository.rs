//! Git 历史与同步消息。异步结果必须匹配仓库、请求代次和选中提交。
use super::{GitMessage, shared};
use crate::app::components::git_panel::repository_ops;
use crate::app::{App, Message};
use iced::Task;

async fn gateway_context(
    path: &str,
) -> Result<crate::app::message::project::helpers::GatewayFileContext, String> {
    let context = crate::app::message::project::helpers::resolve_gateway_file_context(path).await?;
    if context.worktree_id.is_none() {
        let project = crate::app::message::project::helpers::resolve_gateway_project(path).await?;
        let normalize = |value: &str| value.replace('\\', "/").trim_end_matches('/').to_owned();
        if normalize(path) != normalize(&project.directory) {
            return Err("所选目录不是项目主工作区或已注册的 worktree".into());
        }
    }
    Ok(context)
}

fn current_path(app: &App) -> Option<String> {
    crate::app::components::git_panel::git_repo_path_for_app(app)
}

pub(super) fn update(app: &mut App, message: GitMessage) -> Task<Message> {
    match message {
        GitMessage::RemoteAnimationTick => {
            if !app.git_repository.remote_busy {
                return Task::none();
            }
            app.git_repository.remote_animation_frame =
                app.git_repository.remote_animation_frame.wrapping_add(1);
            crate::app::message::after(
                std::time::Duration::from_millis(
                    crate::app::components::status_animation::ACTIVITY_ANIMATION_INTERVAL_MS,
                ),
                Message::Git(GitMessage::RemoteAnimationTick),
            )
        }
        GitMessage::ShowHistory(history) => {
            app.git_repository.history = history && current_path(app).is_some();
            Task::done(Message::Git(GitMessage::RefreshRepository))
        }
        GitMessage::LoadMoreHistory => {
            if app.git_repository.loading {
                return Task::none();
            }
            app.git_repository.limit = (app.git_repository.limit + 100).min(10_000);
            Task::done(Message::Git(GitMessage::RefreshRepository))
        }
        GitMessage::RefreshRepository => {
            let path = current_path(app);
            let state = &mut app.git_repository;
            state.generation = state.generation.wrapping_add(1);
            if state.repo_path != path {
                let history = state.history;
                let remote_busy = state.remote_busy;
                let remote_action = state.remote_action;
                let generation = state.generation;
                *state = repository_ops::RepositoryState {
                    history,
                    remote_busy,
                    remote_action,
                    generation,
                    ..Default::default()
                };
            }
            state.repo_path = path.clone();
            let Some(path) = path else {
                state.history = false;
                state.loading = false;
                state.detail_loading = false;
                state.error = None;
                return Task::none();
            };
            state.limit = state.limit.max(100);
            state.loading = true;
            state.detail_loading = false;
            state.error = None;
            let (generation, limit) = (state.generation, state.limit);
            Task::perform(
                async move {
                    let result = async {
                        let context = gateway_context(&path).await?;
                        crate::app::gateway_client()?
                            .git_repository(
                                &vw_gateway_client::vw_api_types::git::GitRepositoryRequest {
                                    project_id: context.project_id,
                                    worktree_id: context.worktree_id,
                                    limit,
                                },
                            )
                            .await
                    }
                    .await;
                    (path, generation, result)
                },
                |(path, generation, result)| {
                    Message::Git(GitMessage::RepositoryReady { path, generation, result })
                },
            )
        }
        GitMessage::RepositoryReady { path, generation, result } => {
            if current_path(app).as_deref() != Some(&path)
                || app.git_repository.generation != generation
            {
                return Task::none();
            }
            let state = &mut app.git_repository;
            state.loading = false;
            match result {
                Ok(snapshot) => {
                    app.selected_branch = snapshot.branch.clone();
                    let selected = state
                        .selected
                        .clone()
                        .filter(|sha| snapshot.commits.iter().any(|commit| &commit.sha == sha))
                        .or_else(|| snapshot.commits.first().map(|commit| commit.sha.clone()));
                    state.snapshot = snapshot;
                    if state.history
                        && let Some(sha) = selected
                    {
                        return Task::done(Message::Git(GitMessage::SelectHistoryCommit(sha)));
                    }
                    state.selected = None;
                    state.detail = None;
                }
                Err(error) => state.error = Some(error),
            }
            Task::none()
        }
        GitMessage::SelectHistoryCommit(sha) => {
            let Some(path) = current_path(app) else {
                return Task::none();
            };
            let state = &mut app.git_repository;
            if state.repo_path.as_deref() != Some(&path)
                || !state.snapshot.commits.iter().any(|commit| commit.sha == sha)
            {
                return Task::none();
            }
            state.selected = Some(sha.clone());
            state.detail = None;
            state.selected_file = None;
            state.file_menu = None;
            state.detail_loading = true;
            let generation = state.generation;
            Task::perform(
                async move {
                    let result = async {
                        let context = gateway_context(&path).await?;
                        crate::app::gateway_client()?
                            .git_history_detail(
                                &vw_gateway_client::vw_api_types::git::GitHistoryDetailRequest {
                                    project_id: context.project_id,
                                    worktree_id: context.worktree_id,
                                    sha: sha.clone(),
                                },
                            )
                            .await
                    }
                    .await;
                    (path, generation, sha, result)
                },
                |(path, generation, sha, result)| {
                    Message::Git(GitMessage::HistoryDetailReady { path, generation, sha, result })
                },
            )
        }
        GitMessage::HistoryDetailReady { path, generation, sha, result } => {
            if current_path(app).as_deref() != Some(&path)
                || app.git_repository.generation != generation
                || app.git_repository.selected.as_deref() != Some(&sha)
            {
                return Task::none();
            }
            let state = &mut app.git_repository;
            state.detail_loading = false;
            match result {
                Ok(detail) => state.detail = Some(detail),
                Err(error) => state.error = Some(error),
            }
            Task::none()
        }
        GitMessage::OpenHistoryFileMenu(index) => {
            app.git_repository.file_menu = Some(index);
            Task::none()
        }
        GitMessage::CloseHistoryFileMenu => {
            app.git_repository.file_menu = None;
            Task::none()
        }
        action @ (GitMessage::PreviewHistoryFile(_) | GitMessage::CopyHistoryFile(_)) => {
            let (GitMessage::PreviewHistoryFile(index) | GitMessage::CopyHistoryFile(index)) =
                &action
            else {
                unreachable!()
            };
            let snapshot = app
                .git_repository
                .detail
                .as_ref()
                .and_then(|detail| detail.files.get(*index))
                .and_then(|file| {
                    if matches!(
                        file.status,
                        vw_gateway_client::vw_api_types::git::GitFileStatus::Deleted
                    ) {
                        file.old_content.clone()
                    } else {
                        file.new_content.clone()
                    }
                    .map(|content| (file.path.clone(), content))
                });
            app.git_repository.file_menu = None;
            let Some((path, content)) = snapshot else {
                return Task::none();
            };
            if matches!(action, GitMessage::CopyHistoryFile(_)) {
                Task::done(Message::CopyCode(content))
            } else {
                let sha = app.git_repository.selected.as_deref().unwrap_or_default();
                let title = format!(
                    "{} @ {}",
                    std::path::Path::new(&path).file_name().unwrap_or_default().to_string_lossy(),
                    sha.chars().take(8).collect::<String>()
                );
                let key = format!(
                    "git-history://{}/{sha}/{path}",
                    app.git_repository.repo_path.as_deref().unwrap_or_default()
                );
                Task::done(Message::Preview(crate::app::message::PreviewMessage::OpenSnapshot {
                    key,
                    path,
                    title,
                    content,
                }))
            }
        }
        GitMessage::SelectHistoryFile(index) => {
            app.git_repository.file_menu = None;
            app.git_repository.selected_file =
                if app.git_repository.selected_file == Some(index) { None } else { Some(index) };
            Task::none()
        }
        GitMessage::RunRemote(action) => {
            if app.git_repository.remote_busy || app.git_commit_in_progress {
                return Task::none();
            }
            let Some(path) = current_path(app) else {
                return Task::none();
            };
            app.git_repository.remote_busy = true;
            app.git_repository.remote_action = Some(action);
            app.git_repository.remote_animation_frame = 0;
            // 执行前重新读取分支，避免使用刷新前的远端或跟踪配置。
            Task::batch([
                Task::done(Message::Git(GitMessage::RemoteAnimationTick)),
                Task::perform(
                    async move {
                        let result = async {
                            let context = gateway_context(&path).await?;
                            let response = crate::app::gateway_client()?
                                .git_remote(
                                    &vw_gateway_client::vw_api_types::git::GitRemoteRequest {
                                        project_id: context.project_id,
                                        worktree_id: context.worktree_id,
                                        action,
                                    },
                                )
                                .await?;
                            if response.ok {
                                Ok(())
                            } else {
                                Err("网关未完成 Git 同步".to_owned())
                            }
                        }
                        .await;
                        (path, action, result)
                    },
                    |(path, action, result)| {
                        Message::Git(GitMessage::RemoteFinished { path, action, result })
                    },
                ),
            ])
        }
        GitMessage::RemoteFinished { path, action, result } => {
            app.git_repository.remote_busy = false;
            app.git_repository.remote_action = None;
            let toast = match result {
                Ok(()) => app.show_toast(
                    crate::app::state::ToastKind::Success,
                    format!("{}完成", repository_ops::remote_action_label(action)),
                ),
                Err(error) => app.show_toast(crate::app::state::ToastKind::Error, error),
            };
            if current_path(app).as_deref() == Some(&path) {
                Task::batch([toast, shared::refresh_git_panel_data_after_repo_mutation_task(app)])
            } else {
                toast
            }
        }
        _ => unreachable!("unexpected repository message"),
    }
}

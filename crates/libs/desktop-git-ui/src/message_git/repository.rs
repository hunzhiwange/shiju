//! 仓库历史和远端同步的局部更新；宿主只映射跨域效果。
use crate::git_panel::repository_ops;
use iced::Task;
#[derive(Debug, Clone)]
pub enum RepositoryMessage {
    RemoteAnimationTick,
    ShowHistory(bool),
    RefreshRepository,
    LoadMoreHistory,
    RepositoryReady {
        path: String,
        generation: u64,
        result:
            Result<repository_ops::RepositorySnapshot, String>,
    },
    SelectHistoryCommit(String),
    HistoryDetailReady {
        path: String,
        generation: u64,
        sha: String,
        result: Result<repository_ops::HistoryDetail, String>,
    },
    SelectHistoryFile(usize),
    OpenHistoryFileMenu(usize),
    CloseHistoryFileMenu,
    PreviewHistoryFile(usize),
    CopyHistoryFile(usize),
    RunRemote(repository_ops::RemoteAction),
    RemoteFinished {
        path: String,
        action: repository_ops::RemoteAction,
        result: Result<(), String>,
    },
}
pub enum Effect {
    CopyCode(String),
    PreviewSnapshot { key: String, path: String, title: String, content: String },
    Toast { success: bool, message: String },
    RefreshAfterMutation,
    ScheduleAnimationTick,
}
async fn gateway_context(
    client: &vw_gateway_client::GatewayClient,
    tenant_scope: &vw_api_project::TenantScopeDto,
    path: &str,
) -> Result<desktop_workspace::project_message::gateway::GatewayFileContext, String> {
    let context = desktop_workspace::project_message::gateway::resolve_gateway_file_context(client, tenant_scope, path).await?;
    if context.worktree_id.is_none() {
        let project = desktop_workspace::project_message::gateway::resolve_gateway_project(client, tenant_scope, path).await?;
        let normalize = |value: &str| value.replace('\\', "/").trim_end_matches('/').to_owned();
        if normalize(path) != normalize(&project.directory) {
            return Err("所选目录不是项目主工作区或已注册的 worktree".into());
        }
    }
    Ok(context)
}

pub fn update(
    state: &mut repository_ops::RepositoryState,
    selected_branch: &mut Option<String>,
    current_path: Option<String>,
    commit_in_progress: bool,
    gateway: Result<(vw_gateway_client::GatewayClient, vw_api_project::TenantScopeDto), String>,
    effects: &mut Vec<Effect>,
    message: RepositoryMessage,
) -> Task<RepositoryMessage> {
    match message {
        RepositoryMessage::RemoteAnimationTick => {
            if !(*state).remote_busy {
                return Task::none();
            }
            (*state).remote_animation_frame =
                (*state).remote_animation_frame.wrapping_add(1);
            effects.push(Effect::ScheduleAnimationTick);
            Task::none()
        }
        RepositoryMessage::ShowHistory(history) => {
            (*state).history = history && current_path.clone().is_some();
            Task::done(RepositoryMessage::RefreshRepository)
        }
        RepositoryMessage::LoadMoreHistory => {
            if (*state).loading {
                return Task::none();
            }
            (*state).limit = ((*state).limit + 100).min(10_000);
            Task::done(RepositoryMessage::RefreshRepository)
        }
        RepositoryMessage::RefreshRepository => {
            let path = current_path.clone();
            let state = &mut (*state);
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
                        let (client, tenant_scope) = gateway?;
                        let context = gateway_context(&client, &tenant_scope, &path).await?;
                        gateway_project::GatewayProjectClient::new(client.transport().clone())
                            .git_repository(
                                &vw_api_git::GitRepositoryRequest {
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
                    RepositoryMessage::RepositoryReady { path, generation, result }
                },
            )
        }
        RepositoryMessage::RepositoryReady { path, generation, result } => {
            if current_path.clone().as_deref() != Some(&path)
                || (*state).generation != generation
            {
                return Task::none();
            }
            let state = &mut (*state);
            state.loading = false;
            match result {
                Ok(snapshot) => {
                    (*selected_branch) = snapshot.branch.clone();
                    let selected = state
                        .selected
                        .clone()
                        .filter(|sha| snapshot.commits.iter().any(|commit| &commit.sha == sha))
                        .or_else(|| snapshot.commits.first().map(|commit| commit.sha.clone()));
                    state.snapshot = snapshot;
                    if state.history
                        && let Some(sha) = selected
                    {
                        return Task::done(RepositoryMessage::SelectHistoryCommit(sha));
                    }
                    state.selected = None;
                    state.detail = None;
                }
                Err(error) => state.error = Some(error),
            }
            Task::none()
        }
        RepositoryMessage::SelectHistoryCommit(sha) => {
            let Some(path) = current_path.clone() else {
                return Task::none();
            };
            let state = &mut (*state);
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
                        let (client, tenant_scope) = gateway?;
                        let context = gateway_context(&client, &tenant_scope, &path).await?;
                        gateway_project::GatewayProjectClient::new(client.transport().clone())
                            .git_history_detail(
                                &vw_api_git::GitHistoryDetailRequest {
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
                    RepositoryMessage::HistoryDetailReady { path, generation, sha, result }
                },
            )
        }
        RepositoryMessage::HistoryDetailReady { path, generation, sha, result } => {
            if current_path.clone().as_deref() != Some(&path)
                || (*state).generation != generation
                || (*state).selected.as_deref() != Some(&sha)
            {
                return Task::none();
            }
            let state = &mut (*state);
            state.detail_loading = false;
            match result {
                Ok(detail) => state.detail = Some(detail),
                Err(error) => state.error = Some(error),
            }
            Task::none()
        }
        RepositoryMessage::OpenHistoryFileMenu(index) => {
            (*state).file_menu = Some(index);
            Task::none()
        }
        RepositoryMessage::CloseHistoryFileMenu => {
            (*state).file_menu = None;
            Task::none()
        }
        action @ (RepositoryMessage::PreviewHistoryFile(_) | RepositoryMessage::CopyHistoryFile(_)) => {
            let (RepositoryMessage::PreviewHistoryFile(index) | RepositoryMessage::CopyHistoryFile(index)) =
                &action
            else {
                unreachable!()
            };
            let snapshot = (*state)
                .detail
                .as_ref()
                .and_then(|detail| detail.files.get(*index))
                .and_then(|file| {
                    if matches!(
                        file.status,
                        vw_api_git::GitFileStatus::Deleted
                    ) {
                        file.old_content.clone()
                    } else {
                        file.new_content.clone()
                    }
                    .map(|content| (file.path.clone(), content))
                });
            (*state).file_menu = None;
            let Some((path, content)) = snapshot else {
                return Task::none();
            };
            if matches!(action, RepositoryMessage::CopyHistoryFile(_)) {
                { effects.push(Effect::CopyCode(content)); Task::none() }
            } else {
                let sha = (*state).selected.as_deref().unwrap_or_default();
                let title = format!(
                    "{} @ {}",
                    std::path::Path::new(&path).file_name().unwrap_or_default().to_string_lossy(),
                    sha.chars().take(8).collect::<String>()
                );
                let key = format!(
                    "git-history://{}/{sha}/{path}",
                    (*state).repo_path.as_deref().unwrap_or_default()
                );
                effects.push(Effect::PreviewSnapshot {
                    key,
                    path,
                    title,
                    content,
                });
                Task::none()
            }
        }
        RepositoryMessage::SelectHistoryFile(index) => {
            (*state).file_menu = None;
            (*state).selected_file =
                if (*state).selected_file == Some(index) { None } else { Some(index) };
            Task::none()
        }
        RepositoryMessage::RunRemote(action) => {
            if (*state).remote_busy || commit_in_progress {
                return Task::none();
            }
            let Some(path) = current_path.clone() else {
                return Task::none();
            };
            (*state).remote_busy = true;
            (*state).remote_action = Some(action);
            (*state).remote_animation_frame = 0;
            // 执行前重新读取分支，避免使用刷新前的远端或跟踪配置。
            Task::batch([
                Task::done(RepositoryMessage::RemoteAnimationTick),
                Task::perform(
                    async move {
                        let result = async {
                            let (client, tenant_scope) = gateway?;
                        let context = gateway_context(&client, &tenant_scope, &path).await?;
                            let response = gateway_project::GatewayProjectClient::new(client.transport().clone())
                                .git_remote(
                                    &vw_api_git::GitRemoteRequest {
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
                        RepositoryMessage::RemoteFinished { path, action, result }
                    },
                ),
            ])
        }
        RepositoryMessage::RemoteFinished { path, action, result } => {
            (*state).remote_busy = false;
            (*state).remote_action = None;
            let (success, message) = match result {
                Ok(()) => (true, format!("{}完成", repository_ops::remote_action_label(action))),
                Err(error) => (false, error),
            };
            effects.push(Effect::Toast { success, message });
            if current_path.as_deref() == Some(&path) {
                effects.push(Effect::RefreshAfterMutation);
            }
            Task::none()
        }
    }
}

#[cfg(test)]
#[path = "repository_tests.rs"]
mod tests;

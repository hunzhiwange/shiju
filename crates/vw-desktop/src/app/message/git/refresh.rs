//! 刷新 Git 仓库状态，并把分支、变更和错误信息写回应用状态。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use super::GitMessage;
use super::shared::{
    load_git_worktree_options, load_missing_diff_contents, persist_file_tree_expanded,
    refresh_git_panel_data_task,
};
use crate::app::{App, Message};
use iced::Task;

fn same_directory(left: &str, right: &str) -> bool {
    left.replace('\\', "/").trim_end_matches('/') == right.replace('\\', "/").trim_end_matches('/')
}

/// update 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(super) fn update(app: &mut App, message: GitMessage) -> Task<Message> {
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
        GitMessage::RefreshGitPanelData => refresh_git_panel_data_task(),
        GitMessage::RefreshWorktreeOptions => {
            let Some(project_path) = app.project_path.clone() else {
                app.git_worktree_options.clear();
                app.selected_git_worktree_directory = None;
                app.git_worktree_options_loading = false;
                app.git_worktree_options_project_path = None;
                app.git_worktree_menu_open = false;
                return Task::none();
            };

            #[cfg(not(target_arch = "wasm32"))]
            if git2::Repository::discover(&project_path).is_err() {
                app.git_worktree_options.clear();
                app.selected_git_worktree_directory = None;
                app.git_worktree_options_loading = false;
                app.git_worktree_options_project_path = None;
                app.git_worktree_menu_open = false;
                return Task::none();
            }

            if app.git_worktree_options_loading
                && app.git_worktree_options_project_path.as_deref() == Some(project_path.as_str())
            {
                return Task::none();
            }

            app.git_worktree_options_loading = true;
            app.git_worktree_options_project_path = Some(project_path.clone());
            Task::perform(
                async move {
                    let result = load_git_worktree_options(project_path.clone()).await;
                    (project_path, result)
                },
                |(project_path, result)| {
                    Message::Git(GitMessage::WorktreeOptionsReady { project_path, result })
                },
            )
        }
        GitMessage::WorktreeOptionsReady { project_path, result } => {
            if app.git_worktree_options_project_path.as_deref() != Some(project_path.as_str()) {
                return Task::none();
            }
            app.git_worktree_options_loading = false;
            match result {
                Ok(options) => {
                    let current_directory = app
                        .selected_git_worktree_directory
                        .clone()
                        .or_else(|| crate::app::components::git_panel::git_repo_path_for_app(app))
                        .or_else(|| app.project_path.clone());
                    app.git_worktree_options = options;
                    app.selected_git_worktree_directory = current_directory
                        .as_ref()
                        .and_then(|directory| {
                            app.git_worktree_options
                                .iter()
                                .find(|option| same_directory(&option.directory, directory))
                        })
                        .or_else(|| app.git_worktree_options.first())
                        .map(|option| option.directory.clone());
                    if let Some(selected) =
                        app.selected_git_worktree_directory.as_ref().and_then(|directory| {
                            app.git_worktree_options
                                .iter()
                                .find(|option| option.directory.as_str() == directory.as_str())
                        })
                    {
                        app.selected_branch = selected.branch.clone();
                    }
                }
                Err(error) => {
                    app.git_worktree_options.clear();
                    app.selected_git_worktree_directory = None;
                    app.git_worktree_menu_open = false;
                    app.error_message = Some(format!("读取 Git worktree 失败: {}", error));
                }
            }
            Task::none()
        }
        GitMessage::SelectGitWorktree(option) => {
            app.git_worktree_menu_open = false;
            app.selected_git_worktree_directory = Some(option.directory.clone());
            app.selected_branch = option.branch.clone();
            app.git_changed_files.clear();
            app.git_changed_files_loading = false;
            app.git_changed_files_repo_path = None;
            app.git_diff_file_metas.clear();
            app.git_diff_file_metas_loading = false;
            app.git_diff_file_metas_repo_path = None;
            app.git_diff_contents.clear();
            app.git_diff_contents_loading.clear();
            app.git_diff_selected_lines.clear();
            app.git_diff_selected_range = None;
            app.git_diff_comment_draft = None;
            refresh_git_panel_data_task()
        }
        GitMessage::ToggleGitWorktreeMenu(open) => {
            app.git_worktree_menu_open = open;
            Task::none()
        }
        GitMessage::RefreshChangedFiles => {
            app.git_diff_context_menu = None;
            if app.git_changed_files_loading {
                return Task::none();
            }
            let Some(path) = crate::app::components::git_panel::git_repo_path_for_app(app) else {
                app.git_changed_files.clear();
                app.git_changed_files_loading = false;
                app.git_changed_files_repo_path = None;
                return Task::none();
            };

            app.git_changed_files_loading = true;
            app.git_changed_files_repo_path = Some(path.clone());
            let result_repo_path = path.clone();
            // 耗时或平台相关操作交给异步任务，避免阻塞界面消息循环。
            Task::perform(
                async move {
                    let repo_path = path.clone();
                    crate::app::message::spawn_blocking_opt(move || {
                        Some(crate::app::components::git_panel::changed_files_in_repo(&repo_path))
                    })
                    .await
                    .unwrap_or_default()
                },
                move |files| {
                    Message::Git(GitMessage::ChangedFilesReady {
                        repo_path: Some(result_repo_path.clone()),
                        files,
                    })
                },
            )
        }
        GitMessage::RefreshDiffFileMetas => {
            app.git_diff_context_menu = None;
            let repo_path = crate::app::components::git_panel::git_repo_path_for_app(app);
            let Some(path) = repo_path.clone() else {
                app.git_diff_file_metas.clear();
                app.git_diff_file_metas_loading = false;
                app.git_diff_file_metas_repo_path = None;
                app.git_diff_contents.clear();
                app.git_diff_contents_loading.clear();
                return Task::none();
            };

            if app.git_diff_file_metas_loading
                && app.git_diff_file_metas_repo_path.as_deref() == Some(path.as_str())
            {
                return Task::none();
            }

            app.git_diff_file_metas_loading = true;
            app.git_diff_file_metas_repo_path = Some(path.clone());

            Task::perform(
                async move {
                    let repo_path = path.clone();
                    let metas = crate::app::message::spawn_blocking_opt(move || {
                        Some(crate::app::components::git_panel::get_diff_file_metas_for_repo_path(
                            &repo_path,
                        ))
                    })
                    .await
                    .unwrap_or_default();
                    (Some(path), metas)
                },
                |(repo_path, metas)| {
                    Message::Git(GitMessage::DiffFileMetasReady { repo_path, metas })
                },
            )
        }
        GitMessage::ChangedFilesReady { repo_path, files } => {
            if app.git_changed_files_repo_path != repo_path {
                return Task::none();
            }
            app.git_changed_files = files;
            app.git_changed_files_loading = false;
            let was_manual_refresh = app.file_manager_changes_refreshing;
            app.file_manager_changes_refreshing = false;
            app.git_diff_selected_lines.clear();
            app.git_diff_selected_range = None;
            app.git_diff_comment_draft = None;

            if app.file_manager_show_changes {
                for key in crate::app::components::file_tree::change_list_expand_keys(
                    app.git_changed_files.iter().map(String::as_str),
                ) {
                    app.ensure_file_tree_dir_expanded(key);
                }
                persist_file_tree_expanded(app);
            }

            if was_manual_refresh {
                app.show_success_toast("Git 更改已刷新")
            } else {
                Task::none()
            }
        }
        GitMessage::DiffFileMetasReady { repo_path, metas } => {
            if app.git_diff_file_metas_repo_path != repo_path {
                return Task::none();
            }
            app.git_diff_file_metas = metas;
            app.git_diff_file_metas_loading = false;
            app.git_diff_contents.clear();
            app.git_diff_contents_loading.clear();
            load_missing_diff_contents(app, app.expanded_files.clone())
        }
        GitMessage::LoadDiffContent(file) => load_missing_diff_contents(app, std::iter::once(file)),
        GitMessage::DiffContentReady { repo_path, file, old_content, new_content } => {
            if app.git_diff_file_metas_repo_path != repo_path {
                return Task::done(Message::Git(GitMessage::RefreshDiffFileMetas));
            }
            app.git_diff_contents_loading.remove(&file);
            if !app.git_diff_file_metas.iter().any(|meta| meta.path == file) {
                return Task::none();
            }
            app.git_diff_contents.insert(file, (old_content, new_content));
            Task::none()
        }
        _ => unreachable!("unexpected refresh git message"),
    }
}

//! 本领域完整消息分支；宿主只执行返回的跨域和系统效果。
use super::Context;
use super::Effect;
use super::GitMessage;
use super::State;
use super::Task;
use super::load_missing;
use super::refresh_task;

pub fn update(
    app: &mut State,
    ctx: &Context,
    effects: &mut Vec<Effect>,
    message: GitMessage,
) -> Task<GitMessage> {
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
        GitMessage::RefreshGitPanelData => refresh_task(),
        GitMessage::RefreshWorktreeOptions => {
            let Some(project_path) = crate::message_git::refresh::prepare_worktrees(
                &mut ctx.project_path.clone(),
                &mut app.git_worktree_options,
                &mut app.selected_git_worktree_directory,
                &mut app.git_worktree_options_loading,
                &mut app.git_worktree_options_project_path,
                &mut app.git_worktree_menu_open,
            ) else {
                return Task::none();
            };
            effects.push(Effect::LoadWorktrees(project_path));
            Task::none()
        }
        GitMessage::WorktreeOptionsReady { project_path, result } => {
            let current_directory = app
                .selected_git_worktree_directory
                .clone()
                .or_else(|| ctx.repo_path.clone())
                .or_else(|| ctx.project_path.clone());
            if let Some(error) = crate::message_git::refresh::apply_worktrees(
                &mut app.git_worktree_options_project_path,
                &mut app.git_worktree_options_loading,
                &mut app.git_worktree_options,
                &mut app.selected_git_worktree_directory,
                &mut app.selected_branch,
                &mut app.git_worktree_menu_open,
                project_path,
                result,
                current_directory,
            ) {
                effects
                    .push(Effect::InputError { message: "读取 Git worktree 失败: %s", error });
            }
            Task::none()
        }
        GitMessage::SelectGitWorktree(option) => {
            crate::message_git::refresh::select_git_worktree(
                &mut app.git_worktree_menu_open,
                &mut app.selected_git_worktree_directory,
                &mut app.selected_branch,
                &mut app.git_changed_files,
                &mut app.git_changed_files_loading,
                &mut app.git_changed_files_repo_path,
                &mut app.git_diff_file_metas,
                &mut app.git_diff_file_metas_loading,
                &mut app.git_diff_file_metas_repo_path,
                &mut app.git_diff_contents,
                &mut app.git_diff_contents_loading,
                &mut app.git_diff_selected_lines,
                &mut app.git_diff_selected_range,
                &mut app.git_diff_comment_draft,
                option,
            );
            refresh_task()
        }
        GitMessage::ToggleGitWorktreeMenu(open) => {
            crate::message_git::refresh::toggle_git_worktree_menu(
                &mut app.git_worktree_menu_open,
                open,
            );
            Task::none()
        }
        GitMessage::RefreshChangedFiles => {
            let repo_path = ctx.repo_path.clone();
            let mut manual = ctx.manual_refresh;
            let Some(path) = crate::message_git::refresh::prepare_changed_files(
                &mut app.git_diff_context_menu,
                &mut app.git_changed_files_loading,
                &mut app.git_changed_files,
                &mut app.git_changed_files_repo_path,
                &mut manual,
                repo_path,
            ) else {
                if !manual && ctx.manual_refresh {
                    effects.push(Effect::ClearManualRefresh);
                }
                return Task::none();
            };
            effects.push(Effect::LoadChangedFiles(path));
            Task::none()
        }
        GitMessage::RefreshDiffFileMetas => {
            let repo_path = ctx.repo_path.clone();
            let Some(path) = crate::message_git::refresh::prepare_diff_metas(
                &mut app.git_diff_context_menu,
                &mut app.git_diff_file_metas,
                &mut app.git_diff_file_metas_loading,
                &mut app.git_diff_file_metas_repo_path,
                &mut app.git_diff_contents,
                &mut app.git_diff_contents_loading,
                repo_path,
            ) else {
                return Task::none();
            };
            effects.push(Effect::LoadDiffMetas(path));
            Task::none()
        }
        GitMessage::ChangedFilesReady { repo_path, files } => {
            if !crate::message_git::refresh::apply_changed_files(
                &mut app.git_changed_files_repo_path,
                &mut app.git_changed_files,
                &mut app.git_changed_files_loading,
                &mut app.git_diff_selected_lines,
                &mut app.git_diff_selected_range,
                &mut app.git_diff_comment_draft,
                repo_path,
                files,
            ) {
                return Task::none();
            }
            effects.push(Effect::ChangedFilesApplied);
            Task::none()
        }
        GitMessage::DiffFileMetasReady { repo_path, metas } => {
            if crate::message_git::refresh::apply_diff_metas(
                &mut app.git_diff_file_metas_repo_path,
                &mut app.git_diff_file_metas,
                &mut app.git_diff_file_metas_loading,
                &mut app.git_diff_contents,
                &mut app.git_diff_contents_loading,
                repo_path,
                metas,
            ) {
                {
                    let files = app.expanded_files.clone();
                    load_missing(app, ctx.repo_path.clone(), effects, files)
                }
            } else {
                Task::none()
            }
        }
        GitMessage::LoadDiffContent(file) => {
            load_missing(app, ctx.repo_path.clone(), effects, std::iter::once(file))
        }
        GitMessage::DiffContentReady { repo_path, file, old_content, new_content } => {
            if crate::message_git::refresh::apply_diff_content(
                &mut app.git_diff_file_metas_repo_path,
                &mut app.git_diff_contents_loading,
                &mut app.git_diff_file_metas,
                &mut app.git_diff_contents,
                repo_path,
                file,
                old_content,
                new_content,
            ) {
                Task::done(GitMessage::RefreshDiffFileMetas)
            } else {
                Task::none()
            }
        }
        _ => unreachable!("unexpected refresh git message"),
    }
}

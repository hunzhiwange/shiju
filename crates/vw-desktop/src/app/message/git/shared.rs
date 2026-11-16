//! 提供 Git 消息处理之间共享的状态更新和任务构造辅助逻辑。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use super::{GitMessage, SelectedCommitRequest};
use crate::app::message::project::ProjectMessage;
use crate::app::{
    App, Message,
    state::GitWorktreeOption,
};
#[cfg(test)]
use crate::app::state::{GitDiffLineRange, GitDiffSelectedLine};
use iced::Task;
use std::time::Duration;

/// schedule_commit_button_animation_tick 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(super) fn schedule_commit_button_animation_tick() -> iced::Task<Message> {
    crate::app::message::after(
        Duration::from_millis(90),
        Message::Project(ProjectMessage::FileManagerRefreshAnimationTick),
    )
}

/// dismiss_preview_transient_ui 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn dismiss_preview_transient_ui(app: &mut App) {
    crate::app::message::preview::dismiss_preview_popup_menus(app);
    app.git_diff_file_menu = None;

    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::app::message::preview::lsp::clear_lsp_hover(app);
        crate::app::message::preview::lsp::clear_lsp_completion(app, false);
    }
}

/// git_context_path_for_app 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(crate) fn git_context_path_for_app(app: &App) -> Option<String> {
    let active_directory = app.active_session_id.as_ref().and_then(|id| app.sessions.iter().find(|session| &session.id == id)).map(|session| session.directory.as_str());
    desktop_git_ui::message_git::shared::git_context_path(app.selected_git_worktree_directory.as_deref(), active_directory, app.project_path.as_deref())
}

/// build_selected_commit_request 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
fn build_commit_request(app: &App, stage_all: bool) -> Result<SelectedCommitRequest, String> {
    desktop_git_ui::message_git::shared::build_commit_request(&app.git_commit_message, &app.git_commit_description, &app.git_commit_type, &app.git_commit_scope, &app.staged_files_selected, &app.staged_hunks_selected, &app.staged_lines_selected, &app.staged_old_lines_selected, stage_all)
}

pub(super) fn build_selected_commit_request(app: &App) -> Result<SelectedCommitRequest, String> {
    build_commit_request(app, false)
}

pub(super) fn build_all_commit_request(app: &App) -> Result<SelectedCommitRequest, String> {
    build_commit_request(app, true)
}

/// reset_commit_form_state 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn reset_commit_form_state(app: &mut App) {
    desktop_git_ui::message_git::shared::reset_commit_form_state(&mut app.staged_files_selected, &mut app.staged_hunks_selected, &mut app.staged_lines_selected, &mut app.staged_old_lines_selected, &mut app.expanded_files, &mut app.expanded_files_set, &mut app.expanded_hunks, &mut app.context_expansions, &mut app.git_commit_message, &mut app.git_commit_type, &mut app.git_commit_scope, &mut app.git_commit_description, &mut app.git_commit_description_editor)
}

/// execute_selected_commit_via_gateway 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) async fn execute_selected_commit_via_gateway(
    project_path: String,
    request: SelectedCommitRequest,
) -> Result<(), String> {
    desktop_git_ui::message_git::shared::execute_selected_commit_via_gateway(&crate::app::gateway_client()?, &crate::app::session_gateway::active_tenant_scope(), project_path, request).await
}

pub(super) async fn execute_stage_all_via_gateway(directory: String) -> Result<(), String> {
    desktop_git_ui::message_git::shared::execute_stage_all_via_gateway(&crate::app::gateway_client()?, &crate::app::session_gateway::active_tenant_scope(), directory).await
}

/// refresh_git_panel_data_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(super) fn refresh_git_panel_data_task() -> Task<Message> {
    Task::batch(vec![
        Task::done(Message::Git(GitMessage::RefreshRepository)),
        Task::done(Message::Git(GitMessage::RefreshWorktreeOptions)),
        Task::done(Message::Git(GitMessage::RefreshChangedFiles)),
        Task::done(Message::Git(GitMessage::RefreshDiffFileMetas)),
    ])
}

pub(super) fn refresh_git_panel_data_after_repo_mutation_task(app: &mut App) -> Task<Message> {
    desktop_git_ui::message_git::refresh::reset_loading_after_mutation(
        &mut app.git_changed_files_loading, &mut app.git_changed_files_repo_path,
        &mut app.git_diff_file_metas_loading, &mut app.git_diff_file_metas_repo_path,
        &mut app.git_diff_contents_loading,
    );
    refresh_git_panel_data_task()
}

pub(super) async fn load_git_worktree_options(
    project_path: String,
) -> Result<Vec<GitWorktreeOption>, String> {
    desktop_git_ui::message_git::shared::load_git_worktree_options(&crate::app::gateway_client()?, &crate::app::session_gateway::active_tenant_scope(), project_path).await
}

/// take_diff_content_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(super) fn take_diff_content_task(app: &mut App, file: String) -> Option<Task<Message>> {
    use desktop_git_ui::message_git::refresh::DiffContentLoad;
    let path = crate::app::components::git_panel::git_repo_path_for_app(app);
    let (repo_path, meta) = match desktop_git_ui::message_git::refresh::prepare_diff_content(
        &app.git_diff_contents, &mut app.git_diff_contents_loading,
        app.git_diff_file_metas_repo_path.as_deref(), &app.git_diff_file_metas, path, &file,
    ) {
        DiffContentLoad::None => return None,
        DiffContentLoad::RefreshMetas => return Some(Task::done(Message::Git(GitMessage::RefreshDiffFileMetas))),
        DiffContentLoad::Load { repo_path, meta } => (repo_path, meta),
    };

    Some(Task::perform(
        async move {
            let load_repo_path = repo_path.clone();
            let load_meta = meta.clone();
            let (old_content, new_content) = crate::app::message::spawn_blocking_opt(move || {
                Some(crate::app::components::git_panel::load_diff_content_for_repo_path(
                    &load_repo_path,
                    &load_meta,
                ))
            })
            .await
            .unwrap_or_default();
            (Some(repo_path), file, old_content, new_content)
        },
        |(repo_path, file, old_content, new_content)| {
            Message::Git(GitMessage::DiffContentReady { repo_path, file, old_content, new_content })
        },
    ))
}

/// load_missing_diff_contents 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(super) fn load_missing_diff_contents<I>(app: &mut App, files: I) -> Task<Message>
where
    I: IntoIterator<Item = String>,
{
    let tasks: Vec<_> =
        files.into_iter().filter_map(|file| take_diff_content_task(app, file)).collect();

    if tasks.is_empty() { Task::none() } else { Task::batch(tasks) }
}

#[cfg(not(target_arch = "wasm32"))]
/// git_repo_path_for_app 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn git_repo_path_for_app(app: &App) -> Option<String> {
    crate::app::components::git_panel::git_repo_path_for_app(app)
}

#[cfg(not(target_arch = "wasm32"))]
/// changed_diff_line_sets 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) fn changed_diff_line_sets(
    app: &App,
    file: &str,
) -> Option<(std::collections::HashSet<usize>, std::collections::HashSet<usize>)> {
    desktop_git_ui::message_git::shared::changed_diff_line_sets(&git_repo_path_for_app(app)?, file)
}

/// selected_lines_from_range 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
#[cfg(test)]
pub(super) fn selected_lines_from_range(app: &App, range: &GitDiffLineRange) -> Vec<GitDiffSelectedLine> {
    #[cfg(not(target_arch = "wasm32"))]
    let path = git_repo_path_for_app(app);
    #[cfg(target_arch = "wasm32")]
    let path: Option<String> = None;
    desktop_git_ui::message_git::shared::selected_lines_from_range(path.as_deref(), range)
}

pub(super) fn persist_file_tree_expanded(app: &mut App) -> Task<Message> {
    app.save_desktop_preference("file_tree_expanded", serde_json::json!(app.file_tree_expanded))
}

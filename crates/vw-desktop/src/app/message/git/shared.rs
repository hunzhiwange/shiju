//! 提供 Git 消息处理之间共享的状态更新和任务构造辅助逻辑。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use crate::app::message::project::ProjectMessage;
use crate::app::{App, Message, state::GitWorktreeOption};
use desktop_git_ui::message_git::shared::SelectedCommitRequest;
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
    let active_directory = app.session_state.active_session_id
        .as_ref()
        .and_then(|id| app.session_state.sessions.iter().find(|session| &session.id == id))
        .map(|session| session.directory.as_str());
    desktop_git_ui::message_git::shared::git_context_path(
        app.git.selected_git_worktree_directory.as_deref(),
        active_directory,
        app.navigation_tabs.project_path.as_deref(),
    )
}

/// execute_selected_commit_via_gateway 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 变更范围限制在当前消息处理路径内，不引入额外的流程分支。
pub(super) async fn execute_selected_commit_via_gateway(
    project_path: String,
    request: SelectedCommitRequest,
) -> Result<(), String> {
    desktop_git_ui::message_git::shared::execute_selected_commit_via_gateway(
        &crate::app::gateway_client()?,
        &crate::app::session_gateway::active_tenant_scope(),
        project_path,
        request,
    )
    .await
}

pub(super) async fn execute_stage_all_via_gateway(directory: String) -> Result<(), String> {
    desktop_git_ui::message_git::shared::execute_stage_all_via_gateway(
        &crate::app::gateway_client()?,
        &crate::app::session_gateway::active_tenant_scope(),
        directory,
    )
    .await
}

/// refresh_git_panel_data_task 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(super) fn refresh_git_panel_data_after_repo_mutation_task(app: &mut App) -> Task<Message> {
    desktop_git_ui::message_git::controller::after_mutation(&mut app.git).map(Message::Git)
}

pub(super) async fn load_git_worktree_options(
    project_path: String,
) -> Result<Vec<GitWorktreeOption>, String> {
    desktop_git_ui::message_git::shared::load_git_worktree_options(
        &crate::app::gateway_client()?,
        &crate::app::session_gateway::active_tenant_scope(),
        project_path,
    )
    .await
}

pub(super) fn persist_file_tree_expanded(app: &mut App) -> Task<Message> {
    app.save_desktop_preference("file_tree_expanded", serde_json::json!(app.workspace_state.file_tree_expanded))
}

//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::config::gateway_client_endpoint;
use crate::app::i18n::I18n;
use crate::app::projects::{save_recent_projects_background, save_recent_projects_meta_background};
use crate::app::{App, Message};
use iced::Task;
#[cfg(test)]
use serde_json::{Value, json};
use vw_gateway_client::GatewayClient;

use super::messages::SettingsMessage;

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        event @ (SettingsMessage::RecentProjectQueryChanged(_)
        | SettingsMessage::RecentProjectDeleteRequested(_)
        | SettingsMessage::RecentProjectDeleteCanceled
        | SettingsMessage::RecentProjectDeleteConfirmed(_)) => {
            use desktop_settings_workspace::projects::{self, Event, State};
            let event = match event {
                SettingsMessage::RecentProjectQueryChanged(query) => Event::QueryChanged(query),
                SettingsMessage::RecentProjectDeleteRequested(index) => {
                    Event::DeleteRequested(index)
                }
                SettingsMessage::RecentProjectDeleteCanceled => Event::DeleteCanceled,
                SettingsMessage::RecentProjectDeleteConfirmed(index) => {
                    Event::DeleteConfirmed(index)
                }
                _ => unreachable!(),
            };
            let deleted = projects::apply_with_metadata(
                State {
                    query: &mut app.workspace_state.recent_project_query,
                    tools_menu_path: &mut app.workspace_state.project_tools_menu_path,
                    delete_confirm_idx: &mut app.workspace_state.recent_project_delete_confirm_idx,
                    paths: &mut app.workspace_state.recent_projects,
                    edits: &mut app.workspace_state.recent_projects_edits,
                },
                event,
                app.navigation_tabs.project_path.as_deref(),
                &mut app.workspace_state.recent_projects_meta,
            );
            let mut deleting_current = false;
            let mut save_worktree = Task::none();
            if let Some(deleted) = deleted {
                let path = deleted.path;
                deleting_current = deleted.was_current;
                save_recent_projects_background(app.workspace_state.recent_projects.clone());
                save_recent_projects_meta_background(app.workspace_state.recent_projects_meta.clone());
                save_worktree = set_worktree_enabled(app, path, false);
            }
            if deleting_current {
                Task::batch([save_worktree, app.open_first_recent_project_or_home()])
            } else {
                save_worktree
            }
        }
        SettingsMessage::ProjectEnableWorktreeToggled(path, enabled) => {
            set_worktree_enabled(app, path, enabled)
        }
        SettingsMessage::ProjectWorktreeSaved(result) => {
            app.workspace_state.project_worktree_save_running = false;
            let next = start_worktree_save(app);
            #[cfg(not(target_arch = "wasm32"))]
            if !app.workspace_state.project_worktree_save_running
                && let Some(idle) = app.workspace_state.project_worktree_save_idle.take()
            {
                idle.send_replace(true);
            }
            if let Err(error) = result {
                tracing::warn!(target: "vw_desktop", %error, "failed to save project worktree setting");
                let label = I18n::new(app.general_settings.desktop_locale).__("Worktree 设置保存失败。");
                return Task::batch([next, app.show_error_toast(label)]);
            }
            next
        }
        _ => Task::none(),
    }
}

/// 应用 Worktree 选项并安排单项目配置写入。
pub(crate) fn set_worktree_enabled(app: &mut App, path: String, enabled: bool) -> Task<Message> {
    let Some(patch) = desktop_settings_workspace::projects::set_worktree_enabled(
        &mut app.workspace_state.project_worktree_enabled,
        &path,
        enabled,
    ) else {
        return Task::none();
    };

    #[cfg(not(target_arch = "wasm32"))]
    app.note_startup_worktree_edit(&path);
    let endpoint = gateway_client_endpoint();
    app.workspace_state.project_worktree_save_queue.push_back(Task::perform(
        async move {
            gateway_desktop_settings::GatewayDesktopSettingsClient::new(
                GatewayClient::new(endpoint)?.transport().clone(),
            )
            .desktop_system_settings_patch(&patch)
            .await
        },
        |result| Message::Settings(SettingsMessage::ProjectWorktreeSaved(result)),
    ));
    start_worktree_save(app)
}

/// Keep the existing project editor cache aligned with an acknowledged gateway write.
pub(crate) fn environment_command_saved(app: &mut App, path: &str, command: Option<String>) {
    if let Some(meta) = app.workspace_state.recent_projects_meta.iter_mut().find(|meta| meta.path == path) {
        meta.worktree_start_command = command;
        save_recent_projects_meta_background(app.workspace_state.recent_projects_meta.clone());
    }
}

fn start_worktree_save(app: &mut App) -> Task<Message> {
    if app.workspace_state.project_worktree_save_running {
        return Task::none();
    }
    let Some(task) = app.workspace_state.project_worktree_save_queue.pop_front() else {
        return Task::none();
    };
    app.workspace_state.project_worktree_save_running = true;
    task
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn wait_for_worktree_saves(app: &mut App) -> Option<tokio::sync::watch::Receiver<bool>> {
    if !app.workspace_state.project_worktree_save_running {
        return None;
    }
    let (sender, receiver) = tokio::sync::watch::channel(false);
    app.workspace_state.project_worktree_save_idle = Some(sender);
    Some(receiver)
}

#[cfg(test)]
#[path = "projects_tests.rs"]
mod projects_tests;

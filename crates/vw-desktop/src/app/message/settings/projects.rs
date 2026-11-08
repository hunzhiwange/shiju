//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::config::gateway_client_endpoint;
use crate::app::i18n::I18n;
use crate::app::projects::{save_recent_projects_background, save_recent_projects_meta_background};
use crate::app::{App, Message};
use iced::Task;
use serde_json::{Value, json};
use vw_gateway_client::GatewayClient;

use super::messages::SettingsMessage;

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::RecentProjectQueryChanged(query) => {
            app.recent_project_query = query;
            app.project_tools_menu_path = None;
            app.recent_project_delete_confirm_idx = None;
            Task::none()
        }
        SettingsMessage::RecentProjectDeleteRequested(idx) => {
            app.project_tools_menu_path = None;
            app.recent_project_delete_confirm_idx = Some(idx);
            Task::none()
        }
        SettingsMessage::RecentProjectDeleteCanceled => {
            app.recent_project_delete_confirm_idx = None;
            Task::none()
        }
        SettingsMessage::RecentProjectDeleteConfirmed(idx) => {
            let deleting_current = app
                .recent_projects
                .get(idx)
                .is_some_and(|path| app.project_path.as_ref() == Some(path));
            let mut save_worktree = Task::none();
            if idx < app.recent_projects.len() {
                let path = app.recent_projects.remove(idx);
                if idx < app.recent_projects_edits.len() {
                    app.recent_projects_edits.remove(idx);
                }
                app.recent_projects_meta.retain(|m| m.path != path);
                save_recent_projects_background(app.recent_projects.clone());
                save_recent_projects_meta_background(app.recent_projects_meta.clone());
                save_worktree = set_worktree_enabled(app, path, false);
                app.recent_projects_edits = app
                    .recent_projects
                    .iter()
                    .map(|p| {
                        if let Some(m) = app.recent_projects_meta.iter().find(|m| &m.path == p) {
                            m.name.clone()
                        } else {
                            std::path::Path::new(p)
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or(p)
                                .to_string()
                        }
                    })
                    .collect();
            }
            app.recent_project_delete_confirm_idx = None;
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
            app.project_worktree_save_running = false;
            let next = start_worktree_save(app);
            #[cfg(not(target_arch = "wasm32"))]
            if !app.project_worktree_save_running
                && let Some(idle) = app.project_worktree_save_idle.take()
            {
                idle.send_replace(true);
            }
            if let Err(error) = result {
                tracing::warn!(target: "vw_desktop", %error, "failed to save project worktree setting");
                let label = I18n::new(app.desktop_locale).__("Worktree 设置保存失败。");
                return Task::batch([next, app.show_error_toast(label)]);
            }
            next
        }
        _ => Task::none(),
    }
}

/// 应用 Worktree 选项并安排单项目配置写入。
pub(crate) fn set_worktree_enabled(app: &mut App, path: String, enabled: bool) -> Task<Message> {
    let changed = if enabled {
        app.project_worktree_enabled.insert(path.clone(), true) != Some(true)
    } else {
        app.project_worktree_enabled.remove(&path).is_some()
    };
    if !changed {
        return Task::none();
    }

    let endpoint = gateway_client_endpoint();
    // The gateway recursively merges the patch. Null removes the project key.
    let value = if enabled { Value::Bool(true) } else { Value::Null };
    let patch = json!({ "project_worktree_enabled": { path: value } });
    app.project_worktree_save_queue.push_back(Task::perform(
        async move { GatewayClient::new(endpoint)?.desktop_system_settings_patch(&patch).await },
        |result| Message::Settings(SettingsMessage::ProjectWorktreeSaved(result)),
    ));
    start_worktree_save(app)
}

fn start_worktree_save(app: &mut App) -> Task<Message> {
    if app.project_worktree_save_running {
        return Task::none();
    }
    let Some(task) = app.project_worktree_save_queue.pop_front() else {
        return Task::none();
    };
    app.project_worktree_save_running = true;
    task
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn wait_for_worktree_saves(app: &mut App) -> Option<tokio::sync::watch::Receiver<bool>> {
    if !app.project_worktree_save_running {
        return None;
    }
    let (sender, receiver) = tokio::sync::watch::channel(false);
    app.project_worktree_save_idle = Some(sender);
    Some(receiver)
}

#[cfg(test)]
#[path = "projects_tests.rs"]
mod projects_tests;

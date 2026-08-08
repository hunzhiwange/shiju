//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::config::update_system_settings_config;
use crate::app::projects::{save_recent_projects_background, save_recent_projects_meta_background};
use crate::app::{
    App, Message,
    state::{
        RecentProjectMeta, default_recent_project_session_auto_refresh,
        default_recent_project_session_refresh_interval_seconds,
    },
};
use iced::Task;

use super::messages::SettingsMessage;

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::RecentProjectRenameChanged(idx, name) => {
            if idx < app.recent_projects_edits.len() {
                app.recent_projects_edits[idx] = name;
            }
            Task::none()
        }
        SettingsMessage::RecentProjectRenameSave(idx) => {
            if idx < app.recent_projects_edits.len() && idx < app.recent_projects.len() {
                let path = app.recent_projects[idx].clone();
                let name = app.recent_projects_edits[idx].clone();
                if let Some(m) = app.recent_projects_meta.iter_mut().find(|m| m.path == path) {
                    m.name = name;
                } else {
                    app.recent_projects_meta.push(RecentProjectMeta {
                        path,
                        name,
                        task_board_settings: None,
                        session_auto_refresh: default_recent_project_session_auto_refresh(),
                        session_refresh_interval_seconds:
                            default_recent_project_session_refresh_interval_seconds(),
                        icon: None,
                        icon_color: None,
                        worktree_start_command: None,
                    });
                }
                save_recent_projects_meta_background(app.recent_projects_meta.clone());
            }
            Task::none()
        }
        SettingsMessage::RecentProjectDeleteRequested(idx) => {
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
            if idx < app.recent_projects.len() {
                let path = app.recent_projects.remove(idx);
                if idx < app.recent_projects_edits.len() {
                    app.recent_projects_edits.remove(idx);
                }
                app.recent_projects_meta.retain(|m| m.path != path);
                app.project_worktree_enabled.remove(&path);
                save_recent_projects_background(app.recent_projects.clone());
                save_recent_projects_meta_background(app.recent_projects_meta.clone());
                let project_worktree_enabled = app.project_worktree_enabled.clone();
                update_system_settings_config(|system| {
                    system.project_worktree_enabled = project_worktree_enabled;
                });
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
                app.open_first_recent_project_or_home()
            } else {
                Task::none()
            }
        }
        SettingsMessage::ProjectEnableWorktreeToggled(path, v) => {
            if v {
                app.project_worktree_enabled.insert(path, true);
            } else {
                app.project_worktree_enabled.remove(&path);
            }
            let project_worktree_enabled = app.project_worktree_enabled.clone();
            update_system_settings_config(|system| {
                system.project_worktree_enabled = project_worktree_enabled;
            });
            Task::none()
        }
        _ => Task::none(),
    }
}

#[cfg(test)]
#[path = "projects_tests.rs"]
mod projects_tests;

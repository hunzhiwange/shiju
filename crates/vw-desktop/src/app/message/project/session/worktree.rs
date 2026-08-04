//! 处理项目会话工作区相关操作，包括分支、工作树和路径状态更新。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use super::common::{
    clear_new_session_picker_messages, reset_new_session_picker_state, save_config_field_task,
};
use crate::app::message::project::ProjectMessage;
use crate::app::message::project::helpers::{
    create_gateway_session_in_directory, create_gateway_worktree_session, delete_gateway_worktree,
    reset_gateway_worktree,
};
use crate::app::{App, Message, state::QuickMessageHandoffState};

/// handle 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(crate) fn handle(app: &mut App, message: ProjectMessage) -> Option<iced::Task<Message>> {
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
        ProjectMessage::ProjectCreateSessionPickerLoaded { project_path, options } => {
            if app.new_session_picker_project.as_ref() != Some(&project_path) {
                return Some(iced::Task::none());
            }
            match options {
                Ok(mut list) => {
                    if let Some(last_directory) = app.new_session_last_directory.as_ref()
                        && let Some(pos) =
                            list.iter().position(|(directory, _)| directory == last_directory)
                    {
                        let item = list.remove(pos);
                        let insert_at =
                            if !list.is_empty() && list[0].1 == "主工作区" { 1 } else { 0 };
                        list.insert(insert_at, item);
                    }
                    app.new_session_picker_options = list;
                }
                Err(e) => {
                    app.error_message = Some(format!("加载工作区列表失败: {}", e));
                    app.new_session_picker_options =
                        vec![("__create_worktree__".to_string(), "创建新的独立工作区".to_string())];
                }
            }
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectCreateSessionPickerClose => {
            app.hovered_recent_project = None;
            reset_new_session_picker_state(app);
            if matches!(
                app.quick_message_handoff,
                Some(QuickMessageHandoffState::WaitingForSession { .. })
            ) {
                app.quick_message_handoff = None;
            }
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectCreateSessionWorktreeNameChanged(v) => {
            app.new_session_worktree_name = v;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectCreateSessionPicked { project_path: _, directory } => {
            app.new_session_last_directory = Some(directory.clone());
            let save_last_directory_task = save_config_field_task(
                "new_session_last_directory",
                serde_json::Value::String(directory.clone()),
            );
            reset_new_session_picker_state(app);
            Some(iced::Task::batch(vec![
                save_last_directory_task,
                iced::Task::perform(
                    async move { create_gateway_session_in_directory(directory).await },
                    |res| match res {
                        Ok(info) => Message::Project(ProjectMessage::SessionCreated(info)),
                        Err(error) => Message::Project(ProjectMessage::SessionCreateFailed(error)),
                    },
                ),
            ]))
        }
        ProjectMessage::ProjectCreateSessionWorktree(project_path) => {
            let requested_name = app.new_session_worktree_name.trim().to_string();
            if requested_name.is_empty() {
                app.error_message = Some("请输入 worktree 英文名称".to_string());
                return Some(iced::Task::none());
            }
            let valid = requested_name
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-');
            if !valid {
                app.error_message =
                    Some("worktree 名称仅支持小写英文、数字、连字符(-)".to_string());
                return Some(iced::Task::none());
            }
            reset_new_session_picker_state(app);
            let project_path_clone = project_path.clone();
            Some(iced::Task::perform(
                async move { create_gateway_worktree_session(project_path_clone, requested_name).await },
                |res| match res {
                    Ok(info) => Message::Project(ProjectMessage::SessionCreated(info)),
                    Err(error) => Message::Project(ProjectMessage::SessionCreateFailed(error)),
                },
            ))
        }
        ProjectMessage::ProjectCreateSessionDeleteWorktree(directory) => {
            let is_primary = app
                .new_session_picker_options
                .iter()
                .any(|(d, label)| d == &directory && label == "主工作区");
            if is_primary || directory == app.new_session_picker_project.clone().unwrap_or_default()
            {
                app.new_session_delete_error = Some("主工作区禁止删除".to_string());
                app.new_session_confirm_delete_directory = None;
                app.new_session_force_delete_directory = None;
                app.new_session_confirm_reset_directory = None;
                return Some(iced::Task::none());
            }
            app.new_session_confirm_delete_directory = Some(directory);
            app.new_session_delete_error = None;
            app.new_session_force_delete_directory = None;
            app.new_session_confirm_reset_directory = None;
            app.new_session_reset_error = None;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectCreateSessionDeleteWorktreeCancel => {
            clear_new_session_picker_messages(app);
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectCreateSessionDeleteWorktreeConfirmed => {
            let Some(project_path) = app.new_session_picker_project.clone() else {
                return Some(iced::Task::none());
            };
            let Some(directory) = app.new_session_confirm_delete_directory.clone() else {
                return Some(iced::Task::none());
            };
            if directory == project_path {
                app.new_session_delete_error = Some("主工作区禁止删除".to_string());
                app.new_session_confirm_delete_directory = None;
                app.new_session_force_delete_directory = None;
                return Some(iced::Task::none());
            }
            clear_new_session_picker_messages(app);
            let delete_directory = directory.clone();
            let result_directory = directory.clone();
            let project_path_clone = project_path.clone();
            Some(iced::Task::perform(
                async move {
                    delete_gateway_worktree(&project_path_clone, &delete_directory, false).await
                },
                move |result| {
                    Message::Project(ProjectMessage::ProjectCreateSessionDeleteWorktreeResult {
                        project_path,
                        directory: result_directory.clone(),
                        result,
                    })
                },
            ))
        }
        ProjectMessage::ProjectCreateSessionDeleteWorktreeForceConfirmed => {
            let Some(project_path) = app.new_session_picker_project.clone() else {
                return Some(iced::Task::none());
            };
            let Some(directory) = app.new_session_force_delete_directory.clone() else {
                return Some(iced::Task::none());
            };
            app.new_session_force_delete_directory = None;
            let delete_directory = directory.clone();
            let result_directory = directory.clone();
            let project_path_clone = project_path.clone();
            Some(iced::Task::perform(
                async move {
                    delete_gateway_worktree(&project_path_clone, &delete_directory, true).await
                },
                move |result| {
                    Message::Project(ProjectMessage::ProjectCreateSessionDeleteWorktreeResult {
                        project_path,
                        directory: result_directory.clone(),
                        result,
                    })
                },
            ))
        }
        ProjectMessage::ProjectCreateSessionDeleteWorktreeResult {
            project_path,
            directory,
            result,
        } => {
            if app.new_session_picker_project.as_ref() != Some(&project_path) {
                return Some(iced::Task::none());
            }

            match result {
                Ok(()) => {
                    app.new_session_delete_error = None;
                    app.new_session_force_delete_directory = None;
                    app.new_session_reset_error = None;
                    if app.new_session_last_directory.as_ref() == Some(&directory) {
                        app.new_session_last_directory = None;
                        return Some(iced::Task::batch(vec![
                            save_config_field_task(
                                "new_session_last_directory",
                                serde_json::Value::Null,
                            ),
                            iced::Task::done(Message::Project(
                                ProjectMessage::ProjectCreateSession(project_path),
                            )),
                        ]));
                    }
                    return Some(iced::Task::done(Message::Project(
                        ProjectMessage::ProjectCreateSession(project_path),
                    )));
                }
                Err(e) => {
                    app.new_session_delete_error = Some(e);
                    app.new_session_force_delete_directory = Some(directory);
                }
            }
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectCreateSessionResetWorktree(directory) => {
            let is_primary = app
                .new_session_picker_options
                .iter()
                .any(|(d, label)| d == &directory && label == "主工作区");
            if is_primary || directory == app.new_session_picker_project.clone().unwrap_or_default()
            {
                app.new_session_reset_error = Some("主工作区禁止重置".to_string());
                app.new_session_confirm_reset_directory = None;
                app.new_session_confirm_delete_directory = None;
                app.new_session_force_delete_directory = None;
                return Some(iced::Task::none());
            }
            app.new_session_confirm_reset_directory = Some(directory);
            app.new_session_reset_error = None;
            app.new_session_confirm_delete_directory = None;
            app.new_session_force_delete_directory = None;
            app.new_session_delete_error = None;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectCreateSessionResetWorktreeCancel => {
            app.new_session_confirm_reset_directory = None;
            app.new_session_reset_error = None;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectCreateSessionResetWorktreeConfirmed => {
            let Some(project_path) = app.new_session_picker_project.clone() else {
                return Some(iced::Task::none());
            };
            let Some(directory) = app.new_session_confirm_reset_directory.clone() else {
                return Some(iced::Task::none());
            };
            if directory == project_path {
                app.new_session_reset_error = Some("主工作区禁止重置".to_string());
                app.new_session_confirm_reset_directory = None;
                return Some(iced::Task::none());
            }
            app.new_session_confirm_reset_directory = None;
            app.new_session_reset_error = None;
            let reset_directory = directory.clone();
            let result_directory = directory.clone();
            let project_path_clone = project_path.clone();
            Some(iced::Task::perform(
                async move { reset_gateway_worktree(&project_path_clone, &reset_directory).await },
                move |result| {
                    Message::Project(ProjectMessage::ProjectCreateSessionResetWorktreeResult {
                        project_path,
                        directory: result_directory.clone(),
                        result,
                    })
                },
            ))
        }
        ProjectMessage::ProjectCreateSessionResetWorktreeResult {
            project_path,
            directory: _,
            result,
        } => {
            if app.new_session_picker_project.as_ref() != Some(&project_path) {
                return Some(iced::Task::none());
            }

            match result {
                Ok(()) => {
                    app.new_session_reset_error = None;
                    app.push_notification("工作区已重置".to_string());
                }
                Err(e) => {
                    app.new_session_reset_error = Some(e);
                }
            }
            Some(iced::Task::none())
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "worktree_tests.rs"]
mod worktree_tests;

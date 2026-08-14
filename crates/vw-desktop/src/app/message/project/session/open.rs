//! 处理项目会话打开流程，把用户选择映射为活动会话和加载任务。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use super::common::{clear_new_session_picker_messages, reset_new_session_picker_state};
use crate::app::message::project::ProjectMessage;
use crate::app::message::project::helpers::{
    create_gateway_session_in_directory, load_project_worktree_picker_options,
    load_session_messages_task, prepare_session_ui_task,
};
use crate::app::projects::save_recent_projects_meta_background;
use crate::app::{App, Message, models};

fn merge_project_sessions(
    existing: Option<Vec<vw_shared::session::info::Info>>,
    incoming: Vec<vw_shared::session::info::Info>,
) -> Vec<vw_shared::session::info::Info> {
    let Some(existing) = existing else {
        return incoming;
    };

    let mut merged = incoming;
    for session in existing {
        if !merged.iter().any(|item| item.id == session.id) {
            merged.push(session);
        }
    }
    merged
}

#[cfg(not(target_arch = "wasm32"))]
fn load_session_ui_store_task(project_path: String, session_id: String) -> iced::Task<Message> {
    iced::Task::perform(
        async move {
            let result =
                crate::app::session_gateway::gateway_load_session_any_async(&session_id).await;
            (session_id, project_path, result)
        },
        |(session_id, project_path, result)| {
            Message::Project(ProjectMessage::SessionUiStoreLoaded {
                session_id,
                project_path,
                result,
            })
        },
    )
}

/// handle 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(crate) fn handle(app: &mut App, message: ProjectMessage) -> Option<iced::Task<Message>> {
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
        ProjectMessage::OpenRecentPressed(path) => {
            app.hovered_recent_project = None;
            app.session_menu_id = None;
            app.session_menu_anchor = None;
            reset_new_session_picker_state(app);
            Some(app.open_project_and_index(path))
        }
        ProjectMessage::SwitchWorkSpacePressed(path) => {
            app.hovered_recent_project = None;
            app.session_menu_id = None;
            app.session_menu_anchor = None;
            reset_new_session_picker_state(app);
            Some(app.switch_work_space_preserving_chat(path))
        }
        ProjectMessage::OpenProjectSessionPressed(path, id) => {
            app.show_agent_sessions_popover = false;
            let resolved_path = app
                .known_session_directory(&id)
                .filter(|directory| !directory.trim().is_empty())
                .unwrap_or_else(|| path.clone());
            if resolved_path != path {
                tracing::warn!(
                    target: "vw_desktop",
                    session_id = %id,
                    requested_path = %path,
                    resolved_path = %resolved_path,
                    "corrected project session open path to the session directory"
                );
            } else {
                tracing::info!(
                    target: "vw_desktop",
                    session_id = %id,
                    project_path = %resolved_path,
                    "opening project session"
                );
            }
            app.hovered_recent_project = None;
            app.session_menu_id = None;
            app.session_menu_anchor = None;
            app.cache_active_session_chat();

            let open_task = if app.project_path.as_ref() == Some(&resolved_path) {
                iced::Task::none()
            } else {
                app.open_project(resolved_path.clone())
            };

            app.prepare_historical_session_model(&id);
            app.active_session_id = Some(id.clone());
            app.chat_scroll_id = iced::widget::Id::unique();
            app.mark_active_session_viewed();
            app.restore_chat_for_session(&id);
            app.usage = models::TokenUsage::default();
            app.active_session_view_state.updated_ms = 0;
            app.clear_active_session_steps();
            app.active_session_view_state.ui_preparing = true;
            app.active_session_view_state.base_ready = false;
            app.invalidate_chat_ui_state();
            // plan-run 子会话进入后关闭 workflow_mode，避免续聊误触发 plan-run。
            let is_plan_run_child = app
                .sessions
                .iter()
                .find(|session| session.id == id)
                .and_then(|session| session.parent_id.as_deref())
                .or_else(|| {
                    app.project_sessions
                        .values()
                        .flat_map(|sessions| sessions.iter())
                        .find(|session| session.id == id)
                        .and_then(|session| session.parent_id.as_deref())
                })
                .is_some_and(|parent_id| !parent_id.trim().is_empty());
            if is_plan_run_child {
                app.get_session_runtime_mut(&id).workflow_mode_enabled = false;
                app.get_session_runtime_mut(&id).goal_mode_enabled = false;
            }
            app.sync_active_session_preferences();
            tracing::info!(
                target: "vw_desktop",
                session_id = %id,
                project_path = %resolved_path,
                cached_messages = app.chat.len(),
                "prepared active session state before loading history"
            );

            let base_chunk_start = app.preferred_base_chat_ui_chunk_start();
            let initial_prewarm_task = if app.chat.is_empty() {
                iced::Task::none()
            } else {
                app.mark_chat_ui_chunks_preparing(&[base_chunk_start]);
                app.pin_chat_ui_chunk(Some(base_chunk_start));
                prepare_session_ui_task(
                    id.clone(),
                    app.active_shared_chat_messages(),
                    base_chunk_start,
                    true,
                    app.dialogue_flow_show_reasoning_summary,
                )
            };

            let has_requesting_cache =
                app.session_is_requesting(&id) && app.session_chat_cache.contains_key(&id);
            #[cfg(not(target_arch = "wasm32"))]
            let history_load_task = if app.chat.is_empty() && !app.session_is_requesting(&id) {
                load_session_ui_store_task(resolved_path.clone(), id.clone())
            } else if !has_requesting_cache {
                load_session_messages_task(resolved_path.clone(), id.clone())
            } else {
                iced::Task::none()
            };
            #[cfg(target_arch = "wasm32")]
            let history_load_task = if !has_requesting_cache {
                load_session_messages_task(resolved_path.clone(), id.clone())
            } else {
                iced::Task::none()
            };
            tracing::info!(
                target: "vw_desktop",
                session_id = %id,
                project_path = %resolved_path,
                has_requesting_cache,
                current_messages = app.chat.len(),
                "resolved session history bootstrap strategy"
            );
            Some(iced::Task::batch(vec![
                open_task,
                initial_prewarm_task,
                history_load_task,
                iced::Task::done(Message::Chat(
                    crate::app::message::ChatMessage::LoadInputPanelTodos,
                )),
            ]))
        }
        ProjectMessage::SessionUiStoreLoaded { session_id, project_path, result } => {
            if app.active_session_id.as_ref() != Some(&session_id) {
                return Some(iced::Task::none());
            }

            let mut session = match result {
                Ok(Some(session)) if !session.messages.is_empty() => session,
                Ok(_) => {
                    return Some(load_session_messages_task(project_path, session_id));
                }
                Err(error) => {
                    tracing::warn!(
                        target: "vw_desktop",
                        session_id = %session_id,
                        error = %error,
                        "failed to load session UI snapshot; falling back to message history"
                    );
                    return Some(load_session_messages_task(project_path, session_id));
                }
            };

            let missing_message_model_count =
                session.messages.iter().filter(|message| message.model.is_none()).count();
            crate::app::session::backfill_message_models_from_steps(
                &mut session.messages,
                &session.steps,
            );
            let backfilled_message_model = session
                .messages
                .iter()
                .filter(|message| message.model.is_none())
                .count()
                < missing_message_model_count;
            let persist_backfill_task = if backfilled_message_model {
                let session = session.clone();
                let directory = project_path.clone();
                iced::Task::perform(
                    async move {
                        crate::app::session_gateway::gateway_save_session_async(
                            &session,
                            Some(&directory),
                        )
                        .await
                    },
                    |_| Message::None,
                )
            } else {
                iced::Task::none()
            };

            let usage = session.steps.iter().fold(
                models::TokenUsage::default(),
                |mut acc, step| {
                    acc.input_tokens += step.usage.input_tokens;
                    acc.output_tokens += step.usage.output_tokens;
                    acc.cached_tokens += step.usage.cached_tokens;
                    acc.reasoning_tokens += step.usage.reasoning_tokens;
                    acc
                },
            );
            let shared_chat =
                crate::app::session::shared_chat_messages(session.messages.clone());
            app.chat = shared_chat.iter().cloned().collect();
            app.chat_message_ids = if session.message_ids.len() == app.chat.len() {
                session.message_ids.clone()
            } else {
                vec![None; app.chat.len()]
            };
            let has_missing_message_ids = app.chat_message_ids.iter().any(Option::is_none);
            app.store_session_chat_snapshot(
                session_id.clone(),
                shared_chat.clone(),
                app.chat_message_ids.clone(),
            );
            app.usage = usage;
            app.active_session_view_state.updated_ms = session.updated_ms;
            app.replace_active_session_steps(session.steps.clone());
            app.replace_active_session_calls(session.calls.clone());
            if !has_missing_message_ids
                && app.restore_session_model_from_steps(&session_id, &session.steps)
            {
                app.sync_active_session_preferences();
            }

            app.active_session_view_state.ui_preparing = true;
            app.active_session_view_state.base_ready = false;
            app.invalidate_chat_ui_state();
            let base_chunk_start = app.preferred_base_chat_ui_chunk_start();
            app.mark_chat_ui_chunks_preparing(&[base_chunk_start]);
            app.pin_chat_ui_chunk(Some(base_chunk_start));
            let prewarm_task = prepare_session_ui_task(
                session_id.clone(),
                shared_chat,
                base_chunk_start,
                true,
                app.dialogue_flow_show_reasoning_summary,
            );
            let remote_refresh_task = if has_missing_message_ids {
                load_session_messages_task(project_path, session_id)
            } else {
                iced::Task::none()
            };

            Some(iced::Task::batch([
                persist_backfill_task,
                prewarm_task,
                remote_refresh_task,
            ]))
        }
        ProjectMessage::RecentHovered(id) => {
            app.hovered_recent_project = id.clone();
            if let Some(path) = id
                && !app.project_sessions.contains_key(&path)
                && !app.project_sessions_loading.contains(&path)
            {
                app.project_sessions_loading.insert(path.clone());
                let project_path_clone = path.clone();
                return Some(iced::Task::perform(
                    async move {
                        let client = crate::app::gateway_client().map_err(|err| err.to_string())?;
                        client
                            .session_list::<Vec<vw_shared::session::info::Info>>(Some(
                                &project_path_clone,
                            ))
                            .await
                    },
                    |res| Message::Project(ProjectMessage::ProjectSessionsLoaded(path, res)),
                ));
            }
            Some(iced::Task::done(Message::Chat(
                crate::app::message::ChatMessage::LoadInputPanelTodos,
            )))
        }
        ProjectMessage::RecentOverlayClosed => {
            if app.new_session_picker_project.is_some() {
                return Some(iced::Task::none());
            }
            app.hovered_recent_project = None;
            app.session_menu_id = None;
            app.session_menu_anchor = None;
            app.project_tools_menu_path = None;
            reset_new_session_picker_state(app);
            Some(iced::Task::none())
        }
        ProjectMessage::SessionsLoaded(res) => {
            match res {
                Ok(sessions) => {
                    app.sessions = sessions;
                }
                Err(e) => {
                    app.push_notification(format!("Failed to load sessions: {}", e));
                }
            }
            Some(iced::Task::none())
        }
        ProjectMessage::SessionBootstrapLoaded {
            project_path,
            result,
            previews,
            archived_session_ids,
        } => {
            if let Some(path) = project_path.as_ref() {
                app.project_sessions_loading.remove(path);
            }
            if project_path != app.project_path {
                return Some(iced::Task::none());
            }
            app.session_previews = previews;
            app.archived_session_ids = archived_session_ids;
            match result {
                Ok(sessions) => {
                    if let Some(path) = project_path {
                        let existing = app.project_sessions.get(&path).cloned();
                        app.project_sessions
                            .insert(path, merge_project_sessions(existing, sessions.clone()));
                    }
                    app.sessions = sessions;
                }
                Err(e) => {
                    app.push_notification(format!("Failed to load sessions: {}", e));
                }
            }
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectInfoLoaded(res) => {
            if let Ok(loaded) = res {
                if app.project_path.as_deref() != Some(loaded.project_path.as_str()) {
                    return Some(iced::Task::none());
                }
                let info = loaded.info;
                let project_id = info.id;
                let icon = info
                    .icon
                    .as_ref()
                    .and_then(|x| x.override_icon.clone())
                    .filter(|x| !x.trim().is_empty());
                let icon_color = info
                    .icon
                    .as_ref()
                    .and_then(|x| x.color.clone())
                    .filter(|x| !x.trim().is_empty());
                let start_command = info
                    .commands
                    .as_ref()
                    .and_then(|x| x.start.clone())
                    .filter(|x| !x.trim().is_empty());
                if let Some(path) = app.project_path.clone()
                    && let Some(meta) = app.recent_projects_meta.iter_mut().find(|m| m.path == path)
                    && (icon.is_some() || icon_color.is_some() || start_command.is_some())
                {
                    if icon.is_some() {
                        meta.icon = icon;
                    }
                    if icon_color.is_some() {
                        meta.icon_color = icon_color;
                    }
                    if start_command.is_some() {
                        meta.worktree_start_command = start_command;
                    }
                    save_recent_projects_meta_background(app.recent_projects_meta.clone());
                }
                app.project_updated_at_ms =
                    if info.time.updated > 0 { Some(info.time.updated) } else { None };
                if loaded.current_branch.is_some() {
                    app.selected_branch = loaded.current_branch;
                }
                app.project_is_git_repo = matches!(info.vcs, Some(vw_shared::project::Vcs::Git));
                if !app.project_is_git_repo {
                    app.task_board_worktree_snapshot = None;
                    app.task_board_worktree_snapshot_loading = false;
                    app.task_board_last_worktree_snapshot_at_ms = 0;
                    app.task_board_worktree_maintenance_in_flight = false;
                    app.task_board_worktree_manual_action_kind = None;
                    app.task_board_worktree_manual_confirm_kind = None;
                    if app.task_board_executor_running {
                        app.task_board_executor_running = false;
                    }
                }
                app.project_id = Some(project_id);
            }
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectSessionsLoaded(project_path, res) => {
            app.project_sessions_loading.remove(&project_path);
            match res {
                Ok(sessions) => {
                    let existing = app.project_sessions.get(&project_path).cloned();
                    let updated = merge_project_sessions(existing, sessions);
                    app.project_sessions.insert(project_path.clone(), updated);
                    app.project_session_load_counts.insert(project_path.clone(), 10);
                }
                Err(e) => {
                    app.error_message = Some(format!("Failed to load project sessions: {}", e));
                }
            }
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectSessionListScrollChanged {
            project_path,
            has_vertical_scrollbar,
        } => {
            app.project_session_has_vertical_scrollbar.insert(project_path, has_vertical_scrollbar);
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectLoadMoreSessions(project_path) => {
            let current_count =
                app.project_session_load_counts.get(&project_path).copied().unwrap_or(10);
            app.project_session_load_counts.insert(project_path.clone(), current_count + 5);
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectCreateSession(project_path) => {
            if app.new_session_creating {
                return Some(iced::Task::none());
            }
            let project_switch_task = if app.project_path.as_deref() == Some(project_path.as_str())
            {
                iced::Task::none()
            } else {
                app.open_project_and_index(project_path.clone())
            };
            let project_worktree_enabled =
                app.project_worktree_enabled.get(&project_path).copied().unwrap_or(false);
            if !project_worktree_enabled {
                app.new_session_creating = true;
                reset_new_session_picker_state(app);
                let project_path_clone = project_path.clone();
                return Some(iced::Task::batch(vec![
                    project_switch_task,
                    iced::Task::perform(
                        async move { create_gateway_session_in_directory(project_path_clone).await },
                        |res| match res {
                            Ok(info) => Message::Project(ProjectMessage::SessionCreated(info)),
                            Err(error) => {
                                Message::Project(ProjectMessage::SessionCreateFailed(error))
                            }
                        },
                    ),
                ]));
            }
            app.new_session_picker_project = Some(project_path.clone());
            app.new_session_picker_options.clear();
            app.new_session_worktree_name.clear();
            clear_new_session_picker_messages(app);
            let project_path_clone = project_path.clone();
            Some(iced::Task::batch(vec![
                project_switch_task,
                iced::Task::perform(
                    async move { load_project_worktree_picker_options(&project_path_clone).await },
                    move |res| {
                        Message::Project(ProjectMessage::ProjectCreateSessionPickerLoaded {
                            project_path,
                            options: res,
                        })
                    },
                ),
            ]))
        }
        ProjectMessage::SessionCreateFailed(error) => {
            app.new_session_creating = false;
            Some(app.show_error_toast(format!("创建新会话失败：{error}")))
        }
        ProjectMessage::ProjectBranchesLoaded { project_path, selected_branch, branches } => {
            if app.project_path.as_deref() != Some(project_path.as_str()) {
                return Some(iced::Task::none());
            }
            app.selected_branch = selected_branch;
            app.branches = branches;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectLoadSessions(project_path) => {
            if app.project_sessions_loading.contains(&project_path) {
                return Some(iced::Task::none());
            }
            app.project_sessions_loading.insert(project_path.clone());
            let project_path_clone = project_path.clone();
            Some(iced::Task::perform(
                async move {
                    let client = crate::app::gateway_client().map_err(|err| err.to_string())?;
                    client
                        .session_list::<Vec<vw_shared::session::info::Info>>(Some(
                            &project_path_clone,
                        ))
                        .await
                },
                |res| Message::Project(ProjectMessage::ProjectSessionsLoaded(project_path, res)),
            ))
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "open_tests.rs"]
mod open_tests;

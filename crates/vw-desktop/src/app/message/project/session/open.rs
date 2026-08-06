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
use crate::app::{
    App, Message, models,
    state::{
        QuickMessageHandoffState, default_recent_project_session_auto_refresh,
        default_recent_project_session_refresh_interval_seconds,
    },
};

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

            #[cfg(not(target_arch = "wasm32"))]
            let mut loaded_from_ui_store_has_missing_message_ids = false;
            #[cfg(not(target_arch = "wasm32"))]
            let loaded_from_ui_store = if app.chat.is_empty() && !app.session_is_requesting(&id) {
                if let Some(mut session) =
                    crate::app::session_gateway::gateway_load_session_any(&id)
                {
                    if session.messages.is_empty() {
                        false
                    } else {
                        let missing_message_model_count = session
                            .messages
                            .iter()
                            .filter(|message| message.model.is_none())
                            .count();
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
                        if backfilled_message_model {
                            let _ = crate::app::session_gateway::gateway_save_session(
                                &session,
                                Some(&resolved_path),
                            );
                        }
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
                        loaded_from_ui_store_has_missing_message_ids =
                            app.chat_message_ids.iter().any(Option::is_none);
                        app.store_session_chat_snapshot(
                            id.clone(),
                            shared_chat,
                            app.chat_message_ids.clone(),
                        );
                        app.usage = usage;
                        app.active_session_view_state.updated_ms = session.updated_ms;
                        app.replace_active_session_steps(session.steps.clone());
                        app.replace_active_session_calls(session.calls.clone());
                        if !loaded_from_ui_store_has_missing_message_ids
                            && app.restore_session_model_from_steps(&id, &session.steps)
                        {
                            app.sync_active_session_preferences();
                        }
                        true
                    }
                } else {
                    false
                }
            } else {
                false
            };
            #[cfg(target_arch = "wasm32")]
            let loaded_from_ui_store_has_missing_message_ids = false;
            #[cfg(target_arch = "wasm32")]
            let loaded_from_ui_store = false;

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
            let should_load_remote = (!loaded_from_ui_store
                || loaded_from_ui_store_has_missing_message_ids)
                && !has_requesting_cache;
            tracing::info!(
                target: "vw_desktop",
                session_id = %id,
                project_path = %resolved_path,
                loaded_from_ui_store,
                loaded_from_ui_store_has_missing_message_ids,
                has_requesting_cache,
                should_load_remote,
                current_messages = app.chat.len(),
                "resolved session history bootstrap strategy"
            );

            let load_task = if should_load_remote {
                load_session_messages_task(resolved_path, id)
            } else {
                iced::Task::none()
            };
            Some(iced::Task::batch(vec![
                open_task,
                initial_prewarm_task,
                load_task,
                iced::Task::done(Message::Chat(
                    crate::app::message::ChatMessage::LoadInputPanelTodos,
                )),
            ]))
        }
        ProjectMessage::RecentHovered(id) => {
            app.hovered_recent_project = id.clone();
            if let Some(path) = id
                && !app.project_sessions.contains_key(&path)
                && !app.project_sessions_loading.contains(&path)
            {
                app.project_sessions_loading.insert(path.clone());
                app.project_sessions_last_refresh_at.insert(path.clone(), web_time::Instant::now());
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
        ProjectMessage::SessionBootstrapLoaded { result, previews, archived_session_ids } => {
            app.session_previews = previews;
            app.archived_session_ids = archived_session_ids;
            match result {
                Ok(sessions) => {
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
            let project_switch_task =
                if app.project_path.as_deref() == Some(project_path.as_str()) {
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
            if matches!(
                app.quick_message_handoff,
                Some(QuickMessageHandoffState::WaitingForSession { .. })
            ) {
                app.quick_message_handoff = None;
            }
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
        ProjectMessage::ProjectSessionsRefreshTick => {
            let project_path = if app.show_settings {
                app.project_path.clone()
            } else {
                app.hovered_recent_project.clone()
            };

            if let Some(path) = project_path {
                let (auto_refresh, interval_seconds) = app
                    .recent_projects_meta
                    .iter()
                    .find(|meta| meta.path == path)
                    .map(|meta| {
                        (
                            meta.session_auto_refresh,
                            meta.session_refresh_interval_seconds.clamp(1, 3600),
                        )
                    })
                    .unwrap_or((
                        default_recent_project_session_auto_refresh(),
                        default_recent_project_session_refresh_interval_seconds(),
                    ));
                if !auto_refresh {
                    return Some(iced::Task::none());
                }
                let refresh_due = app
                    .project_sessions_last_refresh_at
                    .get(&path)
                    .map(|last| last.elapsed().as_secs() >= interval_seconds)
                    .unwrap_or(true);
                if !refresh_due {
                    return Some(iced::Task::none());
                }
                return Some(iced::Task::perform(async move { path }, |p| {
                    Message::Project(ProjectMessage::ProjectLoadSessions(p))
                }));
            }
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectLoadSessions(project_path) => {
            if app.project_sessions_loading.contains(&project_path) {
                return Some(iced::Task::none());
            }
            app.project_sessions_loading.insert(project_path.clone());
            app.project_sessions_last_refresh_at
                .insert(project_path.clone(), web_time::Instant::now());
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

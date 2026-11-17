//! 处理项目会话生命周期，负责加载历史、重置视图和切换活动会话。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use crate::app::message::project::PreparedSessionOpenSource;
use crate::app::message::project::ProjectMessage;
use crate::app::message::project::helpers::{
    find_session_project_path, load_prepared_session_open_from_gateway_task, now_ms,
    prepare_session_ui_chunks_task, prepare_session_ui_task,
};
use crate::app::{App, Message};
use web_time::Instant;

/// 幂等登记外部创建的会话，不触碰当前活动会话及聊天运行态。
pub(crate) fn cache_external_session(app: &mut App, info: session_info::Info) {
    cache_project_session(app, info.directory.clone(), info);
}

pub(super) fn cache_project_session(app: &mut App, project_path: String, info: session_info::Info) {
    desktop_workspace::project_message::session::lifecycle::cache_project_session(
        &mut app.session_state.sessions,
        &mut app.workspace_state.project_sessions,
        project_path,
        info,
    );
}

use desktop_workspace::project_message::session::history_open::{
    HistoryDecision, HistoryOpen, HistorySource,
};
pub(crate) use desktop_workspace::project_message::session::lifecycle::latest_assistant_model_label;

#[cfg(test)]
pub(crate) fn loaded_chat_from_gateway_messages(
    messages: Vec<message_types::WithParts>,
) -> (Vec<models::ChatMessage>, Vec<Option<String>>) {
    desktop_workspace::project_message::session::lifecycle::loaded_chat_from_gateway_messages(
        messages,
        crate::app::components::chat_panel::message_view::COMPACTION_COMPLETED_TEXT,
    )
}

pub(super) fn history_state(
    app: &mut App,
) -> desktop_session::session::history_commit::HistoryState<
    '_,
    crate::app::components::chat_panel::trajectory::TrajectoryProjection,
> {
    desktop_session::session::history_commit::HistoryState {
        chat: &mut app.chat_view.chat,
        ids: &mut app.chat_view.chat_message_ids,
        usage: &mut app.chat_view.usage,
        view: &mut app.chat_view.active_session_view_state,
        timeline: &mut app.session_state.paged_chat_timeline,
    }
}

fn apply_prepared_session_open(
    app: &mut App,
    prepared: crate::app::session::PreparedSessionOpen,
    open_generation: u64,
) {
    let committed = history_state(app).install(prepared, open_generation);
    let session_id = committed.session_id.clone();
    if let Some(seq_base) = committed.seq_base {
        app.get_session_runtime_mut(&session_id).stream_save.seq_base = seq_base;
    }
    app.chat_view.chat_scroll_id = iced::widget::Id::unique();
    let restored_session_model_from_steps =
        app.restore_session_model_from_steps(&session_id, &committed.steps);
    let desktop_session::session::history_commit::PreparedViewCompletion {
        session_model,
        base_ui_chunk,
    } = committed.apply_view(
        &mut app.chat_view.active_session_view_state,
        crate::app::components::chat_panel::trajectory::project_trajectory,
    );
    let restored_session_model_from_history = session_model
        .as_deref()
        .is_some_and(|model| app.restore_session_model_from_history(&session_id, model));
    if restored_session_model_from_steps || restored_session_model_from_history {
        app.sync_active_session_preferences();
    }

    history_state(app).begin_history_ui();
    app.invalidate_chat_ui_state();
    let base_installed = base_ui_chunk.is_some();
    if let Some(base_ui_chunk) = base_ui_chunk {
        let chunk_start_idx = base_ui_chunk.chunk_start_idx;
        app.pin_chat_ui_chunk(Some(chunk_start_idx));
        app.apply_prepared_chat_ui_phase(crate::app::session::PreparedChatUiPhase::Base(
            base_ui_chunk,
        ));
    } else {
        app.pin_chat_ui_chunk(None);
    }
    history_state(app).finish_history_ui(base_installed);
}

/// handle 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(crate) fn handle(app: &mut App, message: ProjectMessage) -> Option<iced::Task<Message>> {
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
        message @ (ProjectMessage::SessionRightClicked(..)
        | ProjectMessage::SessionMenuClose
        | ProjectMessage::SessionTreeToggled(_)
        | ProjectMessage::SessionTitleClicked(_)
        | ProjectMessage::SessionRenamePressed(_)
        | ProjectMessage::SessionRenameChanged(_)
        | ProjectMessage::SessionRenameCancel
        | ProjectMessage::SessionRenameSave) => {
            use desktop_workspace::project_message::session::interaction::{self, Event, State};
            let event = match message {
                ProjectMessage::SessionRightClicked(id, x, y) => Event::RightClick(id, x, y),
                ProjectMessage::SessionMenuClose => Event::Close,
                ProjectMessage::SessionTreeToggled(id) => Event::Toggle(id),
                ProjectMessage::SessionTitleClicked(id) => {
                    let now = web_time::Instant::now();
                    let elapsed = app.session_state.session_title_last_click
                        .map(|last| now.duration_since(last).as_millis());
                    app.session_state.session_title_last_click = Some(now);
                    Event::Title(id, elapsed)
                }
                ProjectMessage::SessionRenamePressed(id) => Event::Rename(id),
                ProjectMessage::SessionRenameChanged(value) => Event::Change(value),
                ProjectMessage::SessionRenameCancel => Event::Cancel,
                ProjectMessage::SessionRenameSave => Event::Save,
                _ => unreachable!(),
            };
            let rename = interaction::update(
                &mut State {
                    menu: &mut app.workspace_state.session_menu_id,
                    anchor: &mut app.workspace_state.session_menu_anchor,
                    actions: &mut app.popover_visibility.session_actions,
                    project_menu: &mut app.navigation_mode.work_current_project_menu,
                    expanded: &mut app.workspace_state.session_tree_expanded,
                    rename_id: &mut app.session_state.session_rename_id,
                    rename_value: &mut app.session_state.session_rename_value,
                    sessions: &mut app.session_state.sessions,
                    projects: &mut app.workspace_state.project_sessions,
                },
                event,
            );
            let Some(interaction::Rename { id, title }) = rename else {
                return Some(iced::Task::none());
            };
            app.refresh_task_pet_session_title(&id, &title);

            let project_path = find_session_project_path(app, &id);
            Some(iced::Task::perform(
                async move {
                    let client = crate::app::gateway_client().map_err(|err| err.to_string())?;
                    desktop_session::session::loading::persist_session_mutation(
                        &client,
                        project_path.as_deref(),
                        desktop_session::session::loading::SessionMutation::Rename { id, title },
                    )
                    .await
                },
                |_| Message::None,
            ))
        }
        ProjectMessage::SessionPinPressed(project_id, id, pinned) => {
            Some(app.set_session_pin(project_id, id, pinned))
        }
        ProjectMessage::SessionPinSaved(key, result) => Some(app.apply_session_pin(key, result)),
        ProjectMessage::SessionArchivePressed(id) => {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(runtime) = app.session_state.session_runtime_states.get_mut(&id) {
                for panel in runtime.mcp_app_panels.drain(..) {
                    panel.cancel();
                }
            }
            app.workspace_state.session_menu_id = None;
            app.workspace_state.session_menu_anchor = None;
            app.popover_visibility.session_actions = false;

            desktop_workspace::project_message::session::lifecycle::SessionMembership {
                sessions: &mut app.session_state.sessions,
                projects: &mut app.workspace_state.project_sessions,
            }
            .archive(&id, now_ms);

            let mut switch_task = iced::Task::none();
            if app.session_state.active_session_id.as_ref() == Some(&id) {
                let next =
                    desktop_workspace::project_message::session::lifecycle::SessionMembership {
                        sessions: &mut app.session_state.sessions,
                        projects: &mut app.workspace_state.project_sessions,
                    }
                    .replacement(Some(&id));

                if let Some(next_id) = next {
                    app.prepare_historical_session_model(&next_id);
                    app.session_state.active_session_id = Some(next_id.clone());
                    app.mark_active_session_viewed();
                    app.sync_active_session_preferences();
                    let load_permissions = app.load_session_full_access(&next_id);
                    let load_messages = iced::Task::perform(
                        async move {
                            let client = match crate::app::gateway_client() {
                                Ok(client) => client,
                                Err(err) => return Err(err),
                            };
                            desktop_session::session::loading::load_replacement_history(
                                &client,
                                next_id,
                                desktop_session::session::loading::ReplacementHistory::Archived,
                            )
                            .await
                        },
                        |res| Message::Project(ProjectMessage::SessionMessagesLoaded(res)),
                    );
                    switch_task = iced::Task::batch([load_messages, load_permissions]);
                } else {
                    app.session_state.active_session_id = None;
                    history_state(app).close();
                    app.invalidate_chat_ui_state();
                }
            }

            let project_path = find_session_project_path(app, &id);
            Some(iced::Task::batch(vec![
                iced::Task::perform(
                    async move {
                        let client = match crate::app::gateway_client() {
                            Ok(client) => client,
                            Err(err) => return Err(err.to_string()),
                        };
                        desktop_session::session::loading::persist_session_mutation(
                            &client,
                            project_path.as_deref(),
                            desktop_session::session::loading::SessionMutation::Archive { id },
                        )
                        .await
                    },
                    |_| Message::None,
                ),
                switch_task,
            ]))
        }
        ProjectMessage::SessionDeletePressed(id) => {
            app.workspace_state.session_menu_id = None;
            app.workspace_state.session_menu_anchor = None;
            app.popover_visibility.session_actions = false;
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(runtime) = app.session_state.session_runtime_states.get(&id) {
                for panel in &runtime.mcp_app_panels {
                    panel.cancel();
                }
            }
            desktop_session::session::cache::remove_session(
                &id,
                &mut app.session_state.session_runtime_states,
                &mut app.session_state.session_view_cache,
                &mut app.session_state.session_chat_cache,
                &mut app.session_state.session_chat_message_id_cache,
                &mut app.session_state.session_history_reset_boundaries,
            );

            let project_path = find_session_project_path(app, &id);

            desktop_workspace::project_message::session::lifecycle::SessionMembership {
                sessions: &mut app.session_state.sessions,
                projects: &mut app.workspace_state.project_sessions,
            }
            .delete(&id);

            let mut switch_task = iced::Task::none();

            if app.session_state.active_session_id.as_ref() == Some(&id) {
                let next =
                    desktop_workspace::project_message::session::lifecycle::SessionMembership {
                        sessions: &mut app.session_state.sessions,
                        projects: &mut app.workspace_state.project_sessions,
                    }
                    .replacement(None);

                if let Some(next_id) = next {
                    app.prepare_historical_session_model(&next_id);
                    app.session_state.active_session_id = Some(next_id.clone());
                    app.mark_active_session_viewed();
                    app.sync_active_session_preferences();
                    let load_permissions = app.load_session_full_access(&next_id);
                    let load_messages = iced::Task::perform(
                        async move {
                            let client = match crate::app::gateway_client() {
                                Ok(client) => client,
                                Err(err) => return Err(err),
                            };
                            desktop_session::session::loading::load_replacement_history(
                                &client,
                                next_id,
                                desktop_session::session::loading::ReplacementHistory::Deleted,
                            )
                            .await
                        },
                        |res| Message::Project(ProjectMessage::SessionMessagesLoaded(res)),
                    );
                    switch_task = iced::Task::batch([load_messages, load_permissions]);
                } else {
                    app.session_state.active_session_id = None;
                    history_state(app).close();
                    app.invalidate_chat_ui_state();
                }
            }

            Some(iced::Task::batch(vec![
                iced::Task::perform(
                    async move {
                        let client = crate::app::gateway_client().map_err(|err| err.to_string())?;
                        desktop_session::session::loading::persist_session_mutation(
                            &client,
                            project_path.as_deref(),
                            desktop_session::session::loading::SessionMutation::Delete { id },
                        )
                        .await
                    },
                    |_| Message::None,
                ),
                switch_task,
            ]))
        }
        ProjectMessage::SessionCopyPressed(id) => {
            let project_path = find_session_project_path(app, &id);
            Some(iced::Task::perform(
                async move {
                    let client = crate::app::gateway_client().map_err(|err| err.to_string())?;
                    gateway_session::GatewaySessionClient::new(client.transport().clone())
                        .session_fork::<session_info::Info>(&id, project_path.as_deref(), &None)
                        .await
                },
                |res| match res {
                    Ok(info) => Message::Project(ProjectMessage::SessionCopied(info)),
                    Err(e) => {
                        eprintln!("Fork failed: {}", e);
                        Message::None
                    }
                },
            ))
        }
        ProjectMessage::SessionCopied(info) => {
            desktop_workspace::project_message::session::lifecycle::SessionMembership {
                sessions: &mut app.session_state.sessions,
                projects: &mut app.workspace_state.project_sessions,
            }
            .insert(app.navigation_tabs.project_path.as_deref(), info, false);
            Some(iced::Task::none())
        }
        ProjectMessage::SessionCreated(info) => {
            app.cache_active_session_chat();
            let activate =
                desktop_workspace::project_message::session::lifecycle::SessionMembership {
                    sessions: &mut app.session_state.sessions,
                    projects: &mut app.workspace_state.project_sessions,
                }
                .insert(
                    app.navigation_tabs.project_path.as_deref(),
                    info.clone(),
                    true,
                );
            if !activate {
                return Some(iced::Task::none());
            }

            app.workspace_state.new_session_creating = false;
            app.workspace_state.new_session_last_directory = Some(info.directory.clone());
            let save_last_directory_task = app.save_desktop_preference(
                "new_session_last_directory",
                serde_json::Value::String(info.directory.clone()),
            );
            app.session_state.active_session_id = Some(info.id.clone());
            app.chat_view.chat_scroll_id = iced::widget::Id::unique();
            app.mark_active_session_viewed();
            history_state(app).blank_draft();
            app.store_session_chat_snapshot(
                info.id.clone(),
                crate::app::session::shared_chat_messages(app.chat_view.chat.clone()),
                app.chat_view.chat_message_ids.clone(),
            );
            app.invalidate_chat_ui_state();
            app.sync_active_session_preferences();
            Some(iced::Task::batch([
                save_last_directory_task,
                app.load_session_full_access(&info.id),
            ]))
        }
        ProjectMessage::SessionOpenPrepared {
            target_session_id,
            project_path,
            open_generation,
            source,
            result,
        } => {
            let history_open = HistoryOpen {
                active_session: app.session_state.active_session_id.as_deref(),
                target_session: &target_session_id,
                current_generation: app.session_state.session_view_cache
                    .is_current_open(&target_session_id, open_generation),
                source: if source == PreparedSessionOpenSource::UiStore {
                    HistorySource::UiStore
                } else {
                    HistorySource::Remote
                },
                current_ids: &app.chat_view.chat_message_ids,
                reset_boundary: app.session_history_reset_boundary(&target_session_id),
                current_signature: crate::app::session::session_content_signature(
                    &app.chat_view.chat,
                    &app.chat_view.chat_message_ids,
                ),
            };
            let acceptance = history_open
                .accept(result.as_ref().ok().map(|prepared| prepared.session_id.as_str()));
            let content_decision = result.as_ref().ok().map(|prepared| {
                history_open.content(&prepared.message_ids, prepared.content_signature)
            });
            let prepared = match result {
                Ok(prepared) if acceptance == HistoryDecision::Replace => prepared,
                Ok(_) => {
                    tracing::info!(
                        target: "vw_desktop",
                        session_id = %target_session_id,
                        open_generation,
                        branch = "stale_open_result",
                        "discarded prepared session open because the active request changed"
                    );
                    return Some(iced::Task::none());
                }
                Err(error) if acceptance == HistoryDecision::LoadRemote => {
                    tracing::warn!(
                        target: "vw_desktop",
                        session_id = %target_session_id,
                        error = %error,
                        "failed to prepare session UI store history; falling back to remote history"
                    );
                    return Some(load_prepared_session_open_from_gateway_task(
                        project_path,
                        target_session_id,
                        open_generation,
                        app.general_settings.dialogue_flow_show_reasoning_summary,
                    ));
                }
                Err(error) if acceptance == HistoryDecision::ReportRemoteError => {
                    tracing::warn!(
                        target: "vw_desktop",
                        session_id = %target_session_id,
                        error = %error,
                        "failed to prepare remote session history"
                    );
                    if app.chat_view.chat.is_empty() {
                        app.input_feedback.error_message =
                            Some(format!("Failed to load session messages: {error}"));
                    }
                    app.chat_view.active_session_view_state.ui_preparing = false;
                    return Some(iced::Task::none());
                }
                Err(_) => return Some(iced::Task::none()),
            };

            if let Some(binding) = prepared.agent_binding.as_ref() {
                app.get_session_runtime_mut(&target_session_id).restore_open_binding(binding);
            }
            if let Some(full_access) = prepared.full_access {
                app.restore_session_full_access(&target_session_id, full_access);
            }
            if content_decision == Some(HistoryDecision::KeepCurrent) {
                tracing::warn!(
                    target: "vw_desktop",
                    session_id = %target_session_id,
                    remote_messages = prepared.message_ids.len(),
                    retained_messages = app.chat_view.chat_message_ids.len(),
                    "preserved prepared session history because remote history was incomplete"
                );
                app.chat_view.active_session_view_state.ui_preparing = false;
                return Some(iced::Task::none());
            }

            if content_decision == Some(HistoryDecision::RefreshMetadata) {
                history_state(app).refresh_metadata(
                    prepared,
                    crate::app::components::chat_panel::trajectory::project_trajectory,
                );
                if let Some(summary) = app.session_state.session_view_cache.record_open_commit(
                    &target_session_id,
                    open_generation,
                    crate::app::session::view_cache::SessionOpenUiCommit::SameContentRefresh,
                    app.chat_view.chat.len(),
                    web_time::Duration::ZERO,
                    true,
                ) {
                    summary.emit();
                }
                let trajectory = crate::app::message::chat::refresh_active_trajectory_raw_task(
                    app,
                    &target_session_id,
                );
                let queued = crate::app::message::chat::stream::start_next(app, &target_session_id);
                return Some(iced::Task::batch([trajectory, queued]));
            }

            let commit_started_at = Instant::now();
            let message_count = prepared.messages.len();
            apply_prepared_session_open(app, prepared, open_generation);
            let trajectory_refresh_task = iced::Task::batch([
                crate::app::message::chat::refresh_active_trajectory_raw_task(
                    app,
                    &target_session_id,
                ),
                iced::Task::done(crate::app::Message::Chat(
                    crate::app::message::ChatMessage::AcpSourceRefresh,
                )),
            ]);
            let finalize = source == PreparedSessionOpenSource::RemoteHistory;
            if let Some(summary) = app.session_state.session_view_cache.record_open_commit(
                &target_session_id,
                open_generation,
                crate::app::session::view_cache::SessionOpenUiCommit::PreparedHistory,
                message_count,
                commit_started_at.elapsed(),
                finalize,
            ) {
                summary.emit();
            }
            if source == PreparedSessionOpenSource::UiStore {
                return Some(iced::Task::batch([
                    load_prepared_session_open_from_gateway_task(
                        project_path,
                        target_session_id,
                        open_generation,
                        app.general_settings.dialogue_flow_show_reasoning_summary,
                    ),
                    trajectory_refresh_task,
                ]));
            }
            let queued = crate::app::message::chat::stream::start_next(app, &target_session_id);
            Some(iced::Task::batch([trajectory_refresh_task, queued]))
        }
        ProjectMessage::SessionHistoryPageLoaded {
            session_id,
            open_generation,
            requested_cursor,
            result,
        } => {
            let current_generation =
                app.session_state.session_view_cache.is_current_open(&session_id, open_generation);
            let active_session = app.session_state.active_session_id.clone();
            let commit = history_state(app).prepend(
                active_session.as_deref(),
                &session_id,
                current_generation,
                open_generation,
                &requested_cursor,
                result,
            );
            let Some(desktop_session::session::history_commit::PrependCommit {
                inserted,
                seq_base,
            }) = commit
            else {
                return Some(iced::Task::none());
            };
            app.get_session_runtime_mut(&session_id).stream_save.seq_base = seq_base;
            app.chat_view.chat_virtualizer.prepend_estimated_heights(&vec![120.0; inserted]);
            app.chat_view.chat_render_cache.clear();
            app.chat_view.chat_visible_text_cache.clear();
            app.chat_view.chat_copy_hash_cache.clear();
            app.chat_view.chat_message_measured_heights.clear();
            app.sync_chat_virtualizer_compat_fields();
            Some(crate::app::message::project::prepare_session_ui_chunks_task(
                session_id,
                app.active_shared_chat_messages(),
                vec![0],
                None,
                app.general_settings.dialogue_flow_show_reasoning_summary,
            ))
        }
        ProjectMessage::SessionMessagesLoaded(res) => {
            match res {
                Ok((id, msgs, usage)) => {
                    let loaded_session_model = latest_assistant_model_label(&msgs);
                    tracing::info!(
                        target: "vw_desktop",
                        session_id = %id,
                        message_count = msgs.len(),
                        active_session_matches = app.session_state.active_session_id.as_ref() == Some(&id),
                        "received session messages load result"
                    );
                    if app.session_state.active_session_id.as_ref() == Some(&id) {
                        let restored_session_model =
                            loaded_session_model.as_deref().is_some_and(|model| {
                                app.restore_session_model_from_history(&id, model)
                            });
                        let local_messages = app
                            .cached_chat_messages(&id)
                            .map(|chat| chat.iter().cloned().collect::<Vec<_>>());
                        let local_message_ids = app.cached_chat_message_ids(&id);
                        let reset_boundary =
                            app.session_history_reset_boundary(&id).map(|ids| ids.to_vec());
                        let commit = history_state(app).loaded(msgs, local_messages, local_message_ids, reset_boundary, usage,
                            crate::app::components::chat_panel::message_view::COMPACTION_COMPLETED_TEXT,
                            crate::app::components::chat_panel::trajectory::project_trajectory);
                        if let Some((gateway_messages, cached_messages)) = commit.shorter_gateway {
                            tracing::warn!(target: "vw_desktop", session_id = %id, gateway_messages, cached_messages,
                                "preserved cached session history because gateway history was shorter");
                        }
                        let shared_chat = commit.shared;
                        app.store_session_chat_snapshot(
                            id.clone(),
                            shared_chat.clone(),
                            app.chat_view.chat_message_ids.clone(),
                        );
                        if restored_session_model {
                            app.sync_active_session_preferences();
                        }
                        if commit.same_visible {
                            return Some(
                                crate::app::message::chat::refresh_active_trajectory_raw_task(
                                    app, &id,
                                ),
                            );
                        }
                        app.chat_view.chat_scroll_id = iced::widget::Id::unique();
                        app.invalidate_chat_ui_state();
                        if app.chat_view.chat.is_empty() {
                            tracing::warn!(
                                target: "vw_desktop",
                                session_id = %id,
                                "session message load completed but chat remained empty"
                            );
                        } else {
                            tracing::info!(
                                target: "vw_desktop",
                                session_id = %id,
                                chat_messages = app.chat_view.chat.len(),
                                "applied loaded session messages to active chat"
                            );
                        }

                        if !(history_state(app)).prepare_loaded_ui() {
                            app.pin_chat_ui_chunk(None);
                        } else {
                            let base_chunk_start = app.preferred_base_chat_ui_chunk_start();
                            app.pin_chat_ui_chunk(Some(base_chunk_start));
                            app.mark_chat_ui_chunks_preparing(&[base_chunk_start]);
                            return Some(prepare_session_ui_task(
                                id,
                                shared_chat,
                                base_chunk_start,
                                true,
                                app.general_settings.dialogue_flow_show_reasoning_summary,
                            ));
                        }
                    } else {
                        tracing::warn!(
                            target: "vw_desktop",
                            session_id = %id,
                            active_session_id = app.session_state.active_session_id.as_deref().unwrap_or("<none>"),
                            "discarded session messages because active session changed"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        target: "vw_desktop",
                        active_session_id = app.session_state.active_session_id.as_deref().unwrap_or("<none>"),
                        error = %e,
                        "failed to load session messages"
                    );
                    if let Some(id) = app.session_state.active_session_id.clone()
                        && let Some(local) = app.cached_chat_messages(&id)
                        && !local.is_empty()
                    {
                        let local_messages = local.iter().cloned().collect::<Vec<_>>();
                        let local_message_ids = app.cached_chat_message_ids(&id);
                        let shared_chat = history_state(app).restore_cached(
                            local_messages,
                            local_message_ids,
                            crate::app::components::chat_panel::trajectory::project_trajectory,
                        );
                        app.store_session_chat_snapshot(
                            id.clone(),
                            shared_chat.clone(),
                            app.chat_view.chat_message_ids.clone(),
                        );
                        app.invalidate_chat_ui_state();
                        let base_chunk_start = app.preferred_base_chat_ui_chunk_start();
                        app.pin_chat_ui_chunk(Some(base_chunk_start));
                        app.mark_chat_ui_chunks_preparing(&[base_chunk_start]);
                        return Some(prepare_session_ui_task(
                            id,
                            shared_chat,
                            base_chunk_start,
                            true,
                            app.general_settings.dialogue_flow_show_reasoning_summary,
                        ));
                    } else {
                        app.input_feedback.error_message = Some(format!("Failed to load session messages: {}", e));
                    }
                }
            }
            Some(iced::Task::none())
        }
        ProjectMessage::SessionUiPrepared { session_id, stream_generation, phase } => {
            let is_active_session = app.session_state.active_session_id.as_ref() == Some(&session_id);
            if let Some(generation) = stream_generation {
                let reprepare = app
                    .get_session_runtime_mut(&session_id)
                    .stream_tail_state
                    .finish_session_open_ui(generation, is_active_session);
                if let Some((message_idx, latest_generation)) = reprepare
                    && let Some(message) = app.chat_view.chat.get(message_idx).cloned()
                {
                    return Some(crate::app::message::project::prepare_stream_message_ui_task(
                        session_id,
                        message,
                        message_idx,
                        latest_generation,
                        app.general_settings.dialogue_flow_show_reasoning_summary,
                    ));
                }
                if app.get_session_runtime(&session_id).stream_tail_state.ui_generation()
                    != generation
                {
                    return Some(iced::Task::none());
                }
            }
            if is_active_session {
                let is_base_phase =
                    matches!(phase, crate::app::session::PreparedChatUiPhase::Base(_));
                app.apply_prepared_chat_ui_phase(phase);
                if is_base_phase {
                    history_state(app).finish_base_ui();
                    let (visible_start_idx, visible_end_idx) = app.visible_chat_message_window();
                    let nearby_chunk_starts =
                        app.pending_chat_ui_chunk_starts(visible_start_idx, visible_end_idx, true);
                    if !nearby_chunk_starts.is_empty() {
                        app.mark_chat_ui_chunks_preparing(&nearby_chunk_starts);
                        return Some(prepare_session_ui_chunks_task(
                            session_id,
                            app.active_shared_chat_messages(),
                            nearby_chunk_starts,
                            None,
                            app.general_settings.dialogue_flow_show_reasoning_summary,
                        ));
                    }
                }
            }
            Some(iced::Task::none())
        }
        _ => None,
    }
}

#[cfg(test)]
use crate::app::models;

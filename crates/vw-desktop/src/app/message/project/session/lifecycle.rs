//! 处理项目会话生命周期，负责加载历史、重置视图和切换活动会话。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use crate::app::message::project::PreparedSessionOpenSource;
use crate::app::message::project::ProjectMessage;
use crate::app::message::project::helpers::{
    find_session_project_path, load_prepared_session_open_from_gateway_task, now_ms,
    prepare_session_ui_chunks_task, prepare_session_ui_task,
};
use crate::app::{App, Message, models, state::ActiveSessionViewState};
use web_time::Instant;

/// 幂等登记外部创建的会话，不触碰当前活动会话及聊天运行态。
pub(crate) fn cache_external_session(app: &mut App, info: session_info::Info) {
    cache_project_session(app, info.directory.clone(), info);
}

pub(super) fn cache_project_session(
    app: &mut App,
    project_path: String,
    info: session_info::Info,
) {
    desktop_workspace::project_message::session::lifecycle::cache_project_session(
        &mut app.sessions, &mut app.project_sessions, project_path, info,
    );
}

pub(crate) use desktop_workspace::project_message::session::lifecycle::{
    latest_assistant_model_label,
    steps_from_gateway_messages,
};
use desktop_workspace::project_message::session::history_open::{HistoryOpen, HistorySource, HistoryDecision};

pub(crate) fn loaded_chat_from_gateway_messages(
    messages: Vec<message_types::WithParts>,
) -> (Vec<models::ChatMessage>, Vec<Option<String>>) {
    desktop_workspace::project_message::session::lifecycle::loaded_chat_from_gateway_messages(
        messages,
        crate::app::components::chat_panel::message_view::COMPACTION_COMPLETED_TEXT,
    )
}

fn apply_prepared_session_open(
    app: &mut App,
    prepared: crate::app::session::PreparedSessionOpen,
    open_generation: u64,
) {
    let crate::app::session::PreparedSessionOpen {
        session_id,
        agent_binding: _,
        full_access: _,
        messages,
        message_ids,
        usage,
        steps,
        calls,
        step_llm_raw,
        llm_raw_step_indices,
        updated_ms,
        content_signature: _,
        session_model,
        base_ui_chunk,
        timeline_page,
    } = prepared;

    if let Some(page) = timeline_page {
        app.get_session_runtime_mut(&session_id).stream_save.seq_base =
            page.entries.first().and_then(|entry| entry.seq_no).unwrap_or(0);
        let mut timeline = crate::app::session::chat_timeline::PagedChatTimeline::new(
            session_id.clone(),
            open_generation,
        );
        timeline.merge_initial_page(page);
        app.paged_chat_timeline = Some(timeline);
    }

    app.chat_scroll_id = iced::widget::Id::unique();
    app.chat = messages;
    app.chat_message_ids = message_ids;
    app.usage = usage;
    app.active_session_view_state.updated_ms = updated_ms;
    let restored_session_model_from_steps =
        app.restore_session_model_from_steps(&session_id, &steps);
    app.replace_active_session_steps(steps);
    app.replace_active_session_calls(calls);
    app.replace_active_session_step_llm_raw(step_llm_raw);
    app.replace_active_session_llm_raw_step_indices(llm_raw_step_indices);
    let restored_session_model_from_history = session_model
        .as_deref()
        .is_some_and(|model| app.restore_session_model_from_history(&session_id, model));
    if restored_session_model_from_steps || restored_session_model_from_history {
        app.sync_active_session_preferences();
    }

    app.active_session_view_state.ui_preparing = true;
    app.active_session_view_state.base_ready = false;
    app.invalidate_chat_ui_state();
    if let Some(base_ui_chunk) = base_ui_chunk {
        let chunk_start_idx = base_ui_chunk.chunk_start_idx;
        app.pin_chat_ui_chunk(Some(chunk_start_idx));
        app.apply_prepared_chat_ui_phase(crate::app::session::PreparedChatUiPhase::Base(
            base_ui_chunk,
        ));
        app.active_session_view_state.base_ready = true;
        app.rebuild_active_session_message_meta();
    } else {
        app.pin_chat_ui_chunk(None);
    }
    app.active_session_view_state.ui_preparing = false;
}

/// handle 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(crate) fn handle(app: &mut App, message: ProjectMessage) -> Option<iced::Task<Message>> {
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
        message @ (ProjectMessage::SessionRightClicked(..)
            | ProjectMessage::SessionMenuClose | ProjectMessage::SessionTreeToggled(_)
            | ProjectMessage::SessionTitleClicked(_) | ProjectMessage::SessionRenamePressed(_)
            | ProjectMessage::SessionRenameChanged(_) | ProjectMessage::SessionRenameCancel
            | ProjectMessage::SessionRenameSave) => {
            use desktop_workspace::project_message::session::interaction::{self, Event, State};
            let event = match message {
                ProjectMessage::SessionRightClicked(id, x, y) => Event::RightClick(id, x, y),
                ProjectMessage::SessionMenuClose => Event::Close,
                ProjectMessage::SessionTreeToggled(id) => Event::Toggle(id),
                ProjectMessage::SessionTitleClicked(id) => {
                    let now = web_time::Instant::now();
                    let elapsed = app.session_title_last_click.map(|last| now.duration_since(last).as_millis());
                    app.session_title_last_click = Some(now);
                    Event::Title(id, elapsed)
                },
                ProjectMessage::SessionRenamePressed(id) => Event::Rename(id),
                ProjectMessage::SessionRenameChanged(value) => Event::Change(value),
                ProjectMessage::SessionRenameCancel => Event::Cancel,
                ProjectMessage::SessionRenameSave => Event::Save,
                _ => unreachable!(),
            };
            let rename = interaction::update(&mut State {
                menu: &mut app.session_menu_id, anchor: &mut app.session_menu_anchor,
                actions: &mut app.popover_visibility.session_actions,
                project_menu: &mut app.navigation_mode.work_current_project_menu,
                expanded: &mut app.session_tree_expanded,
                rename_id: &mut app.session_rename_id, rename_value: &mut app.session_rename_value,
                sessions: &mut app.sessions, projects: &mut app.project_sessions,
            }, event);
            let Some(interaction::Rename { id, title }) = rename else { return Some(iced::Task::none()); };
            app.refresh_task_pet_session_title(&id, &title);

            let project_path = find_session_project_path(app, &id);
            Some(iced::Task::perform(
                async move {
                    let client = crate::app::gateway_client().map_err(|err| err.to_string())?;
                    gateway_session::GatewaySessionClient::new(client.transport().clone())
                        .session_update::<session_info::Info>(
                            &id,
                            project_path.as_deref(),
                            &vw_api_session::GatewaySessionPatchBody {
                                title: Some(title),
                                time: None,
                            },
                        )
                        .await
                },
                |_| Message::None,
            ))
        }
        ProjectMessage::SessionPinPressed(project_id, id, pinned) => Some(app.set_session_pin(project_id, id, pinned)),
        ProjectMessage::SessionPinSaved(key, result) => Some(app.apply_session_pin(key, result)),
        ProjectMessage::SessionArchivePressed(id) => {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(runtime) = app.session_runtime_states.get_mut(&id) {
                for panel in runtime.mcp_app_panels.drain(..) {
                    panel.cancel();
                }
            }
            app.session_menu_id = None;
            app.session_menu_anchor = None;
            app.popover_visibility.session_actions = false;

            if let Some(s) = app.sessions.iter_mut().find(|s| s.id == id) {
                s.time.archived = Some(now_ms());
            }
            for sessions in app.project_sessions.values_mut() {
                if let Some(s) = sessions.iter_mut().find(|s| s.id == id) {
                    s.time.archived = Some(now_ms());
                }
            }

            let mut switch_task = iced::Task::none();
            if app.active_session_id.as_ref() == Some(&id) {
                let next = app
                    .sessions
                    .iter()
                    .find(|s| s.time.archived.is_none() && s.id != id && s.is_user_navigable())
                    .map(|s| s.id.clone());

                if let Some(next_id) = next {
                    app.prepare_historical_session_model(&next_id);
                    app.active_session_id = Some(next_id.clone());
                    app.mark_active_session_viewed();
                    app.sync_active_session_preferences();
                    let load_permissions = app.load_session_full_access(&next_id);
                    let load_messages = iced::Task::perform(
                        async move {
                            let client = match crate::app::gateway_client() {
                                Ok(client) => client,
                                Err(err) => return Err(err),
                            };
                            let msgs = gateway_session::GatewaySessionClient::new(client.transport().clone())
                                .session_messages::<Vec<agent_message::WithParts>>(&next_id, None)
                                .await;
                            let info = gateway_session::GatewaySessionClient::new(client.transport().clone())
                                .session_get::<session_info::Info>(&next_id, None)
                                .await;
                            match (msgs, info) {
                                (Ok(msgs), Ok(_info)) => {
                                    let mut usage = models::TokenUsage::default();
                                    for m in &msgs {
                                        if let agent_message::Info::Assistant(a) = &m.info {
                                            usage.input_tokens += a.tokens.input;
                                            usage.output_tokens += a.tokens.output;
                                            usage.cached_tokens +=
                                                a.tokens.cache.read + a.tokens.cache.write;
                                            usage.reasoning_tokens += a.tokens.reasoning;
                                        }
                                    }
                                    Ok((next_id, msgs, usage))
                                }
                                (Err(e), _) => Err(e.to_string()),
                                (_, Err(e)) => Err(e.to_string()),
                            }
                        },
                        |res| Message::Project(ProjectMessage::SessionMessagesLoaded(res)),
                    );
                    switch_task = iced::Task::batch([load_messages, load_permissions]);
                } else {
                    app.active_session_id = None;
                    app.chat.clear();
                    app.usage = models::TokenUsage::default();
                    app.active_session_view_state = ActiveSessionViewState::default();
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
                        gateway_session::GatewaySessionClient::new(client.transport().clone())
                            .session_update::<session_info::Info>(
                                &id,
                                project_path.as_deref(),
                                &vw_api_session::GatewaySessionPatchBody {
                                    title: None,
                                    time: Some(vw_api_session::GatewaySessionPatchTime {
                                        archived: Some(runtime_time::now_ms()),
                                    }),
                                },
                            )
                            .await
                            .map_err(|err| err.to_string())
                    },
                    |_| Message::None,
                ),
                switch_task,
            ]))
        }
        ProjectMessage::SessionDeletePressed(id) => {
            app.session_menu_id = None;
            app.session_menu_anchor = None;
            app.popover_visibility.session_actions = false;
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(runtime) = app.session_runtime_states.get(&id) {
                for panel in &runtime.mcp_app_panels {
                    panel.cancel();
                }
            }
            app.session_runtime_states.remove(&id);
            app.session_view_cache.remove(&id);
            app.session_chat_cache.remove(&id);
            app.session_chat_message_id_cache.remove(&id);
            app.session_history_reset_boundaries.remove(&id);

            let project_path = find_session_project_path(app, &id);

            if let Some(pos) = app.sessions.iter().position(|s| s.id == id) {
                app.sessions.remove(pos);
            }
            for sessions in app.project_sessions.values_mut() {
                if let Some(pos) = sessions.iter().position(|s| s.id == id) {
                    sessions.remove(pos);
                }
            }

            let mut switch_task = iced::Task::none();

            if app.active_session_id.as_ref() == Some(&id) {
                let next = app
                    .sessions
                    .iter()
                    .find(|s| s.time.archived.is_none() && s.is_user_navigable())
                    .map(|s| s.id.clone());

                if let Some(next_id) = next {
                    app.prepare_historical_session_model(&next_id);
                    app.active_session_id = Some(next_id.clone());
                    app.mark_active_session_viewed();
                    app.sync_active_session_preferences();
                    let load_permissions = app.load_session_full_access(&next_id);
                    let load_messages = iced::Task::perform(
                        async move {
                            let client = match crate::app::gateway_client() {
                                Ok(client) => client,
                                Err(err) => return Err(err),
                            };
                            let msgs = gateway_session::GatewaySessionClient::new(client.transport().clone())
                                .session_messages::<Vec<agent_message::WithParts>>(&next_id, None)
                                .await;
                            match msgs {
                                Ok(msgs) => {
                                    let mut usage = models::TokenUsage::default();
                                    for m in &msgs {
                                        if let agent_message::Info::Assistant(a) = &m.info {
                                            usage.input_tokens += a.tokens.input;
                                            usage.output_tokens += a.tokens.output;
                                            usage.cached_tokens +=
                                                a.tokens.cache.read + a.tokens.cache.write;
                                            usage.reasoning_tokens += a.tokens.reasoning;
                                        }
                                    }
                                    Ok((next_id, msgs, usage))
                                }
                                Err(e) => Err(e.to_string()),
                            }
                        },
                        |res| Message::Project(ProjectMessage::SessionMessagesLoaded(res)),
                    );
                    switch_task = iced::Task::batch([load_messages, load_permissions]);
                } else {
                    app.active_session_id = None;
                    app.chat.clear();
                    app.usage = models::TokenUsage::default();
                    app.active_session_view_state = ActiveSessionViewState::default();
                    app.invalidate_chat_ui_state();
                }
            }

            Some(iced::Task::batch(vec![
                iced::Task::perform(
                    async move {
                        let client = crate::app::gateway_client().map_err(|err| err.to_string())?;
                        gateway_session::GatewaySessionClient::new(client.transport().clone()).session_delete(&id, project_path.as_deref()).await
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
                        .session_fork::<session_info::Info>(
                            &id,
                            project_path.as_deref(),
                            &None,
                        )
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
            if app.navigation_tabs.project_path.as_ref() == Some(&info.directory)
                && !app.sessions.iter().any(|s| s.id == info.id)
            {
                app.sessions.insert(0, info.clone());
            }
            let sessions_key = app.navigation_tabs.project_path.clone().unwrap_or_else(|| info.directory.clone());
            if let Some(list) = app.project_sessions.get_mut(&sessions_key) {
                if !list.iter().any(|s| s.id == info.id) {
                    list.insert(0, info.clone());
                }
            } else {
                app.project_sessions.insert(sessions_key, vec![info.clone()]);
            }
            Some(iced::Task::none())
        }
        ProjectMessage::SessionCreated(info) => {
            let is_discovered_child =
                info.parent_id.as_deref().is_some_and(|parent_id| !parent_id.trim().is_empty());
            app.cache_active_session_chat();
            if !app.sessions.iter().any(|s| s.id == info.id) {
                app.sessions.insert(0, info.clone());
            }
            let sessions_key = app.navigation_tabs.project_path.clone().unwrap_or_else(|| info.directory.clone());
            if let Some(list) = app.project_sessions.get_mut(&sessions_key) {
                if !list.iter().any(|s| s.id == info.id) {
                    list.insert(0, info.clone());
                }
            } else {
                app.project_sessions.insert(sessions_key, vec![info.clone()]);
            }
            if is_discovered_child {
                return Some(iced::Task::none());
            }

            app.new_session_creating = false;
            app.new_session_last_directory = Some(info.directory.clone());
            let save_last_directory_task = app.save_desktop_preference(
                "new_session_last_directory",
                serde_json::Value::String(info.directory.clone()),
            );
            app.active_session_id = Some(info.id.clone());
            app.chat_scroll_id = iced::widget::Id::unique();
            app.mark_active_session_viewed();
            app.chat.clear();
            app.chat_message_ids.clear();
            app.store_session_chat_snapshot(
                info.id.clone(),
                crate::app::session::shared_chat_messages(app.chat.clone()),
                app.chat_message_ids.clone(),
            );
            app.usage = models::TokenUsage::default();
            app.active_session_view_state = ActiveSessionViewState::default();
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
                active_session: app.active_session_id.as_deref(),
                target_session: &target_session_id,
                current_generation: app.session_view_cache.is_current_open(&target_session_id, open_generation),
                source: if source == PreparedSessionOpenSource::UiStore { HistorySource::UiStore } else { HistorySource::Remote },
                current_ids: &app.chat_message_ids,
                reset_boundary: app.session_history_reset_boundary(&target_session_id),
                current_signature: crate::app::session::session_content_signature(&app.chat, &app.chat_message_ids),
            };
            let acceptance = history_open.accept(result.as_ref().ok().map(|prepared| prepared.session_id.as_str()));
            let content_decision = result.as_ref().ok().map(|prepared| history_open.content(&prepared.message_ids, prepared.content_signature));
            let prepared = match result {
                Ok(prepared) if acceptance == HistoryDecision::Replace => {
                    prepared
                }
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
                        app.dialogue_flow_show_reasoning_summary,
                    ));
                }
                Err(error) if acceptance == HistoryDecision::ReportRemoteError => {
                    tracing::warn!(
                        target: "vw_desktop",
                        session_id = %target_session_id,
                        error = %error,
                        "failed to prepare remote session history"
                    );
                    if app.chat.is_empty() {
                        app.error_message =
                            Some(format!("Failed to load session messages: {error}"));
                    }
                    app.active_session_view_state.ui_preparing = false;
                    return Some(iced::Task::none());
                }
                Err(_) => return Some(iced::Task::none()),
            };

            if let Some(binding) = prepared.agent_binding.as_ref() {
                let runtime = app.get_session_runtime_mut(&target_session_id);
                if !runtime.fork_agent_selected {
                    runtime.agent = (binding.agent != crate::app::state::MAIN_AGENT_KEY)
                        .then(|| binding.agent.clone());
                    runtime.acp_agent = binding.acp_agent.clone();
                    runtime.agent_label = binding.label.clone();
                }
            }
            if let Some(full_access) = prepared.full_access {
                app.restore_session_full_access(&target_session_id, full_access);
            }
            if content_decision == Some(HistoryDecision::KeepCurrent) {
                tracing::warn!(
                    target: "vw_desktop",
                    session_id = %target_session_id,
                    remote_messages = prepared.message_ids.len(),
                    retained_messages = app.chat_message_ids.len(),
                    "preserved prepared session history because remote history was incomplete"
                );
                app.active_session_view_state.ui_preparing = false;
                return Some(iced::Task::none());
            }

            if content_decision == Some(HistoryDecision::RefreshMetadata) {
                app.active_session_view_state.ui_preparing = false;
                app.usage = prepared.usage;
                if !prepared.steps.is_empty() {
                    app.replace_active_session_steps(prepared.steps);
                }
                if !prepared.step_llm_raw.is_empty() {
                    app.replace_active_session_step_llm_raw(prepared.step_llm_raw);
                }
                if !prepared.llm_raw_step_indices.is_empty() {
                    app.replace_active_session_llm_raw_step_indices(prepared.llm_raw_step_indices);
                }
                if let Some(summary) = app.session_view_cache.record_open_commit(
                    &target_session_id,
                    open_generation,
                    crate::app::session::view_cache::SessionOpenUiCommit::SameContentRefresh,
                    app.chat.len(),
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
            if let Some(summary) = app.session_view_cache.record_open_commit(
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
                        app.dialogue_flow_show_reasoning_summary,
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
            let is_current = app.active_session_id.as_deref() == Some(&session_id)
                && app.session_view_cache.is_current_open(&session_id, open_generation);
            if !is_current {
                return Some(iced::Task::none());
            }
            let Some(timeline) = app.paged_chat_timeline.as_mut() else {
                return Some(iced::Task::none());
            };
            let page = match result {
                Ok(page) => page,
                Err(error) => {
                    timeline.cancel_prepend(&requested_cursor);
                    tracing::warn!(target: "vw_desktop", %session_id, %error, "failed to load older chat history page");
                    return Some(iced::Task::none());
                }
            };
            let old_keys = timeline
                .entries()
                .iter()
                .map(|entry| entry.key.clone())
                .collect::<std::collections::HashSet<_>>();
            let outcome =
                timeline.merge_prepend_page(&session_id, open_generation, &requested_cursor, page);
            let inserted = match outcome {
                crate::app::session::chat_timeline::PageMergeOutcome::Applied { inserted } => {
                    inserted
                }
                _ => return Some(iced::Task::none()),
            };
            let new_entries = timeline
                .entries()
                .iter()
                .filter(|entry| !old_keys.contains(&entry.key))
                .take(inserted)
                .cloned()
                .collect::<Vec<_>>();
            let new_messages =
                new_entries.iter().map(|entry| entry.message.clone()).collect::<Vec<_>>();
            let new_ids = new_entries
                .iter()
                .map(|entry| match &entry.key {
                    crate::app::session::chat_timeline::ChatEntryKey::PersistedId(id) => {
                        Some(id.clone())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            let seq_base = timeline.loaded_seq_range().map_or(0, |(first, _)| first);
            app.get_session_runtime_mut(&session_id).stream_save.seq_base = seq_base;
            app.chat.splice(0..0, new_messages);
            app.chat_message_ids.splice(0..0, new_ids);
            app.chat_virtualizer.prepend_estimated_heights(&vec![120.0; inserted]);
            app.chat_render_cache.clear();
            app.chat_visible_text_cache.clear();
            app.chat_copy_hash_cache.clear();
            app.chat_message_measured_heights.clear();
            app.sync_chat_virtualizer_compat_fields();
            Some(crate::app::message::project::prepare_session_ui_chunks_task(
                session_id,
                app.active_shared_chat_messages(),
                vec![0],
                None,
                app.dialogue_flow_show_reasoning_summary,
            ))
        }
        ProjectMessage::SessionMessagesLoaded(res) => {
            match res {
                Ok((id, msgs, usage)) => {
                    let loaded_session_model = latest_assistant_model_label(&msgs);
                    let loaded_steps = steps_from_gateway_messages(&msgs);
                    tracing::info!(
                        target: "vw_desktop",
                        session_id = %id,
                        message_count = msgs.len(),
                        active_session_matches = app.active_session_id.as_ref() == Some(&id),
                        "received session messages load result"
                    );
                    if app.active_session_id.as_ref() == Some(&id) {
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
                        let (next_chat, next_ids, shorter_gateway) = desktop_workspace::project_message::session::lifecycle::select_loaded_history(
                            msgs, local_messages, local_message_ids, reset_boundary,
                            crate::app::components::chat_panel::message_view::COMPACTION_COMPLETED_TEXT,
                        );
                        if let Some((gateway_messages, cached_messages)) = shorter_gateway {
                            tracing::warn!(
                                target: "vw_desktop", session_id = %id,
                                gateway_messages, cached_messages,
                                "preserved cached session history because gateway history was shorter"
                            );
                        }
                        let same_visible_history = desktop_workspace::project_message::session::lifecycle::same_visible_history(&app.chat, &next_chat);
                        if same_visible_history {
                            app.chat_message_ids = next_ids;
                            let shared_chat =
                                crate::app::session::shared_chat_messages(app.chat.clone());
                            app.store_session_chat_snapshot(
                                id.clone(),
                                shared_chat,
                                app.chat_message_ids.clone(),
                            );
                            app.usage = usage;
                            if !loaded_steps.is_empty() {
                                app.replace_active_session_steps(loaded_steps);
                            }
                            if restored_session_model {
                                app.sync_active_session_preferences();
                            }
                            app.rebuild_active_session_message_meta();
                            return Some(
                                crate::app::message::chat::refresh_active_trajectory_raw_task(
                                    app, &id,
                                ),
                            );
                        }

                        let shared_chat = crate::app::session::shared_chat_messages(next_chat);
                        app.chat_scroll_id = iced::widget::Id::unique();
                        app.chat = shared_chat.iter().cloned().collect();
                        app.chat_message_ids = next_ids;
                        app.store_session_chat_snapshot(
                            id.clone(),
                            shared_chat.clone(),
                            app.chat_message_ids.clone(),
                        );
                        app.usage = usage;
                        let existing_calls = app.active_session_view_state.calls.clone();
                        let existing_steps = app.active_session_view_state.steps.clone();
                        let next_steps =
                            if loaded_steps.is_empty() { existing_steps } else { loaded_steps };
                        app.active_session_view_state.updated_ms = 0;
                        app.clear_active_session_steps();
                        app.replace_active_session_steps(next_steps);
                        app.replace_active_session_calls(existing_calls);
                        if restored_session_model {
                            app.sync_active_session_preferences();
                        }
                        app.invalidate_chat_ui_state();
                        if app.chat.is_empty() {
                            tracing::warn!(
                                target: "vw_desktop",
                                session_id = %id,
                                "session message load completed but chat remained empty"
                            );
                        } else {
                            tracing::info!(
                                target: "vw_desktop",
                                session_id = %id,
                                chat_messages = app.chat.len(),
                                "applied loaded session messages to active chat"
                            );
                        }

                        if app.chat.is_empty() {
                            app.active_session_view_state.ui_preparing = false;
                            app.active_session_view_state.base_ready = true;
                            app.pin_chat_ui_chunk(None);
                        } else {
                            app.active_session_view_state.ui_preparing = true;
                            app.active_session_view_state.base_ready = false;
                            let base_chunk_start = app.preferred_base_chat_ui_chunk_start();
                            app.pin_chat_ui_chunk(Some(base_chunk_start));
                            app.mark_chat_ui_chunks_preparing(&[base_chunk_start]);
                            return Some(prepare_session_ui_task(
                                id,
                                shared_chat,
                                base_chunk_start,
                                true,
                                app.dialogue_flow_show_reasoning_summary,
                            ));
                        }
                    } else {
                        tracing::warn!(
                            target: "vw_desktop",
                            session_id = %id,
                            active_session_id = app.active_session_id.as_deref().unwrap_or("<none>"),
                            "discarded session messages because active session changed"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        target: "vw_desktop",
                        active_session_id = app.active_session_id.as_deref().unwrap_or("<none>"),
                        error = %e,
                        "failed to load session messages"
                    );
                    if let Some(id) = app.active_session_id.clone()
                        && let Some(local) = app.cached_chat_messages(&id)
                        && !local.is_empty()
                    {
                        let local_messages = local.iter().cloned().collect::<Vec<_>>();
                        let local_message_ids = if let Some(ids) = app.cached_chat_message_ids(&id)
                            && ids.len() == local_messages.len()
                        {
                            ids
                        } else {
                            vec![None; local_messages.len()]
                        };
                        let shared_chat = crate::app::session::shared_chat_messages(local_messages);
                        app.chat = shared_chat.iter().cloned().collect();
                        app.chat_message_ids = local_message_ids;
                        app.store_session_chat_snapshot(
                            id.clone(),
                            shared_chat.clone(),
                            app.chat_message_ids.clone(),
                        );
                        app.clear_active_session_steps();
                        app.active_session_view_state.ui_preparing = true;
                        app.active_session_view_state.base_ready = false;
                        app.invalidate_chat_ui_state();
                        let base_chunk_start = app.preferred_base_chat_ui_chunk_start();
                        app.pin_chat_ui_chunk(Some(base_chunk_start));
                        app.mark_chat_ui_chunks_preparing(&[base_chunk_start]);
                        return Some(prepare_session_ui_task(
                            id,
                            shared_chat,
                            base_chunk_start,
                            true,
                            app.dialogue_flow_show_reasoning_summary,
                        ));
                    } else {
                        app.error_message = Some(format!("Failed to load session messages: {}", e));
                    }
                }
            }
            Some(iced::Task::none())
        }
        ProjectMessage::SessionUiPrepared { session_id, stream_generation, phase } => {
            let is_active_session = app.active_session_id.as_ref() == Some(&session_id);
            if let Some(generation) = stream_generation {
                let reprepare = {
                    let state = &mut app.get_session_runtime_mut(&session_id).stream_tail_state;
                    state.finish_ui_preparation(generation);
                    if state.ui_generation() == generation || !is_active_session {
                        None
                    } else {
                        state.message_idx().zip(state.begin_ui_preparation())
                    }
                };
                if let Some((message_idx, latest_generation)) = reprepare
                    && let Some(message) = app.chat.get(message_idx).cloned()
                {
                    return Some(crate::app::message::project::prepare_stream_message_ui_task(
                        session_id,
                        message,
                        message_idx,
                        latest_generation,
                        app.dialogue_flow_show_reasoning_summary,
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
                    app.active_session_view_state.base_ready = true;
                    app.rebuild_active_session_message_meta();
                    app.active_session_view_state.ui_preparing = false;
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
                            app.dialogue_flow_show_reasoning_summary,
                        ));
                    }
                }
            }
            Some(iced::Task::none())
        }
        _ => None,
    }
}

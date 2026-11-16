//! 处理项目会话打开流程，把用户选择映射为活动会话和加载任务。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use super::common::reset_new_session_picker_state;
use crate::app::message::project::ProjectMessage;
use crate::app::message::project::helpers::{
    list_gateway_project_sessions,
    load_prepared_session_open_from_gateway_task, load_project_worktree_picker_options,
    prepare_session_ui_task,
};
use crate::app::projects::save_recent_projects_meta_background;
use crate::app::{App, Message, models};
use session_info::Info;
use web_time::Instant;

use desktop_workspace::project_message::session_open::{self, Navigation, ListEvent, ListEffect, SessionLists};

pub(crate) fn open_session_draft(
    app: &mut App,
    project_path: String,
    directory: String,
) -> iced::Task<Message> {
    app.session_view_cache.navigation_generation += 1;
    app.cache_active_session_chat();
    app.agent_stream_id = app.agent_stream_id.wrapping_add(1);
    app.active_session_id = None;
    app.chat_scroll_id = iced::widget::Id::unique();
    app.chat.clear();
    app.chat_message_ids.clear();
    app.files.clear();
    app.usage = models::TokenUsage::default();
    app.active_session_view_state = Default::default();
    app.invalidate_chat_ui_state();
    app.input_popovers.recommended_prompt = false;
    app.input_popovers.agent_sessions = false;
    app.input_editor = iced::widget::text_editor::Content::new();
    app.reset_draft_full_access();
    let runtime = app.current_session_runtime_mut();
    runtime.input_editor = iced::widget::text_editor::Content::new();
    runtime.fork_execution_started = None;
    runtime.fork_request_submitted = false;
    runtime.fork_agent_selected = false;
    runtime.acp_options_state.source_pending = None;
    runtime.acp_options_state.source_request_id = None;
    runtime.draft = Some(crate::app::state::SessionDraft {
        project_path,
        directory: directory.clone(),
        history_len: 0,
    });
    app.sync_active_session_preferences();
    app.new_session_last_directory = Some(directory.clone());
    app.save_desktop_preference("new_session_last_directory", serde_json::Value::String(directory))
}

pub(crate) fn open_project_session(
    app: &mut App,
    project_path: String,
    id: String,
    info: Option<Info>,
    prepared: Option<crate::app::session::PreparedSessionOpen>,
) -> iced::Task<Message> {
    app.session_view_cache.navigation_generation += 1;
    app.agent_stream_id = app.agent_stream_id.wrapping_add(1);
    app.input_popovers.recommended_prompt = false;
    app.input_popovers.agent_sessions = false;
    app.hovered_recent_project = None;
    app.session_menu_id = None;
    app.session_menu_anchor = None;
    app.stash_active_session_view();

    let open_task = if app.project_path.as_ref() == Some(&project_path) {
        iced::Task::none()
    } else {
        app.open_project(project_path.clone())
    };
    let execution_state = info.as_ref().map(|info| info.fork_execution_started);
    let directory = if let Some(info) = info {
        let directory = info.directory.clone();
        super::lifecycle::cache_project_session(app, project_path, info);
        directory
    } else {
        project_path
    };

    app.prepare_historical_session_model(&id);
    if let Some(started) = execution_state {
        let runtime = app.get_session_runtime_mut(&id);
        runtime.fork_execution_started = started;
        if started != Some(false) {
            runtime.fork_agent_selected = false;
        }
        if !runtime.request_lifecycle.is_requesting && runtime.request_lifecycle.active_agent_request.is_none() {
            runtime.fork_request_submitted = false;
        }
    }
    app.chat_fork_dialog = None;
    app.active_session_id = Some(id.clone());
    let open_generation = app.session_view_cache.begin_open(&id);
    app.mark_active_session_viewed();
    if let Some(snapshot) = app.session_view_cache.take(&id) {
        let commit_started_at = Instant::now();
        snapshot.restore_into(app);
        app.close_session_transient_ui();
        app.sync_active_session_preferences();
        let scroll_offset_y = app.chat_scroll_offset_y;
        if let Some(summary) = app.session_view_cache.record_open_commit(
            &id,
            open_generation,
            crate::app::session::view_cache::SessionOpenUiCommit::HotRestore,
            app.chat.len(),
            commit_started_at.elapsed(),
            true,
        ) {
            summary.emit();
        }
        let restore_scroll_task = iced::widget::operation::snap_to(
            app.chat_scroll_id.clone(),
            iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(scroll_offset_y) },
        )
        .map(|_: ()| Message::CopyDone);
        let queued = crate::app::message::chat::stream::start_next(app, &id);
        return iced::Task::batch(vec![
            open_task,
            restore_scroll_task,
            queued,
            iced::Task::done(Message::Chat(crate::app::message::ChatMessage::LoadInputPanelTodos)),
        ]);
    }

    app.chat_scroll_id = iced::widget::Id::unique();
    app.restore_chat_for_session(&id);
    app.usage = models::TokenUsage::default();
    app.active_session_view_state.updated_ms = 0;
    app.clear_active_session_steps();
    app.active_session_view_state.ui_preparing = true;
    app.active_session_view_state.base_ready = false;
    app.sync_active_session_preferences();
    let has_requesting_cache =
        app.session_is_requesting(&id) && app.session_chat_cache.contains_key(&id);
    let requesting_cache_prewarm_task = if has_requesting_cache && !app.chat.is_empty() {
        let base_chunk_start = app.preferred_base_chat_ui_chunk_start();
        app.pin_chat_ui_chunk(Some(base_chunk_start));
        app.mark_chat_ui_chunks_preparing(&[base_chunk_start]);
        prepare_session_ui_task(
            id.clone(),
            app.active_shared_chat_messages(),
            base_chunk_start,
            true,
            app.dialogue_flow_show_reasoning_summary,
        )
    } else {
        if has_requesting_cache {
            app.active_session_view_state.ui_preparing = false;
            app.active_session_view_state.base_ready = true;
            app.pin_chat_ui_chunk(None);
        }
        iced::Task::none()
    };
    let history_load_task = if let Some(prepared) = prepared {
        super::lifecycle::handle(
            app,
            ProjectMessage::SessionOpenPrepared {
                target_session_id: id,
                project_path: directory,
                open_generation,
                source: super::super::PreparedSessionOpenSource::RemoteHistory,
                result: Ok(prepared),
            },
        )
        .unwrap_or_else(iced::Task::none)
    } else if !has_requesting_cache {
        load_prepared_session_open_from_gateway_task(
            directory,
            id.clone(),
            open_generation,
            app.dialogue_flow_show_reasoning_summary,
        )
    } else {
        iced::Task::none()
    };
    iced::Task::batch(vec![
        open_task,
        requesting_cache_prewarm_task,
        history_load_task,
        iced::Task::done(Message::Chat(crate::app::message::ChatMessage::LoadInputPanelTodos)),
    ])
}

fn request_session_navigation(app: &mut App, path: String, id: String) -> iced::Task<Message> {
    app.session_view_cache.navigation_generation += 1;
    let generation = app.session_view_cache.navigation_generation;
    let requesting_draft = app.session_runtime_states.get(id.trim())
        .filter(|runtime| runtime.request_lifecycle.is_requesting)
        .and_then(|runtime| runtime.draft.as_ref())
        .map(|draft| (draft.project_path.as_str(), draft.directory.as_str()));
    let (id, cached, path) = match session_open::navigation(path, id,
        app.project_path.as_ref(), app.project_id.as_deref(), app.active_session_id.as_deref(),
        &app.sessions, &app.project_sessions, requesting_draft) {
        Navigation::Ignore => return iced::Task::none(),
        Navigation::Cached { project_path, id, info } => return open_project_session(app, project_path, id, Some(info), None),
        Navigation::Draft { project_path, id } => return open_project_session(app, project_path, id, None, None),
        Navigation::Resolve { id, cached, path } => (id, cached, path),
    };
    let from_project = app.project_path.clone();
    let from_session = app.active_session_id.clone();
    iced::Task::perform(
        async move {
            let client = crate::app::gateway_client().ok()?;
            session_open::resolve_navigation(&client, id, cached, path).await
        },
        move |destination| {
            Message::Project(ProjectMessage::SessionNavigationResolved {
                generation,
                from_project: from_project.clone(),
                from_session: from_session.clone(),
                destination,
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
            app.navigation_mode.code_projects_menu_open = false;
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
            Some(request_session_navigation(app, path, id))
        }
        ProjectMessage::SessionNavigationResolved {
            generation,
            from_project,
            from_session,
            destination,
        } => {
            let task = if let Some((project_path, info)) = session_open::accept_navigation(
                generation, app.session_view_cache.navigation_generation,
                &from_project, &app.project_path, &from_session, &app.active_session_id, destination) {
                open_project_session(app, project_path, info.id.clone(), Some(*info), None)
            } else {
                iced::Task::none()
            };
            Some(task)
        }
        ProjectMessage::CodeProjectsMenuChanged(open) => {
            app.navigation_mode.code_projects_menu_open = open;
            app.hovered_recent_project = None;
            Some(iced::Task::none())
        }
        ProjectMessage::RecentHovered(id) => {
            app.hovered_recent_project = id.clone();
            Some(id.map(|path| update_lists(app, ListEvent::Hover(path))).unwrap_or_else(iced::Task::none))
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
        ProjectMessage::SessionsLoaded(res) => Some(update_lists(app, ListEvent::Loaded(res))),
        ProjectMessage::SessionBootstrapLoaded { project_path, result, previews, archived_session_ids } => {
            Some(update_lists(app, ListEvent::Bootstrap { project_path, result, previews, archived: archived_session_ids }))
        }
        ProjectMessage::ProjectInfoLoaded(res) => {
            if let Ok(loaded) = res {
                let Some(completed) = session_open::complete_project(app.project_path.as_deref(),
                    &loaded.project_path, &mut app.recent_projects_meta, loaded.info, loaded.current_branch)
                else { return Some(iced::Task::none()); };
                if completed.metadata_changed {
                    save_recent_projects_meta_background(app.recent_projects_meta.clone());
                }
                app.project_updated_at_ms = completed.updated_at;
                if completed.selected_branch.is_some() { app.git.selected_branch = completed.selected_branch; }
                app.project_is_git_repo = completed.is_git_repo;
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
                app.project_id = Some(completed.id);
            }
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectSessionsLoaded(path, res) => Some(update_lists(app, ListEvent::ProjectLoaded(path, res))),
        ProjectMessage::ProjectSessionListScrollChanged { project_path, has_vertical_scrollbar } => Some(update_lists(app, ListEvent::Scrollbar(project_path, has_vertical_scrollbar))),
        ProjectMessage::ProjectLoadMoreSessions(path) => Some(update_lists(app, ListEvent::More(path))),
        ProjectMessage::ProjectCreateSession(project_path) => {
            let worktree_enabled = app.project_worktree_enabled.get(&project_path).copied().unwrap_or(false);
            let creation = session_open::creation(app.new_session_creating, worktree_enabled);
            if matches!(creation, session_open::Creation::Ignore) { return Some(iced::Task::none()); }
            let project_switch_task = if app.project_path.as_deref() == Some(project_path.as_str())
            {
                iced::Task::none()
            } else {
                app.open_project_and_index(project_path.clone())
            };
            if matches!(creation, session_open::Creation::Draft) {
                reset_new_session_picker_state(app);
                let draft = open_session_draft(app, project_path.clone(), project_path);
                return Some(iced::Task::batch([project_switch_task, draft]));
            }
            let picker_task = super::worktree::begin_picker(app, project_path.clone());
            let project_path_clone = project_path.clone();
            Some(iced::Task::batch(vec![
                project_switch_task,
                picker_task,
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
            Some(app.show_localized_toast(
                crate::app::state::ToastKind::Error,
                "创建新会话失败：%s",
                vec![format!("{}", error).into()],
            ))
        }
        ProjectMessage::ProjectBranchesLoaded { project_path, selected_branch, branches } => {
            desktop_workspace::projects::apply_branch_snapshot(app.project_path.as_deref(),
                desktop_workspace::projects::ProjectBranchSnapshot { project_path, selected_branch, branches },
                &mut app.git.selected_branch, &mut app.git.branches);
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectLoadSessions(path) => Some(update_lists(app, ListEvent::Load(path))),
        _ => None,
    }
}

#[cfg(test)]
#[path = "open_tests.rs"]
mod open_tests;

/// The gateway has validated this existing checkout before any UI state changes.
/// Apply project identity through the ordinary open-completion handler before
/// opening a blank draft; no worktree creation or initialization is involved.
pub(crate) fn open_existing_checkout(
    app: &mut App,
    project: vw_api_project::ProjectDto,
    directory: String,
) -> iced::Task<Message> {
    let path = project.directory.clone();
    let tab_id = format!("project_{path}");
    let switch = if app.project_path.as_deref() == Some(path.as_str())
        && matches!(app.screen, crate::app::Screen::Project)
        && app.active_tab_id.as_deref() == Some(tab_id.as_str())
    {
        iced::Task::none()
    } else {
        app.open_project_and_index(path.clone())
    };
    let loaded =
        crate::app::message::project::helpers::map_loaded_project_info(path.clone(), project);
    let completed =
        handle(app, ProjectMessage::ProjectInfoLoaded(Ok(loaded))).unwrap_or_else(iced::Task::none);
    app.show_system_settings = false;
    let draft = open_session_draft(app, path, directory);
    iced::Task::batch([switch, completed, draft])
}

fn update_lists(app: &mut App, event: ListEvent) -> iced::Task<Message> {
    let effects = session_open::update_lists(&mut SessionLists {
        sessions: &mut app.sessions, previews: &mut app.session_previews, archived: &mut app.archived_session_ids,
        projects: &mut app.project_sessions, counts: &mut app.project_session_load_counts,
        loading: &mut app.project_sessions_loading, scrollbars: &mut app.project_session_has_vertical_scrollbar,
    }, &app.project_path, event);
    let mut tasks = Vec::new();
    for effect in effects {
        match effect {
            ListEffect::Notification(text) => app.push_notification(text),
            ListEffect::Error(text) => app.error_message = Some(text),
            ListEffect::Load(path) => {
                let project_path = path.clone();
                tasks.push(iced::Task::perform(async move {
                    let client = crate::app::gateway_client().map_err(|err| err.to_string())?;
                    list_gateway_project_sessions(&client, &project_path).await
                }, |result| Message::Project(ProjectMessage::ProjectSessionsLoaded(path, result))));
            }
        }
    }
    iced::Task::batch(tasks)
}

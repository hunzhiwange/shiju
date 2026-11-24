//! 处理项目会话打开流程，把用户选择映射为活动会话和加载任务。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use super::common::reset_new_session_picker_state;
use crate::app::message::project::ProjectMessage;
use crate::app::message::project::helpers::{
    list_gateway_project_sessions, load_prepared_session_open_from_gateway_task,
    load_project_worktree_picker_options, prepare_session_ui_task,
};
use crate::app::projects::save_recent_projects_meta_background;
use crate::app::{App, Message};
use session_info::Info;
use web_time::Instant;

use desktop_workspace::project_message::session_open::{
    self, ListEffect, ListEvent, Navigation, SessionLists,
};

pub(crate) fn open_session_draft(
    app: &mut App,
    project_path: String,
    directory: String,
) -> iced::Task<Message> {
    app.session_state.session_view_cache.navigation_generation += 1;
    app.cache_active_session_chat();
    app.session_state.agent_stream_id = app.session_state.agent_stream_id.wrapping_add(1);
    app.session_state.active_session_id = None;
    let runtime = app.current_session_runtime_mut();
    let parked = runtime.parked_input_files.take();
    let editor = parked.as_ref().map(|_| runtime.input_editor.clone());
    runtime.open_draft(project_path, directory.clone());
    if let Some(editor) = editor {
        runtime.input_editor = editor;
    }
    app.session_state.files = parked.unwrap_or_default();
    reset_draft_view(app);
    app.reset_draft_full_access();
    app.sync_active_session_preferences();
    app.workspace_state.new_session_last_directory = Some(directory.clone());
    app.save_desktop_preference("new_session_last_directory", serde_json::Value::String(directory))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn open_employee_session_draft(
    app: &mut App,
    path: String,
    id: String,
) -> iced::Task<Message> {
    app.stash_active_session_view();
    let project_task = app.open_project(path.clone());
    app.session_state.active_session_id = Some(id);
    app.current_session_runtime_mut().open_draft(path.clone(), path);
    reset_draft_view(app);
    project_task
}

fn reset_draft_view(app: &mut App) {
    app.chat_view.chat_scroll_id = iced::widget::Id::unique();
    super::lifecycle::history_state(app).blank_draft();
    app.session_state.paged_chat_timeline = None;
    app.invalidate_chat_ui_state();
    app.session_state.input_popovers.recommended_prompt = false;
    app.session_state.input_popovers.agent_sessions = false;
    app.session_state.input_editor = app.current_session_runtime_ref()
        .map(|runtime| runtime.input_editor.clone())
        .unwrap_or_default();
}

pub(crate) fn open_project_session(
    app: &mut App,
    project_path: String,
    id: String,
    info: Option<Info>,
    prepared: Option<crate::app::session::PreparedSessionOpen>,
) -> iced::Task<Message> {
    #[cfg(not(target_arch = "wasm32"))]
    if app.session_state.session_runtime_states.get(&id).is_some_and(|runtime| runtime.is_employee_chat())
        && crate::app::digital_employees::chat::credential(app, &crate::app::config::gateway_client_endpoint()).is_none()
    {
        return app.show_localized_toast(crate::app::state::ToastKind::Error, "请登录识矩并连接本机网关后重开会话", vec![]);
    }
    app.session_state.session_view_cache.navigation_generation += 1;
    app.session_state.agent_stream_id = app.session_state.agent_stream_id.wrapping_add(1);
    app.session_state.input_popovers.recommended_prompt = false;
    app.session_state.input_popovers.agent_sessions = false;
    app.workspace_state.hovered_recent_project = None;
    app.workspace_state.session_menu_id = None;
    app.workspace_state.session_menu_anchor = None;
    app.stash_active_session_view();

    let open_task = if app.navigation_tabs.project_path.as_ref() == Some(&project_path) {
        resume_project_from_sidebar(app, &project_path)
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
        app.get_session_runtime_mut(&id).apply_open_execution_state(started);
    }
    app.session_state.chat_fork_dialog = None;
    app.session_state.active_session_id = Some(id.clone());
    app.session_state.files = app
        .get_session_runtime_mut(&id)
        .parked_input_files
        .take()
        .unwrap_or_default();
    let open_generation = app.session_state.session_view_cache.begin_open(&id);
    app.mark_active_session_viewed();
    #[cfg(not(target_arch = "wasm32"))]
    let media = crate::app::message::chat::media_jobs::restore(app);
    #[cfg(target_arch = "wasm32")]
    let media = iced::Task::none();
    if let Some(snapshot) = app.session_state.session_view_cache.take(&id) {
        let commit_started_at = Instant::now();
        snapshot.restore_into(app);
        app.close_session_transient_ui();
        app.sync_active_session_preferences();
        let scroll_offset_y = app.chat_view.chat_scroll_offset_y;
        if let Some(summary) = app.session_state.session_view_cache.record_open_commit(
            &id,
            open_generation,
            crate::app::session::view_cache::SessionOpenUiCommit::HotRestore,
            app.chat_view.chat.len(),
            commit_started_at.elapsed(),
            true,
        ) {
            summary.emit();
        }
        let restore_scroll_task = iced::widget::operation::snap_to(
            app.chat_view.chat_scroll_id.clone(),
            iced::widget::scrollable::RelativeOffset { x: Some(0.0), y: Some(scroll_offset_y) },
        )
        .map(|_: ()| Message::CopyDone);
        #[cfg(not(target_arch = "wasm32"))]
        let employee_label = crate::app::digital_employees::chat::refresh_history_label(app, &id);
        #[cfg(target_arch = "wasm32")]
        let employee_label = iced::Task::none();
        let queued = crate::app::message::chat::stream::start_next(app, &id);
        return iced::Task::batch(vec![
            open_task,
            restore_scroll_task,
            employee_label,
            queued,
            media,
            iced::Task::done(Message::Chat(crate::app::message::ChatMessage::LoadInputPanelTodos)),
        ]);
    }

    app.chat_view.chat_scroll_id = iced::widget::Id::unique();
    app.restore_chat_for_session(&id);
    super::lifecycle::history_state(app)
        .begin_cold_open(crate::app::components::chat_panel::trajectory::project_trajectory);
    app.sync_active_session_preferences();
    use desktop_session::session::history_commit::ColdHistory;
    let requesting = app.session_is_requesting(&id);
    let cached = app.session_state.session_chat_cache.contains_key(&id);
    let local_employee_draft = app.current_session_runtime_ref()
        .is_some_and(|runtime| runtime.is_employee_chat() && runtime.draft.is_some());
    let history_route = super::lifecycle::history_state(app)
        .cold_history(requesting || local_employee_draft, cached || local_employee_draft);
    let has_requesting_cache = history_route != ColdHistory::Remote;
    let requesting_cache_prewarm_task = if history_route == ColdHistory::RequestingMessages {
        let base_chunk_start = app.preferred_base_chat_ui_chunk_start();
        app.pin_chat_ui_chunk(Some(base_chunk_start));
        app.mark_chat_ui_chunks_preparing(&[base_chunk_start]);
        prepare_session_ui_task(
            id.clone(),
            app.active_shared_chat_messages(),
            base_chunk_start,
            true,
            app.general_settings.dialogue_flow_show_reasoning_summary,
        )
    } else {
        if history_route == ColdHistory::RequestingEmpty {
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
            app.general_settings.dialogue_flow_show_reasoning_summary,
        )
    } else {
        iced::Task::none()
    };
    iced::Task::batch(vec![
        open_task,
        requesting_cache_prewarm_task,
        history_load_task,
        media,
        iced::Task::done(Message::Chat(crate::app::message::ChatMessage::LoadInputPanelTodos)),
    ])
}

fn request_session_navigation(app: &mut App, path: String, id: String) -> iced::Task<Message> {
    app.session_state.session_view_cache.navigation_generation += 1;
    let generation = app.session_state.session_view_cache.navigation_generation;
    let requesting_draft = app.session_state.session_runtime_states
        .get(id.trim())
        .filter(|runtime| runtime.request_lifecycle.is_requesting || runtime.is_employee_chat())
        .and_then(|runtime| runtime.draft.as_ref())
        .map(|draft| (draft.project_path.as_str(), draft.directory.as_str()));
    let is_active_draft = requesting_draft.is_some_and(|(_, directory)| directory == path)
        && app.session_state.active_session_id.as_deref() == Some(id.trim());
    let (id, cached, path) = match session_open::navigation(
        path,
        id,
        app.navigation_tabs.project_path.as_ref(),
        app.workspace_state.project_id.as_deref(),
        app.session_state.active_session_id.as_deref(),
        &app.session_state.sessions,
        &app.workspace_state.project_sessions,
        requesting_draft,
    ) {
        Navigation::Ignore => {
            if is_active_draft
                && let Some(project_path) = app.navigation_tabs.project_path.clone()
            {
                return resume_project_from_sidebar(app, &project_path);
            }
            return iced::Task::none();
        }
        Navigation::Cached { project_path, id, info } => {
            return open_project_session(app, project_path, id, Some(info), None);
        }
        Navigation::Draft { project_path, id } => {
            return open_project_session(app, project_path, id, None, None);
        }
        Navigation::Resolve { id, cached, path } => (id, cached, path),
    };
    let from_project = app.navigation_tabs.project_path.clone();
    let from_session = app.session_state.active_session_id.clone();
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

fn resume_project_from_sidebar(app: &mut App, project_path: &str) -> iced::Task<Message> {
    if app.navigation_tabs.screen.shows_workspace_session_sidebar() {
        let (tab_id, _) = desktop_workspace::projects::project_tab(project_path);
        if app.navigation_tabs.open_tabs.iter().any(|tab| tab.id == tab_id) {
            return crate::app::message::view::update(
                app,
                crate::app::message::ViewMessage::TabSelected(tab_id),
            );
        }
        return app.open_project(project_path.to_owned());
    }
    iced::Task::none()
}

/// handle 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(crate) fn handle(app: &mut App, message: ProjectMessage) -> Option<iced::Task<Message>> {
    use desktop_workspace::project_message::session::overlay;
    let overlay_event = match &message {
        ProjectMessage::OpenRecentPressed(path) => Some(overlay::Event::Open(path.clone())),
        ProjectMessage::SwitchWorkSpacePressed(path) => Some(overlay::Event::Switch(path.clone())),
        ProjectMessage::CodeProjectsMenuChanged(open) => Some(overlay::Event::Menu(*open)),
        ProjectMessage::RecentHovered(path) => Some(overlay::Event::Hover(path.clone())),
        ProjectMessage::RecentOverlayClosed => Some(overlay::Event::Close),
        ProjectMessage::SessionCreateFailed(error) => {
            Some(overlay::Event::CreateFailed(error.clone()))
        }
        _ => None,
    };
    if let Some(event) = overlay_event {
        let effects = overlay::update(
            &mut overlay::State {
                code_menu: &mut app.navigation_mode.code_projects_menu_open,
                hovered: &mut app.workspace_state.hovered_recent_project,
                menu_id: &mut app.workspace_state.session_menu_id,
                menu_anchor: &mut app.workspace_state.session_menu_anchor,
                tools_path: &mut app.workspace_state.project_tools_menu_path,
                creating: &mut app.workspace_state.new_session_creating,
                picker_open: app.workspace_state.new_session_picker_project.is_some(),
            },
            event,
        );
        let mut tasks = Vec::new();
        for effect in effects {
            match effect {
                overlay::Effect::ResetPicker => reset_new_session_picker_state(app),
                overlay::Effect::Open(path) => tasks.push(app.open_project_and_index(path)),
                overlay::Effect::Switch(path) => {
                    let task = if app.navigation_tabs.project_path.as_deref() == Some(path.as_str()) {
                        resume_project_from_sidebar(app, &path)
                    } else {
                        app.switch_work_space_preserving_chat(path)
                    };
                    tasks.push(task);
                }
                overlay::Effect::Hover(path) => {
                    tasks.push(update_lists(app, ListEvent::Hover(path)))
                }
                overlay::Effect::CreateFailed(error) => tasks.push(app.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    "创建新会话失败：%s",
                    vec![error.into()],
                )),
            }
        }
        return Some(iced::Task::batch(tasks));
    }
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
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
                generation,
                app.session_state.session_view_cache.navigation_generation,
                &from_project,
                &app.navigation_tabs.project_path,
                &from_session,
                &app.session_state.active_session_id,
                destination,
            ) {
                open_project_session(app, project_path, info.id.clone(), Some(*info), None)
            } else {
                iced::Task::none()
            };
            Some(task)
        }
        ProjectMessage::SessionsLoaded(res) => Some(update_lists(app, ListEvent::Loaded(res))),
        ProjectMessage::SessionBootstrapLoaded {
            gateway_user_generation,
            project_path,
            result,
            previews,
            archived_session_ids,
        } => {
            if gateway_user_generation != app.session_state.gateway_user_generation {
                return Some(iced::Task::none());
            }
            Some(update_lists(
                app,
                ListEvent::Bootstrap { project_path, result, previews, archived: archived_session_ids },
            ))
        },
        ProjectMessage::ProjectInfoLoaded(res) => {
            if let Ok(loaded) = res {
                let Some(completed) = session_open::complete_project(
                    app.navigation_tabs.project_path.as_deref(),
                    &loaded.project_path,
                    &mut app.workspace_state.recent_projects_meta,
                    loaded.info,
                    loaded.current_branch,
                ) else {
                    return Some(iced::Task::none());
                };
                if completed.metadata_changed {
                    save_recent_projects_meta_background(app.workspace_state.recent_projects_meta.clone());
                }
                app.workspace_state.project_updated_at_ms = completed.updated_at;
                if completed.selected_branch.is_some() {
                    app.git.selected_branch = completed.selected_branch;
                }
                app.workspace_state.project_is_git_repo = completed.is_git_repo;
                if !app.workspace_state.project_is_git_repo
                    && app.task_board.task_board_executor_running
                {
                    app.task_board.task_board_executor_running = false;
                }
                app.workspace_state.project_id = Some(completed.id);
            }
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectSessionsLoaded(generation, path, res) => {
            if generation != app.session_state.gateway_user_generation {
                return Some(iced::Task::none());
            }
            Some(update_lists(app, ListEvent::ProjectLoaded(path, res)))
        }
        ProjectMessage::ProjectSessionListScrollChanged {
            project_path,
            has_vertical_scrollbar,
        } => Some(update_lists(app, ListEvent::Scrollbar(project_path, has_vertical_scrollbar))),
        ProjectMessage::ProjectLoadMoreSessions(path) => {
            Some(update_lists(app, ListEvent::More(path)))
        }
        ProjectMessage::ProjectCreateSession(project_path) => {
            let worktree_enabled =
                app.workspace_state.project_worktree_enabled.get(&project_path).copied().unwrap_or(false);
            let creation = session_open::creation(app.workspace_state.new_session_creating, worktree_enabled);
            if matches!(creation, session_open::Creation::Ignore) {
                return Some(iced::Task::none());
            }
            let project_switch_task =
                if app.navigation_tabs.project_path.as_deref() == Some(project_path.as_str()) {
                    resume_project_from_sidebar(app, &project_path)
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
        ProjectMessage::ProjectBranchesLoaded { project_path, selected_branch, branches } => {
            desktop_workspace::projects::apply_branch_snapshot(
                app.navigation_tabs.project_path.as_deref(),
                desktop_workspace::projects::ProjectBranchSnapshot {
                    project_path,
                    selected_branch,
                    branches,
                },
                &mut app.git.selected_branch,
                &mut app.git.branches,
            );
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
    let switch = if app.navigation_tabs.project_path.as_deref() == Some(path.as_str())
        && matches!(app.navigation_tabs.screen, crate::app::Screen::Project)
        && app.navigation_tabs.active_tab_id.as_deref() == Some(tab_id.as_str())
    {
        iced::Task::none()
    } else {
        app.open_project_and_index(path.clone())
    };
    let loaded =
        crate::app::message::project::helpers::map_loaded_project_info(path.clone(), project);
    let completed =
        handle(app, ProjectMessage::ProjectInfoLoaded(Ok(loaded))).unwrap_or_else(iced::Task::none);
    app.navigation_panels.show_system_settings = false;
    let draft = open_session_draft(app, path, directory);
    iced::Task::batch([switch, completed, draft])
}

fn update_lists(app: &mut App, event: ListEvent) -> iced::Task<Message> {
    let effects = session_open::update_lists(
        &mut SessionLists {
            sessions: &mut app.session_state.sessions,
            previews: &mut app.session_state.session_previews,
            archived: &mut app.session_state.archived_session_ids,
            projects: &mut app.workspace_state.project_sessions,
            counts: &mut app.workspace_state.project_session_load_counts,
            loading: &mut app.workspace_state.project_sessions_loading,
            scrollbars: &mut app.workspace_state.project_session_has_vertical_scrollbar,
        },
        &app.navigation_tabs.project_path,
        event,
    );
    let mut tasks = Vec::new();
    for effect in effects {
        match effect {
            ListEffect::Notification(text) => app.push_notification(text),
            ListEffect::Error(text) => app.input_feedback.error_message = Some(text),
            ListEffect::Load(path) => {
                let project_path = path.clone();
                let generation = app.session_state.gateway_user_generation;
                let client = crate::app::gateway_client().map_err(|err| err.to_string());
                tasks.push(iced::Task::perform(
                    async move {
                        list_gateway_project_sessions(&client?, &project_path).await
                    },
                    move |result| Message::Project(ProjectMessage::ProjectSessionsLoaded(generation, path, result)),
                ));
            }
        }
    }
    iced::Task::batch(tasks)
}

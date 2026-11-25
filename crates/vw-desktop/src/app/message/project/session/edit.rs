//! 项目编辑领域适配与跨域效果编排。
use crate::app::{App, Message};
use crate::app::message::project::ProjectMessage;
use crate::app::projects::{save_recent_projects_background, save_recent_projects_meta_background};
use desktop_workspace::project_message::edit::{self, Event, Effect, State};
use crate::app::message::task_board::next_agent_scheduler_status_generation;
use crate::app::views::workflow_manager::{load_project_workflow, select_project_workflow};
use vw_api_agent_scheduler::AgentSchedulerSelectWorkflowRequest;
pub(crate) fn handle(app: &mut App, message: ProjectMessage) -> Option<iced::Task<Message>> {
    #[cfg(not(target_arch = "wasm32"))]
    let saved = matches!(&message, ProjectMessage::ProjectEditRuntimeSaved(Ok(())));
    let previous_icon = if matches!(&message, ProjectMessage::ProjectEditSaved | ProjectMessage::ProjectEditWorkflowSaved(_, Ok(_))) {
        app.project_editor.project_edit_path.as_deref().map(|path| {
            app.workspace_state.recent_projects_meta.iter().find(|meta| meta.path == path)
                .map(|meta| (meta.icon.clone(), meta.icon_color.clone())).unwrap_or_default()
        })
    } else {
        None
    };
    let mut workflow_task = iced::Task::none();
    let event = match message {
        ProjectMessage::ProjectEditWorkflowLoaded(generation, result) => {
            let editor = &mut app.project_editor;
            if editor.project_edit_workflow_generation != generation || editor.project_edit_path.is_none() {
                return Some(iced::Task::none());
            }
            return Some(match result {
                Ok(snapshot) => {
                    editor.project_edit_task_board_settings.apply_workflow_snapshot(snapshot.clone());
                    editor.project_edit_workflow_baseline = Some(snapshot);
                    iced::Task::none()
                }
                Err(error) => app.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    "读取项目工作流失败：%s",
                    vec![error.into()],
                ),
            });
        }
        ProjectMessage::ProjectEditWorkflowSaved(generation, result) => {
            let editor = &mut app.project_editor;
            if editor.project_edit_workflow_generation != generation || !editor.project_edit_workflow_saving {
                return Some(iced::Task::none());
            }
            editor.project_edit_workflow_saving = false;
            match result {
                Ok(snapshot) => {
                    editor.project_edit_task_board_settings.apply_workflow_snapshot(Some(snapshot));
                    editor.project_edit_workflow_dirty = false;
                    if let Some(path) = editor.project_edit_path.as_ref()
                        && app.navigation_tabs.project_path.as_ref() == Some(path)
                    {
                        app.task_board.task_board_workflow_generation =
                            next_agent_scheduler_status_generation();
                    }
                    Event::ProjectEditSaved
                }
                Err(error) => return Some(app.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    "保存项目工作流失败：%s",
                    vec![error.into()],
                )),
            }
        }
        ProjectMessage::ProjectEditSaved if app.project_editor.project_edit_workflow_dirty => {
            let editor = &mut app.project_editor;
            let (Some(path), Some(expected_workflow)) =
                (editor.project_edit_path.clone(), editor.project_edit_workflow_baseline.clone())
            else {
                return Some(iced::Task::none());
            };
            if editor.project_edit_workflow_saving {
                return Some(iced::Task::none());
            }
            editor.project_edit_workflow_saving = true;
            let generation = editor.project_edit_workflow_generation;
            let request = AgentSchedulerSelectWorkflowRequest {
                workflow_id: editor.project_edit_task_board_settings.workflow_id.clone(),
                expected_workflow,
            };
            return Some(iced::Task::perform(
                async move { select_project_workflow(&path, request).await },
                move |result| Message::Project(ProjectMessage::ProjectEditWorkflowSaved(generation, result)),
            ));
        }
        ProjectMessage::ProjectToolsMenuToggled(value) => Event::ProjectToolsMenuToggled(value),
        ProjectMessage::CurrentProjectMenuToggled(value) => Event::CurrentProjectMenuToggled(value),
        ProjectMessage::ProjectToolsMenuClosed => Event::ProjectToolsMenuClosed,
        ProjectMessage::ProjectEditOpened(path) => {
            let editor = &mut app.project_editor;
            if editor.project_edit_workflow_saving {
                return Some(iced::Task::none());
            }
            editor.project_edit_workflow_generation = editor.project_edit_workflow_generation.wrapping_add(1);
            editor.project_edit_workflow_baseline = None;
            editor.project_edit_workflow_dirty = false;
            let generation = editor.project_edit_workflow_generation;
            let workflow_path = path.clone();
            workflow_task = iced::Task::perform(
                async move { load_project_workflow(&workflow_path).await },
                move |result| Message::Project(ProjectMessage::ProjectEditWorkflowLoaded(generation, result)),
            );
            Event::ProjectEditOpened(path)
        }
        ProjectMessage::ProjectEditTabSelected(value) => Event::ProjectEditTabSelected(value),
        ProjectMessage::ProjectEditNameChanged(value) => Event::ProjectEditNameChanged(value),
        ProjectMessage::ProjectEditIconChanged(value) => Event::ProjectEditIconChanged(value),
        ProjectMessage::ProjectEditIconHovered(value) => Event::ProjectEditIconHovered(value),
        ProjectMessage::ProjectEditIconPickFile => Event::ProjectEditIconPickFile,
        ProjectMessage::ProjectEditIconFilePicked(value) => Event::ProjectEditIconFilePicked(value),
        ProjectMessage::ProjectEditIconColorChanged(value) => Event::ProjectEditIconColorChanged(value),
        ProjectMessage::ProjectEditIconColorPresetSelected(value) => Event::ProjectEditIconColorPresetSelected(value),
        ProjectMessage::ProjectEditIconColorPickerToggled => Event::ProjectEditIconColorPickerToggled,
        ProjectMessage::ProjectEditIconColorPickerClosed => Event::ProjectEditIconColorPickerClosed,
        ProjectMessage::ProjectEditIconColorFormatChanged(value) => Event::ProjectEditIconColorFormatChanged(value),
        ProjectMessage::ProjectEditStartScriptChanged(value) => Event::ProjectEditStartScriptChanged(value),
        ProjectMessage::ProjectEditStartScriptEditorAction(value) => Event::ProjectEditStartScriptEditorAction(value),
        ProjectMessage::ProjectEditWorktreeToggled(value) => Event::ProjectEditWorktreeToggled(value),
        ProjectMessage::ProjectEditTaskBoardAutoRefreshToggled(value) => Event::ProjectEditTaskBoardAutoRefreshToggled(value),
        ProjectMessage::ProjectEditSessionAutoRefreshToggled(value) => Event::ProjectEditSessionAutoRefreshToggled(value),
        ProjectMessage::ProjectEditMaxConcurrentInputChanged(value) => Event::ProjectEditMaxConcurrentInputChanged(value),
        ProjectMessage::ProjectEditSessionRefreshIntervalSecondsInputChanged(value) => Event::ProjectEditSessionRefreshIntervalSecondsInputChanged(value),
        ProjectMessage::ProjectEditTaskBoardRefreshIntervalSecondsInputChanged(value) => Event::ProjectEditTaskBoardRefreshIntervalSecondsInputChanged(value),
        ProjectMessage::ProjectEditTaskBoardSchedulerTickIntervalSecondsInputChanged(value) => Event::ProjectEditTaskBoardSchedulerTickIntervalSecondsInputChanged(value),
        ProjectMessage::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsInputChanged(value) => Event::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsInputChanged(value),
        ProjectMessage::ProjectEditFailedRetryMinutesInputChanged(value) => Event::ProjectEditFailedRetryMinutesInputChanged(value),
        ProjectMessage::ProjectEditFailedRetryMaxAttemptsInputChanged(value) => Event::ProjectEditFailedRetryMaxAttemptsInputChanged(value),
        ProjectMessage::ProjectEditRunningTimeoutMinutesInputChanged(value) => Event::ProjectEditRunningTimeoutMinutesInputChanged(value),
        ProjectMessage::ProjectEditPrSubmittedStallTimeoutSecondsInputChanged(value) => Event::ProjectEditPrSubmittedStallTimeoutSecondsInputChanged(value),
        ProjectMessage::ProjectEditMaxConcurrentChanged(value) => Event::ProjectEditMaxConcurrentChanged(value),
        ProjectMessage::ProjectEditSessionRefreshIntervalSecondsChanged(value) => Event::ProjectEditSessionRefreshIntervalSecondsChanged(value),
        ProjectMessage::ProjectEditTaskBoardRefreshIntervalSecondsChanged(value) => Event::ProjectEditTaskBoardRefreshIntervalSecondsChanged(value),
        ProjectMessage::ProjectEditTaskBoardSchedulerTickIntervalSecondsChanged(value) => Event::ProjectEditTaskBoardSchedulerTickIntervalSecondsChanged(value),
        ProjectMessage::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsChanged(value) => Event::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsChanged(value),
        ProjectMessage::ProjectEditFailedRetryMinutesChanged(value) => Event::ProjectEditFailedRetryMinutesChanged(value),
        ProjectMessage::ProjectEditFailedRetryMaxAttemptsChanged(value) => Event::ProjectEditFailedRetryMaxAttemptsChanged(value),
        ProjectMessage::ProjectEditRunningTimeoutMinutesChanged(value) => Event::ProjectEditRunningTimeoutMinutesChanged(value),
        ProjectMessage::ProjectEditPrSubmittedStallTimeoutSecondsChanged(value) => Event::ProjectEditPrSubmittedStallTimeoutSecondsChanged(value),
        ProjectMessage::ProjectEditRecycleWorktreeOnTaskFinishToggled(value) => Event::ProjectEditRecycleWorktreeOnTaskFinishToggled(value),
        ProjectMessage::ProjectEditWorkflowIdChanged(value) => {
            let editor = &mut app.project_editor;
            if editor.project_edit_workflow_baseline.is_none() || editor.project_edit_workflow_saving
                || desktop_task_model::task_workflow_by_id(&value).is_none()
            {
                return Some(iced::Task::none());
            }
            editor.project_edit_workflow_dirty = true;
            Event::ProjectEditWorkflowIdChanged(value)
        }
        ProjectMessage::ProjectEditSaved => {
            let editor = &mut app.project_editor;
            if editor.project_edit_path.is_some()
                && editor.project_edit_path == app.navigation_tabs.project_path
            {
                let active = &app.task_board.task_board_settings;
                editor.project_edit_task_board_settings.workflow_id = active.workflow_id.clone();
                editor.project_edit_task_board_settings.workflow_snapshot = active.workflow_snapshot.clone();
            }
            Event::ProjectEditSaved
        }
        ProjectMessage::ProjectEditRuntimeSaved(value) => Event::ProjectEditRuntimeSaved(value),
        ProjectMessage::ProjectEditCanceled => Event::ProjectEditCanceled,
        ProjectMessage::RecentRevealPressed(value) => Event::RecentRevealPressed(value),
        ProjectMessage::RecentRemovePressed(value) => Event::RecentRemovePressed(value),
        ProjectMessage::RecentRemoveCanceled => Event::RecentRemoveCanceled,
        ProjectMessage::RecentRemoveConfirmed(value) => Event::RecentRemoveConfirmed(value),
        _ => return None,
    };
    if app.project_editor.project_edit_workflow_saving {
        return Some(iced::Task::none());
    }
    let effects = edit::handle(&mut State {
        editor: &mut app.project_editor,
        hovered_recent_project: &mut app.workspace_state.hovered_recent_project,
        project_path: &mut app.navigation_tabs.project_path,
        project_tools_menu_path: &mut app.workspace_state.project_tools_menu_path,
        project_worktree_enabled: &mut app.workspace_state.project_worktree_enabled,
        recent_project_remove_confirm_path: &mut app.workspace_state.recent_project_remove_confirm_path,
        recent_projects: &mut app.workspace_state.recent_projects,
        recent_projects_edits: &mut app.workspace_state.recent_projects_edits,
        recent_projects_meta: &mut app.workspace_state.recent_projects_meta,
        session_menu_anchor: &mut app.workspace_state.session_menu_anchor,
        session_menu_id: &mut app.workspace_state.session_menu_id,
        task_board_settings: &mut app.task_board.task_board_settings,
        work_current_project_menu: &mut app.navigation_mode.work_current_project_menu,
    }, event);
    let mut tasks = vec![workflow_task];
    for effect in effects {
        match effect {
            Effect::PickIcon => tasks.push(iced::Task::perform(async {
                #[cfg(not(target_arch = "wasm32"))]
                { rfd::AsyncFileDialog::new().add_filter("Image", &["png", "jpg", "jpeg", "gif", "webp", "bmp"]).pick_file().await.map(|f| f.path().to_string_lossy().to_string()) }
                #[cfg(target_arch = "wasm32")]
                { None }
            }, |picked| Message::Project(ProjectMessage::ProjectEditIconFilePicked(picked)))),
            Effect::SaveWorktree(path, enabled) => tasks.push(crate::app::message::settings::set_worktree_enabled(app, path, enabled)),
            Effect::SaveMeta => save_recent_projects_meta_background(app.workspace_state.recent_projects_meta.clone()),
            Effect::SaveRecent => save_recent_projects_background(app.workspace_state.recent_projects.clone()),
            Effect::OpenFirst => {
                #[cfg(not(target_arch = "wasm32"))]
                if matches!(app.navigation_tabs.screen, crate::app::Screen::CliWorkbench | crate::app::Screen::AppTerminal | crate::app::Screen::GitWorkbench) {
                    continue;
                }
                tasks.push(app.open_first_recent_project_or_home());
            }
            Effect::Reveal(path) => {
                #[cfg(not(target_arch = "wasm32"))]
                { let _ = open::that(&path); }
                #[cfg(target_arch = "wasm32")]
                let _ = path;
            }
            Effect::RuntimeError(err) => {
                #[cfg(not(target_arch = "wasm32"))]
                if matches!(app.navigation_tabs.screen, crate::app::Screen::CliWorkbench | crate::app::Screen::AppTerminal | crate::app::Screen::GitWorkbench)
                    || app.terminal.is_fullscreen
                {
                    tasks.push(app.show_localized_toast(crate::app::state::ToastKind::Error, "保存项目扩展配置失败: %s", vec![err.into()]));
                    continue;
                }
                app.set_localized_input_error(crate::app::i18n::input::ErrorField::General, "保存项目扩展配置失败: %s", vec![err.into()]);
            }
            Effect::RuntimeUpdate { path, name, icon, icon_color, start_command } => {
                let icon = (previous_icon.as_ref() != Some(&(icon.clone(), icon_color.clone())))
                    .then_some(vw_api_project::IconUpdateDto { override_icon: icon, color: icon_color });
                tasks.push(iced::Task::perform(async move {
                    let client = crate::app::gateway_client()?;
                    let project = crate::app::message::project::helpers::resolve_gateway_project(&path).await?;
                    let scope = desktop_session::session_gateway::active_project_scope(crate::app::config::gateway_client_endpoint(), project.id.clone())?;
                    gateway_project::GatewayProjectClient::new(client.transport().clone()).project_update(&scope, &vw_api_project::UpdateProjectRequest {
                        name: Some(name), active_worktree_id: None,
                        icon,
                        commands: Some(vw_api_project::CommandsUpdateDto { start: start_command }),
                    }).await.map(|_| ())
                }, |res| Message::Project(ProjectMessage::ProjectEditRuntimeSaved(res))));
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    if saved {
        tasks.push(crate::app::workbench_projects::refresh_lists(app));
    }
    Some(iced::Task::batch(tasks))
}
#[cfg(test)]
#[path = "edit_tests.rs"]
mod edit_tests;

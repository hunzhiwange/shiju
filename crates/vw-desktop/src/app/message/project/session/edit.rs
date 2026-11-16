//! 项目编辑领域适配与跨域效果编排。
use crate::app::{App, Message};
use crate::app::message::project::ProjectMessage;
use crate::app::projects::{save_recent_projects_background, save_recent_projects_meta_background};
use desktop_workspace::project_message::edit::{self, Event, Effect, State};
pub(crate) fn handle(app: &mut App, message: ProjectMessage) -> Option<iced::Task<Message>> {
    let event = match message {
        ProjectMessage::ProjectToolsMenuToggled(value) => Event::ProjectToolsMenuToggled(value),
        ProjectMessage::CurrentProjectMenuToggled(value) => Event::CurrentProjectMenuToggled(value),
        ProjectMessage::ProjectToolsMenuClosed => Event::ProjectToolsMenuClosed,
        ProjectMessage::ProjectEditOpened(value) => Event::ProjectEditOpened(value),
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
        ProjectMessage::ProjectEditWorkflowIdChanged(value) => Event::ProjectEditWorkflowIdChanged(value),
        ProjectMessage::ProjectEditSaved => Event::ProjectEditSaved,
        ProjectMessage::ProjectEditRuntimeSaved(value) => Event::ProjectEditRuntimeSaved(value),
        ProjectMessage::ProjectEditCanceled => Event::ProjectEditCanceled,
        ProjectMessage::RecentRevealPressed(value) => Event::RecentRevealPressed(value),
        ProjectMessage::RecentRemovePressed(value) => Event::RecentRemovePressed(value),
        ProjectMessage::RecentRemoveCanceled => Event::RecentRemoveCanceled,
        ProjectMessage::RecentRemoveConfirmed(value) => Event::RecentRemoveConfirmed(value),
        _ => return None,
    };
    let effects = edit::handle(&mut State {
        editor: &mut app.project_editor,
        hovered_recent_project: &mut app.hovered_recent_project,
        project_path: &mut app.navigation_tabs.project_path,
        project_tools_menu_path: &mut app.project_tools_menu_path,
        project_worktree_enabled: &mut app.project_worktree_enabled,
        recent_project_remove_confirm_path: &mut app.recent_project_remove_confirm_path,
        recent_projects: &mut app.recent_projects,
        recent_projects_edits: &mut app.recent_projects_edits,
        recent_projects_meta: &mut app.recent_projects_meta,
        session_menu_anchor: &mut app.session_menu_anchor,
        session_menu_id: &mut app.session_menu_id,
        task_board_settings: &mut app.task_board_settings,
        work_current_project_menu: &mut app.navigation_mode.work_current_project_menu,
    }, event);
    let mut tasks = Vec::new();
    for effect in effects {
        match effect {
            Effect::PickIcon => tasks.push(iced::Task::perform(async {
                #[cfg(not(target_arch = "wasm32"))]
                { rfd::AsyncFileDialog::new().add_filter("Image", &["png", "jpg", "jpeg", "gif", "webp", "bmp"]).pick_file().await.map(|f| f.path().to_string_lossy().to_string()) }
                #[cfg(target_arch = "wasm32")]
                { None }
            }, |picked| Message::Project(ProjectMessage::ProjectEditIconFilePicked(picked)))),
            Effect::SaveWorktree(path, enabled) => tasks.push(crate::app::message::settings::set_worktree_enabled(app, path, enabled)),
            Effect::SaveMeta => save_recent_projects_meta_background(app.recent_projects_meta.clone()),
            Effect::SaveRecent => save_recent_projects_background(app.recent_projects.clone()),
            Effect::OpenFirst => tasks.push(app.open_first_recent_project_or_home()),
            Effect::Reveal(path) => {
                #[cfg(not(target_arch = "wasm32"))]
                { let _ = open::that(&path); }
                #[cfg(target_arch = "wasm32")]
                let _ = path;
            }
            Effect::RuntimeError(err) => app.set_localized_input_error(crate::app::i18n::input::ErrorField::General, "保存项目扩展配置失败: %s", vec![format!("{}", err).into()]),
            Effect::RuntimeUpdate { path, name, icon, icon_color, start_command } => {
                tasks.push(iced::Task::perform(async move {
                    let client = crate::app::gateway_client()?;
                    let project = crate::app::message::project::helpers::resolve_gateway_project(&path).await?;
                    let scope = desktop_session::session_gateway::active_project_scope(crate::app::config::gateway_client_endpoint(), project.id.clone())?;
                    gateway_project::GatewayProjectClient::new(client.transport().clone()).project_update(&scope, &vw_api_project::UpdateProjectRequest {
                        name: Some(name), active_worktree_id: None,
                        icon: Some(vw_api_project::IconUpdateDto { override_icon: icon, color: icon_color }),
                        commands: Some(vw_api_project::CommandsUpdateDto { start: start_command }),
                    }).await.map(|_| ())
                }, |res| Message::Project(ProjectMessage::ProjectEditRuntimeSaved(res))));
            }
        }
    }
    Some(iced::Task::batch(tasks))
}
#[cfg(test)]
#[path = "edit_tests.rs"]
mod edit_tests;

//! 项目编辑和最近项目更新；返回由桌面壳执行的跨域效果。
pub use super::edit_state::*;
pub enum Effect {
    PickIcon,
    SaveMeta,
    SaveRecent,
    OpenFirst,
    Reveal(String),
    RuntimeError(String),
    SaveWorktree(String, bool),
    RuntimeUpdate {
        path: String,
        name: String,
        icon: Option<String>,
        icon_color: Option<String>,
        start_command: Option<String>,
    },
}
pub fn handle(app: &mut State<'_>, message: Event) -> Vec<Effect> {
    match message {
        Event::ProjectToolsMenuToggled(path) => {
            (*app.work_current_project_menu) = None;
            if (*app.project_tools_menu_path).as_ref() == Some(&path) {
                (*app.project_tools_menu_path) = None;
            } else {
                (*app.project_tools_menu_path) = Some(path);
            }
            Vec::new()
        }
        Event::CurrentProjectMenuToggled(path) => {
            if (*app.project_path).as_deref() == Some(path.as_str()) {
                (*app.project_tools_menu_path) = None;
                (*app.session_menu_id) = None;
                (*app.session_menu_anchor) = None;
                (*app.work_current_project_menu) =
                    ((*app.work_current_project_menu).as_ref() != Some(&path)).then_some(path);
            }
            Vec::new()
        }
        Event::ProjectToolsMenuClosed => {
            (*app.project_tools_menu_path) = None;
            (*app.work_current_project_menu) = None;
            Vec::new()
        }
        Event::ProjectEditOpened(path) => {
            let fallback_name = std::path::Path::new(&path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&path)
                .to_string();
            let edited_name = (*app.recent_projects)
                .iter()
                .position(|p| p == &path)
                .and_then(|idx| (*app.recent_projects_edits).get(idx).cloned())
                .or_else(|| {
                    (*app.recent_projects_meta)
                        .iter()
                        .find(|meta| meta.path == path)
                        .map(|meta| meta.name.clone())
                })
                .unwrap_or(fallback_name);
            let task_board_settings = (*app.recent_projects_meta)
                .iter()
                .find(|meta| meta.path == path)
                .and_then(|meta| meta.task_board_settings.clone())
                .or_else(|| {
                    if (*app.project_path).as_ref() == Some(&path) {
                        Some((*app.task_board_settings).clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(desktop_task_model::TaskBoardSettings::new);

            app.editor.project_edit_path = Some(path.clone());
            app.editor.project_edit_tab = ProjectEditTab::General;
            app.editor.project_edit_name = edited_name;
            let edited_icon = (*app.recent_projects_meta)
                .iter()
                .find(|meta| meta.path == path)
                .and_then(|meta| meta.icon.clone())
                .unwrap_or_default();
            let edited_icon_color = (*app.recent_projects_meta)
                .iter()
                .find(|meta| meta.path == path)
                .and_then(|meta| meta.icon_color.clone())
                .unwrap_or_default();
            let edited_worktree_start_command = (*app.recent_projects_meta)
                .iter()
                .find(|meta| meta.path == path)
                .and_then(|meta| meta.worktree_start_command.clone())
                .unwrap_or_default();
            app.editor.project_edit_icon = edited_icon;
            app.editor.project_edit_icon_hovered = false;
            app.editor.project_edit_icon_color = edited_icon_color;
            app.editor.project_edit_icon_color_picker_open = false;
            app.editor.project_edit_icon_color_format =
                desktop_ui_controls::color_picker::ColorFormat::Hex;
            app.editor.project_edit_start_script = edited_worktree_start_command;
            app.editor.project_edit_start_script_editor =
                iced::widget::text_editor::Content::with_text(&app.editor.project_edit_start_script);
            app.editor.project_edit_worktree_enabled =
                (*app.project_worktree_enabled).get(&path).copied().unwrap_or(false);
            #[cfg(not(target_arch = "wasm32"))]
            {
                app.editor.project_edit_worktree_dirty = false;
            }
            app.editor.project_edit_task_board_settings = task_board_settings.sanitized();
            let (session_auto_refresh, session_refresh_interval_seconds) = (*app.recent_projects_meta)
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
            app.editor.project_edit_max_concurrent_input =
                app.editor.project_edit_task_board_settings.max_concurrent.clamp(1, 10).to_string();
            app.editor.project_edit_task_board_auto_refresh =
                app.editor.project_edit_task_board_settings.auto_refresh;
            app.editor.project_edit_session_auto_refresh = session_auto_refresh;
            app.editor.project_edit_session_refresh_interval_seconds_input =
                session_refresh_interval_seconds.to_string();
            app.editor.project_edit_task_board_refresh_interval_seconds_input = app.editor.project_edit_task_board_settings
                .refresh_interval_seconds
                .clamp(1, 3600)
                .to_string();
            app.editor.project_edit_task_board_scheduler_tick_interval_seconds_input = app.editor.project_edit_task_board_settings
                .scheduler_tick_interval_seconds
                .clamp(1, 60)
                .to_string();
            app.editor.project_edit_task_board_auto_promote_tick_interval_seconds_input = app.editor.project_edit_task_board_settings
                .auto_promote_tick_interval_seconds
                .clamp(1, 3600)
                .to_string();
            app.editor.project_edit_failed_retry_minutes_input = app.editor.project_edit_task_board_settings
                .failed_retry_minutes
                .clamp(1, 1440)
                .to_string();
            app.editor.project_edit_failed_retry_max_attempts_input = app.editor.project_edit_task_board_settings
                .failed_retry_max_attempts
                .clamp(0, 10)
                .to_string();
            app.editor.project_edit_running_timeout_minutes_input = app.editor.project_edit_task_board_settings
                .running_timeout_minutes
                .clamp(1, 1440)
                .to_string();
            app.editor.project_edit_pr_submitted_stall_timeout_seconds_input = app.editor.project_edit_task_board_settings
                .pr_submitted_stall_timeout_seconds
                .clamp(5, 3600)
                .to_string();
            (*app.project_tools_menu_path) = None;
            (*app.work_current_project_menu) = None;
            Vec::new()
        }
        Event::ProjectEditTabSelected(tab) => {
            app.editor.project_edit_tab = tab;
            Vec::new()
        }
        Event::ProjectEditNameChanged(name) => {
            app.editor.project_edit_name = name;
            Vec::new()
        }
        Event::ProjectEditIconChanged(icon) => {
            app.editor.project_edit_icon = icon;
            app.editor.project_edit_icon_hovered = false;
            Vec::new()
        }
        Event::ProjectEditIconHovered(hovered) => {
            app.editor.project_edit_icon_hovered = hovered;
            Vec::new()
        }
        Event::ProjectEditIconPickFile => vec![Effect::PickIcon],
        Event::ProjectEditIconFilePicked(picked) => {
            if let Some(path) = picked {
                let trimmed = path.trim();
                if !trimmed.is_empty() {
                    app.editor.project_edit_icon = trimmed.to_string();
                    app.editor.project_edit_icon_hovered = false;
                }
            }
            Vec::new()
        }
        Event::ProjectEditIconColorChanged(color) => {
            app.editor.project_edit_icon_color = color;
            Vec::new()
        }
        Event::ProjectEditIconColorPresetSelected(color) => {
            app.editor.project_edit_icon_color = color;
            Vec::new()
        }
        Event::ProjectEditIconColorPickerToggled => {
            app.editor.project_edit_icon_color_picker_open = !app.editor.project_edit_icon_color_picker_open;
            Vec::new()
        }
        Event::ProjectEditIconColorPickerClosed => {
            app.editor.project_edit_icon_color_picker_open = false;
            Vec::new()
        }
        Event::ProjectEditIconColorFormatChanged(format) => {
            app.editor.project_edit_icon_color_format = format;
            Vec::new()
        }
        Event::ProjectEditStartScriptChanged(command) => {
            app.editor.project_edit_start_script = command;
            app.editor.project_edit_start_script_editor =
                iced::widget::text_editor::Content::with_text(&app.editor.project_edit_start_script);
            Vec::new()
        }
        Event::ProjectEditStartScriptEditorAction(action) => {
            app.editor.project_edit_start_script_editor.perform(action);
            app.editor.project_edit_start_script =
                app.editor.project_edit_start_script_editor.text().trim_end_matches('\n').to_string();
            Vec::new()
        }
        Event::ProjectEditWorktreeToggled(enabled) => {
            app.editor.project_edit_worktree_enabled = enabled;
            #[cfg(not(target_arch = "wasm32"))]
            {
                app.editor.project_edit_worktree_dirty = true;
            }
            Vec::new()
        }
        Event::ProjectEditTaskBoardAutoRefreshToggled(enabled) => {
            app.editor.project_edit_task_board_auto_refresh = enabled;
            app.editor.project_edit_task_board_settings.auto_refresh = enabled;
            Vec::new()
        }
        Event::ProjectEditSessionAutoRefreshToggled(enabled) => {
            app.editor.project_edit_session_auto_refresh = enabled;
            Vec::new()
        }
        Event::ProjectEditMaxConcurrentInputChanged(value) => {
            app.editor.project_edit_max_concurrent_input = value;
            Vec::new()
        }
        Event::ProjectEditSessionRefreshIntervalSecondsInputChanged(value) => {
            app.editor.project_edit_session_refresh_interval_seconds_input = value;
            Vec::new()
        }
        Event::ProjectEditTaskBoardRefreshIntervalSecondsInputChanged(value) => {
            app.editor.project_edit_task_board_refresh_interval_seconds_input = value;
            Vec::new()
        }
        Event::ProjectEditTaskBoardSchedulerTickIntervalSecondsInputChanged(value) => {
            app.editor.project_edit_task_board_scheduler_tick_interval_seconds_input = value;
            Vec::new()
        }
        Event::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsInputChanged(value) => {
            app.editor.project_edit_task_board_auto_promote_tick_interval_seconds_input = value;
            Vec::new()
        }
        Event::ProjectEditFailedRetryMinutesInputChanged(value) => {
            app.editor.project_edit_failed_retry_minutes_input = value;
            Vec::new()
        }
        Event::ProjectEditFailedRetryMaxAttemptsInputChanged(value) => {
            app.editor.project_edit_failed_retry_max_attempts_input = value;
            Vec::new()
        }
        Event::ProjectEditRunningTimeoutMinutesInputChanged(value) => {
            app.editor.project_edit_running_timeout_minutes_input = value;
            Vec::new()
        }
        Event::ProjectEditPrSubmittedStallTimeoutSecondsInputChanged(value) => {
            app.editor.project_edit_pr_submitted_stall_timeout_seconds_input = value;
            Vec::new()
        }
        Event::ProjectEditMaxConcurrentChanged(count) => {
            let clamped = count.clamp(1, 10);
            app.editor.project_edit_task_board_settings.max_concurrent = clamped;
            app.editor.project_edit_max_concurrent_input = clamped.to_string();
            Vec::new()
        }
        Event::ProjectEditSessionRefreshIntervalSecondsChanged(seconds) => {
            let clamped = seconds.clamp(1, 3600);
            app.editor.project_edit_session_refresh_interval_seconds_input = clamped.to_string();
            Vec::new()
        }
        Event::ProjectEditTaskBoardRefreshIntervalSecondsChanged(seconds) => {
            let clamped = seconds.clamp(1, 3600);
            app.editor.project_edit_task_board_settings.refresh_interval_seconds = clamped;
            app.editor.project_edit_task_board_refresh_interval_seconds_input = clamped.to_string();
            Vec::new()
        }
        Event::ProjectEditTaskBoardSchedulerTickIntervalSecondsChanged(seconds) => {
            let clamped = seconds.clamp(1, 60);
            app.editor.project_edit_task_board_settings.scheduler_tick_interval_seconds = clamped;
            app.editor.project_edit_task_board_scheduler_tick_interval_seconds_input = clamped.to_string();
            Vec::new()
        }
        Event::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsChanged(seconds) => {
            let clamped = seconds.clamp(1, 3600);
            app.editor.project_edit_task_board_settings.auto_promote_tick_interval_seconds = clamped;
            app.editor.project_edit_task_board_auto_promote_tick_interval_seconds_input =
                clamped.to_string();
            Vec::new()
        }
        Event::ProjectEditFailedRetryMinutesChanged(minutes) => {
            let clamped = minutes.clamp(1, 1440);
            app.editor.project_edit_task_board_settings.failed_retry_minutes = clamped;
            app.editor.project_edit_failed_retry_minutes_input = clamped.to_string();
            Vec::new()
        }
        Event::ProjectEditFailedRetryMaxAttemptsChanged(attempts) => {
            let clamped = attempts.clamp(0, 10);
            app.editor.project_edit_task_board_settings.failed_retry_max_attempts = clamped;
            app.editor.project_edit_failed_retry_max_attempts_input = clamped.to_string();
            Vec::new()
        }
        Event::ProjectEditRunningTimeoutMinutesChanged(minutes) => {
            let clamped = minutes.clamp(1, 1440);
            app.editor.project_edit_task_board_settings.running_timeout_minutes = clamped;
            app.editor.project_edit_running_timeout_minutes_input = clamped.to_string();
            Vec::new()
        }
        Event::ProjectEditPrSubmittedStallTimeoutSecondsChanged(seconds) => {
            let clamped = seconds.clamp(5, 3600);
            app.editor.project_edit_task_board_settings.pr_submitted_stall_timeout_seconds = clamped;
            app.editor.project_edit_pr_submitted_stall_timeout_seconds_input = clamped.to_string();
            Vec::new()
        }
        Event::ProjectEditRecycleWorktreeOnTaskFinishToggled(enabled) => {
            app.editor.project_edit_task_board_settings.recycle_worktree_on_task_finish = enabled;
            Vec::new()
        }
        Event::ProjectEditWorkflowIdChanged(workflow_id) => {
            app.editor.project_edit_task_board_settings.workflow_id =
                if desktop_task_model::task_workflow_by_id(&workflow_id).is_some() {
                    workflow_id
                } else {
                    desktop_task_model::DEFAULT_TASK_WORKFLOW_ID.to_string()
                };
            Vec::new()
        }
        Event::ProjectEditSaved => super::edit_save::save(app),
        Event::ProjectEditRuntimeSaved(res) => {
            if let Err(err) = res {
                return vec![Effect::RuntimeError(err)];
            }
            Vec::new()
        }
        Event::ProjectEditCanceled => {
            app.editor.project_edit_path = None;
            app.editor.project_edit_tab = ProjectEditTab::General;
            app.editor.project_edit_name.clear();
            app.editor.project_edit_icon.clear();
            app.editor.project_edit_icon_hovered = false;
            app.editor.project_edit_icon_color.clear();
            app.editor.project_edit_icon_color_picker_open = false;
            app.editor.project_edit_icon_color_format =
                desktop_ui_controls::color_picker::ColorFormat::Hex;
            app.editor.project_edit_start_script.clear();
            app.editor.project_edit_start_script_editor = iced::widget::text_editor::Content::new();
            app.editor.project_edit_worktree_enabled = false;
            app.editor.project_edit_task_board_settings = desktop_task_model::TaskBoardSettings::new();
            app.editor.project_edit_max_concurrent_input.clear();
            app.editor.project_edit_task_board_auto_refresh = true;
            app.editor.project_edit_session_auto_refresh = default_recent_project_session_auto_refresh();
            app.editor.project_edit_session_refresh_interval_seconds_input.clear();
            app.editor.project_edit_task_board_refresh_interval_seconds_input.clear();
            app.editor.project_edit_task_board_scheduler_tick_interval_seconds_input.clear();
            app.editor.project_edit_task_board_auto_promote_tick_interval_seconds_input.clear();
            app.editor.project_edit_failed_retry_minutes_input.clear();
            app.editor.project_edit_failed_retry_max_attempts_input.clear();
            app.editor.project_edit_running_timeout_minutes_input.clear();
            app.editor.project_edit_pr_submitted_stall_timeout_seconds_input.clear();
            Vec::new()
        }
        Event::RecentRevealPressed(_path) => {
            (*app.work_current_project_menu) = None;
            vec![Effect::Reveal(_path)]
        }
        Event::RecentRemovePressed(path) => {
            (*app.project_tools_menu_path) = None;
            (*app.work_current_project_menu) = None;
            (*app.recent_project_remove_confirm_path) = Some(path);
            Vec::new()
        }
        Event::RecentRemoveCanceled => {
            (*app.recent_project_remove_confirm_path) = None;
            Vec::new()
        }
        Event::RecentRemoveConfirmed(path) => {
            let deleting_current = (*app.project_path).as_ref() == Some(&path);
            remove_recent_project(app, &path);
            (*app.recent_project_remove_confirm_path) = None;
            if deleting_current {
                vec![Effect::SaveRecent, Effect::OpenFirst]
            } else {
                vec![Effect::SaveRecent]
            }
        }

    }
}

fn remove_recent_project(app: &mut State<'_>, path: &str) {
    (*app.recent_projects).retain(|p| p != path);


    if (*app.hovered_recent_project).as_deref() == Some(path) {
        (*app.hovered_recent_project) = None;
    }
    if (*app.project_tools_menu_path).as_deref() == Some(path) {
        (*app.project_tools_menu_path) = None;
    }
    if app.editor.project_edit_path.as_deref() == Some(path) {
        app.editor.project_edit_path = None;
        app.editor.project_edit_name.clear();
        app.editor.project_edit_icon.clear();
        app.editor.project_edit_icon_hovered = false;
        app.editor.project_edit_icon_color.clear();
        app.editor.project_edit_icon_color_picker_open = false;
        app.editor.project_edit_icon_color_format = desktop_ui_controls::color_picker::ColorFormat::Hex;
        app.editor.project_edit_start_script.clear();
        app.editor.project_edit_start_script_editor = iced::widget::text_editor::Content::new();
        app.editor.project_edit_worktree_enabled = false;
        app.editor.project_edit_task_board_settings = desktop_task_model::TaskBoardSettings::new();
        app.editor.project_edit_max_concurrent_input.clear();
        app.editor.project_edit_task_board_auto_refresh = true;
        app.editor.project_edit_session_auto_refresh = default_recent_project_session_auto_refresh();
        app.editor.project_edit_session_refresh_interval_seconds_input.clear();
        app.editor.project_edit_task_board_refresh_interval_seconds_input.clear();
        app.editor.project_edit_task_board_scheduler_tick_interval_seconds_input.clear();
        app.editor.project_edit_task_board_auto_promote_tick_interval_seconds_input.clear();
        app.editor.project_edit_failed_retry_minutes_input.clear();
        app.editor.project_edit_failed_retry_max_attempts_input.clear();
        app.editor.project_edit_running_timeout_minutes_input.clear();
    }

    (*app.recent_projects_edits) = (*app.recent_projects)
        .iter()
        .map(|p| {
            if let Some(m) = (*app.recent_projects_meta).iter().find(|m| &m.path == p) {
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

#[cfg(test)]
#[path = "edit_tests.rs"]
mod edit_tests;

use super::edit::{Effect, State, ProjectEditTab, default_recent_project_session_auto_refresh, RecentProjectMeta};
use super::session_common::{parse_clamped_u32, parse_clamped_u64, trim_to_option};
pub(super) fn save(app: &mut State<'_>) -> Vec<Effect> {
    let Some(path) = app.editor.project_edit_path.clone() else {
        return Vec::new();
    };
    let name = app.editor.project_edit_name.trim();
    let final_name = if name.is_empty() {
        std::path::Path::new(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&path)
            .to_string()
    } else {
        name.to_string()
    };
    let mut task_board_settings = app.editor.project_edit_task_board_settings.clone();
    task_board_settings.auto_refresh = app.editor.project_edit_task_board_auto_refresh;
    task_board_settings.max_concurrent = parse_clamped_u32(
        &app.editor.project_edit_max_concurrent_input,
        task_board_settings.max_concurrent,
        1,
        10,
    );
    let session_auto_refresh = app.editor.project_edit_session_auto_refresh;
    let session_refresh_interval_seconds = parse_clamped_u64(
        &app.editor.project_edit_session_refresh_interval_seconds_input,
        60,
        1,
        3600,
    );
    task_board_settings.refresh_interval_seconds = parse_clamped_u64(
        &app.editor.project_edit_task_board_refresh_interval_seconds_input,
        task_board_settings.refresh_interval_seconds,
        1,
        3600,
    );
    task_board_settings.scheduler_tick_interval_seconds = parse_clamped_u64(
        &app.editor.project_edit_task_board_scheduler_tick_interval_seconds_input,
        task_board_settings.scheduler_tick_interval_seconds,
        1,
        60,
    );
    task_board_settings.auto_promote_tick_interval_seconds = parse_clamped_u64(
        &app.editor.project_edit_task_board_auto_promote_tick_interval_seconds_input,
        task_board_settings.auto_promote_tick_interval_seconds,
        1,
        3600,
    );
    task_board_settings.failed_retry_minutes = parse_clamped_u32(
        &app.editor.project_edit_failed_retry_minutes_input,
        task_board_settings.failed_retry_minutes,
        1,
        1440,
    );
    task_board_settings.failed_retry_max_attempts = parse_clamped_u32(
        &app.editor.project_edit_failed_retry_max_attempts_input,
        task_board_settings.failed_retry_max_attempts,
        0,
        10,
    );
    task_board_settings.running_timeout_minutes = parse_clamped_u32(
        &app.editor.project_edit_running_timeout_minutes_input,
        task_board_settings.running_timeout_minutes,
        1,
        1440,
    );
    task_board_settings.pr_submitted_stall_timeout_seconds = parse_clamped_u32(
        &app.editor.project_edit_pr_submitted_stall_timeout_seconds_input,
        task_board_settings.pr_submitted_stall_timeout_seconds,
        5,
        3600,
    );
    task_board_settings = task_board_settings.sanitized();
    let icon = trim_to_option(app.editor.project_edit_icon.clone());
    let icon_color = trim_to_option(app.editor.project_edit_icon_color.clone());
    let start_command = trim_to_option(app.editor.project_edit_start_script.clone());
    let final_name_for_runtime = final_name.clone();

    let enabled = app.editor.project_edit_worktree_enabled;
    if let Some(idx) = (*app.recent_projects).iter().position(|p| p == &path)
        && idx < (*app.recent_projects_edits).len()
    {
        (*app.recent_projects_edits)[idx] = final_name.clone();
    }
    if let Some(meta) = (*app.recent_projects_meta).iter_mut().find(|meta| meta.path == path) {
        meta.name = final_name;
        meta.task_board_settings = Some(task_board_settings.clone());
        meta.session_auto_refresh = session_auto_refresh;
        meta.session_refresh_interval_seconds = session_refresh_interval_seconds;
        meta.icon = icon.clone();
        meta.icon_color = icon_color.clone();
        meta.worktree_start_command = start_command.clone();
    } else {
        (*app.recent_projects_meta).push(RecentProjectMeta {
            path: path.clone(),
            name: final_name,
            task_board_settings: Some(task_board_settings.clone()),
            session_auto_refresh,
            session_refresh_interval_seconds,
            icon: icon.clone(),
            icon_color: icon_color.clone(),
            worktree_start_command: start_command.clone(),
        });
    }

    if (*app.project_path).as_ref() == Some(&path) {
        (*app.task_board_settings) = task_board_settings;
    }

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
    (*app.project_tools_menu_path) = None;
    vec![Effect::SaveWorktree(path.clone(), enabled), Effect::SaveMeta,
        Effect::RuntimeUpdate { path, name: final_name_for_runtime, icon, icon_color, start_command }]
}

//! 处理项目会话编辑操作，维护会话标题、说明和配置字段。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use super::common::{
    parse_clamped_u32, parse_clamped_u64, save_project_worktree_enabled_task, trim_to_option,
};
use crate::app::message::project::ProjectMessage;
use crate::app::projects::{save_recent_projects_background, save_recent_projects_meta_background};
use crate::app::{
    App, Message,
    state::{
        ProjectEditTab, RecentProjectMeta, default_recent_project_session_auto_refresh,
        default_recent_project_session_refresh_interval_seconds,
    },
};

/// handle 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub(crate) fn handle(app: &mut App, message: ProjectMessage) -> Option<iced::Task<Message>> {
    // 所有界面事件在一个入口显式匹配，方便审计状态变更和异步任务边界。
    match message {
        ProjectMessage::ProjectToolsMenuToggled(path) => {
            if app.project_tools_menu_path.as_ref() == Some(&path) {
                app.project_tools_menu_path = None;
            } else {
                app.project_tools_menu_path = Some(path);
            }
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectToolsMenuClosed => {
            app.project_tools_menu_path = None;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditOpened(path) => {
            let fallback_name = std::path::Path::new(&path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&path)
                .to_string();
            let edited_name = app
                .recent_projects
                .iter()
                .position(|p| p == &path)
                .and_then(|idx| app.recent_projects_edits.get(idx).cloned())
                .or_else(|| {
                    app.recent_projects_meta
                        .iter()
                        .find(|meta| meta.path == path)
                        .map(|meta| meta.name.clone())
                })
                .unwrap_or(fallback_name);
            let task_board_settings = app
                .recent_projects_meta
                .iter()
                .find(|meta| meta.path == path)
                .and_then(|meta| meta.task_board_settings.clone())
                .or_else(|| {
                    if app.project_path.as_ref() == Some(&path) {
                        Some(app.task_board_settings.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(crate::app::task::TaskBoardSettings::new);

            app.project_edit_path = Some(path.clone());
            app.project_edit_tab = ProjectEditTab::General;
            app.project_edit_name = edited_name;
            let edited_icon = app
                .recent_projects_meta
                .iter()
                .find(|meta| meta.path == path)
                .and_then(|meta| meta.icon.clone())
                .unwrap_or_default();
            let edited_icon_color = app
                .recent_projects_meta
                .iter()
                .find(|meta| meta.path == path)
                .and_then(|meta| meta.icon_color.clone())
                .unwrap_or_default();
            let edited_worktree_start_command = app
                .recent_projects_meta
                .iter()
                .find(|meta| meta.path == path)
                .and_then(|meta| meta.worktree_start_command.clone())
                .unwrap_or_default();
            app.project_edit_icon = edited_icon;
            app.project_edit_icon_hovered = false;
            app.project_edit_icon_color = edited_icon_color;
            app.project_edit_icon_color_picker_open = false;
            app.project_edit_icon_color_format =
                crate::app::components::color_picker::ColorFormat::Hex;
            app.project_edit_start_script = edited_worktree_start_command;
            app.project_edit_start_script_editor =
                iced::widget::text_editor::Content::with_text(&app.project_edit_start_script);
            app.project_edit_worktree_enabled =
                app.project_worktree_enabled.get(&path).copied().unwrap_or(false);
            app.project_edit_task_board_settings = task_board_settings.sanitized();
            let (session_auto_refresh, session_refresh_interval_seconds) = app
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
            app.project_edit_max_concurrent_input =
                app.project_edit_task_board_settings.max_concurrent.clamp(1, 10).to_string();
            app.project_edit_task_board_auto_refresh =
                app.project_edit_task_board_settings.auto_refresh;
            app.project_edit_session_auto_refresh = session_auto_refresh;
            app.project_edit_session_refresh_interval_seconds_input =
                session_refresh_interval_seconds.to_string();
            app.project_edit_task_board_refresh_interval_seconds_input = app
                .project_edit_task_board_settings
                .refresh_interval_seconds
                .clamp(1, 3600)
                .to_string();
            app.project_edit_task_board_scheduler_tick_interval_seconds_input = app
                .project_edit_task_board_settings
                .scheduler_tick_interval_seconds
                .clamp(1, 60)
                .to_string();
            app.project_edit_task_board_auto_promote_tick_interval_seconds_input = app
                .project_edit_task_board_settings
                .auto_promote_tick_interval_seconds
                .clamp(1, 3600)
                .to_string();
            app.project_edit_failed_retry_minutes_input = app
                .project_edit_task_board_settings
                .failed_retry_minutes
                .clamp(1, 1440)
                .to_string();
            app.project_edit_failed_retry_max_attempts_input = app
                .project_edit_task_board_settings
                .failed_retry_max_attempts
                .clamp(0, 10)
                .to_string();
            app.project_edit_running_timeout_minutes_input = app
                .project_edit_task_board_settings
                .running_timeout_minutes
                .clamp(1, 1440)
                .to_string();
            app.project_edit_pr_submitted_stall_timeout_seconds_input = app
                .project_edit_task_board_settings
                .pr_submitted_stall_timeout_seconds
                .clamp(5, 3600)
                .to_string();
            app.project_tools_menu_path = None;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditTabSelected(tab) => {
            app.project_edit_tab = tab;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditNameChanged(name) => {
            app.project_edit_name = name;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditIconChanged(icon) => {
            app.project_edit_icon = icon;
            app.project_edit_icon_hovered = false;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditIconHovered(hovered) => {
            app.project_edit_icon_hovered = hovered;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditIconPickFile => Some(iced::Task::perform(
            async {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let handle = rfd::AsyncFileDialog::new()
                        .add_filter("Image", &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
                        .pick_file()
                        .await;
                    handle.map(|f| f.path().to_string_lossy().to_string())
                }
                #[cfg(target_arch = "wasm32")]
                {
                    None
                }
            },
            |picked| {
                Message::Project(
                    crate::app::message::project::ProjectMessage::ProjectEditIconFilePicked(picked),
                )
            },
        )),
        ProjectMessage::ProjectEditIconFilePicked(picked) => {
            if let Some(path) = picked {
                let trimmed = path.trim();
                if !trimmed.is_empty() {
                    app.project_edit_icon = trimmed.to_string();
                    app.project_edit_icon_hovered = false;
                }
            }
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditIconColorChanged(color) => {
            app.project_edit_icon_color = color;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditIconColorPresetSelected(color) => {
            app.project_edit_icon_color = color;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditIconColorPickerToggled => {
            app.project_edit_icon_color_picker_open = !app.project_edit_icon_color_picker_open;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditIconColorPickerClosed => {
            app.project_edit_icon_color_picker_open = false;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditIconColorFormatChanged(format) => {
            app.project_edit_icon_color_format = format;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditStartScriptChanged(command) => {
            app.project_edit_start_script = command;
            app.project_edit_start_script_editor =
                iced::widget::text_editor::Content::with_text(&app.project_edit_start_script);
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditStartScriptEditorAction(action) => {
            app.project_edit_start_script_editor.perform(action);
            app.project_edit_start_script =
                app.project_edit_start_script_editor.text().trim_end_matches('\n').to_string();
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditWorktreeToggled(enabled) => {
            app.project_edit_worktree_enabled = enabled;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditTaskBoardAutoRefreshToggled(enabled) => {
            app.project_edit_task_board_auto_refresh = enabled;
            app.project_edit_task_board_settings.auto_refresh = enabled;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditSessionAutoRefreshToggled(enabled) => {
            app.project_edit_session_auto_refresh = enabled;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditMaxConcurrentInputChanged(value) => {
            app.project_edit_max_concurrent_input = value;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditSessionRefreshIntervalSecondsInputChanged(value) => {
            app.project_edit_session_refresh_interval_seconds_input = value;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditTaskBoardRefreshIntervalSecondsInputChanged(value) => {
            app.project_edit_task_board_refresh_interval_seconds_input = value;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditTaskBoardSchedulerTickIntervalSecondsInputChanged(value) => {
            app.project_edit_task_board_scheduler_tick_interval_seconds_input = value;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsInputChanged(value) => {
            app.project_edit_task_board_auto_promote_tick_interval_seconds_input = value;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditFailedRetryMinutesInputChanged(value) => {
            app.project_edit_failed_retry_minutes_input = value;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditFailedRetryMaxAttemptsInputChanged(value) => {
            app.project_edit_failed_retry_max_attempts_input = value;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditRunningTimeoutMinutesInputChanged(value) => {
            app.project_edit_running_timeout_minutes_input = value;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditPrSubmittedStallTimeoutSecondsInputChanged(value) => {
            app.project_edit_pr_submitted_stall_timeout_seconds_input = value;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditMaxConcurrentChanged(count) => {
            let clamped = count.clamp(1, 10);
            app.project_edit_task_board_settings.max_concurrent = clamped;
            app.project_edit_max_concurrent_input = clamped.to_string();
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditSessionRefreshIntervalSecondsChanged(seconds) => {
            let clamped = seconds.clamp(1, 3600);
            app.project_edit_session_refresh_interval_seconds_input = clamped.to_string();
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditTaskBoardRefreshIntervalSecondsChanged(seconds) => {
            let clamped = seconds.clamp(1, 3600);
            app.project_edit_task_board_settings.refresh_interval_seconds = clamped;
            app.project_edit_task_board_refresh_interval_seconds_input = clamped.to_string();
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditTaskBoardSchedulerTickIntervalSecondsChanged(seconds) => {
            let clamped = seconds.clamp(1, 60);
            app.project_edit_task_board_settings.scheduler_tick_interval_seconds = clamped;
            app.project_edit_task_board_scheduler_tick_interval_seconds_input = clamped.to_string();
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditTaskBoardAutoPromoteTickIntervalSecondsChanged(seconds) => {
            let clamped = seconds.clamp(1, 3600);
            app.project_edit_task_board_settings.auto_promote_tick_interval_seconds = clamped;
            app.project_edit_task_board_auto_promote_tick_interval_seconds_input =
                clamped.to_string();
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditFailedRetryMinutesChanged(minutes) => {
            let clamped = minutes.clamp(1, 1440);
            app.project_edit_task_board_settings.failed_retry_minutes = clamped;
            app.project_edit_failed_retry_minutes_input = clamped.to_string();
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditFailedRetryMaxAttemptsChanged(attempts) => {
            let clamped = attempts.clamp(0, 10);
            app.project_edit_task_board_settings.failed_retry_max_attempts = clamped;
            app.project_edit_failed_retry_max_attempts_input = clamped.to_string();
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditRunningTimeoutMinutesChanged(minutes) => {
            let clamped = minutes.clamp(1, 1440);
            app.project_edit_task_board_settings.running_timeout_minutes = clamped;
            app.project_edit_running_timeout_minutes_input = clamped.to_string();
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditPrSubmittedStallTimeoutSecondsChanged(seconds) => {
            let clamped = seconds.clamp(5, 3600);
            app.project_edit_task_board_settings.pr_submitted_stall_timeout_seconds = clamped;
            app.project_edit_pr_submitted_stall_timeout_seconds_input = clamped.to_string();
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditRecycleWorktreeOnTaskFinishToggled(enabled) => {
            app.project_edit_task_board_settings.recycle_worktree_on_task_finish = enabled;
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditWorkflowIdChanged(workflow_id) => {
            app.project_edit_task_board_settings.workflow_id =
                if crate::app::task::task_workflow_by_id(&workflow_id).is_some() {
                    workflow_id
                } else {
                    crate::app::task::DEFAULT_TASK_WORKFLOW_ID.to_string()
                };
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditSaved => {
            let Some(path) = app.project_edit_path.clone() else {
                return Some(iced::Task::none());
            };
            let name = app.project_edit_name.trim();
            let final_name = if name.is_empty() {
                std::path::Path::new(&path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&path)
                    .to_string()
            } else {
                name.to_string()
            };
            let mut task_board_settings = app.project_edit_task_board_settings.clone();
            task_board_settings.auto_refresh = app.project_edit_task_board_auto_refresh;
            task_board_settings.max_concurrent = parse_clamped_u32(
                &app.project_edit_max_concurrent_input,
                task_board_settings.max_concurrent,
                1,
                10,
            );
            let session_auto_refresh = app.project_edit_session_auto_refresh;
            let session_refresh_interval_seconds = parse_clamped_u64(
                &app.project_edit_session_refresh_interval_seconds_input,
                60,
                1,
                3600,
            );
            task_board_settings.refresh_interval_seconds = parse_clamped_u64(
                &app.project_edit_task_board_refresh_interval_seconds_input,
                task_board_settings.refresh_interval_seconds,
                1,
                3600,
            );
            task_board_settings.scheduler_tick_interval_seconds = parse_clamped_u64(
                &app.project_edit_task_board_scheduler_tick_interval_seconds_input,
                task_board_settings.scheduler_tick_interval_seconds,
                1,
                60,
            );
            task_board_settings.auto_promote_tick_interval_seconds = parse_clamped_u64(
                &app.project_edit_task_board_auto_promote_tick_interval_seconds_input,
                task_board_settings.auto_promote_tick_interval_seconds,
                1,
                3600,
            );
            task_board_settings.failed_retry_minutes = parse_clamped_u32(
                &app.project_edit_failed_retry_minutes_input,
                task_board_settings.failed_retry_minutes,
                1,
                1440,
            );
            task_board_settings.failed_retry_max_attempts = parse_clamped_u32(
                &app.project_edit_failed_retry_max_attempts_input,
                task_board_settings.failed_retry_max_attempts,
                0,
                10,
            );
            task_board_settings.running_timeout_minutes = parse_clamped_u32(
                &app.project_edit_running_timeout_minutes_input,
                task_board_settings.running_timeout_minutes,
                1,
                1440,
            );
            task_board_settings.pr_submitted_stall_timeout_seconds = parse_clamped_u32(
                &app.project_edit_pr_submitted_stall_timeout_seconds_input,
                task_board_settings.pr_submitted_stall_timeout_seconds,
                5,
                3600,
            );
            task_board_settings = task_board_settings.sanitized();
            let icon = trim_to_option(app.project_edit_icon.clone());
            let icon_color = trim_to_option(app.project_edit_icon_color.clone());
            let start_command = trim_to_option(app.project_edit_start_script.clone());
            let final_name_for_runtime = final_name.clone();

            if app.project_edit_worktree_enabled {
                app.project_worktree_enabled.insert(path.clone(), true);
            } else {
                app.project_worktree_enabled.remove(&path);
            }
            let project_worktree_enabled = app.project_worktree_enabled.clone();
            let save_project_worktree_task =
                save_project_worktree_enabled_task(project_worktree_enabled);

            if let Some(idx) = app.recent_projects.iter().position(|p| p == &path)
                && idx < app.recent_projects_edits.len()
            {
                app.recent_projects_edits[idx] = final_name.clone();
            }
            if let Some(meta) = app.recent_projects_meta.iter_mut().find(|meta| meta.path == path) {
                meta.name = final_name;
                meta.task_board_settings = Some(task_board_settings.clone());
                meta.session_auto_refresh = session_auto_refresh;
                meta.session_refresh_interval_seconds = session_refresh_interval_seconds;
                meta.icon = icon.clone();
                meta.icon_color = icon_color.clone();
                meta.worktree_start_command = start_command.clone();
            } else {
                app.recent_projects_meta.push(RecentProjectMeta {
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
            save_recent_projects_meta_background(app.recent_projects_meta.clone());
            if app.project_path.as_ref() == Some(&path) {
                app.task_board_settings = task_board_settings;
            }

            app.project_edit_path = None;
            app.project_edit_tab = ProjectEditTab::General;
            app.project_edit_name.clear();
            app.project_edit_icon.clear();
            app.project_edit_icon_hovered = false;
            app.project_edit_icon_color.clear();
            app.project_edit_icon_color_picker_open = false;
            app.project_edit_icon_color_format =
                crate::app::components::color_picker::ColorFormat::Hex;
            app.project_edit_start_script.clear();
            app.project_edit_start_script_editor = iced::widget::text_editor::Content::new();
            app.project_edit_worktree_enabled = false;
            app.project_edit_task_board_settings = crate::app::task::TaskBoardSettings::new();
            app.project_edit_max_concurrent_input.clear();
            app.project_edit_task_board_auto_refresh = true;
            app.project_edit_session_auto_refresh = default_recent_project_session_auto_refresh();
            app.project_edit_session_refresh_interval_seconds_input.clear();
            app.project_edit_task_board_refresh_interval_seconds_input.clear();
            app.project_edit_task_board_scheduler_tick_interval_seconds_input.clear();
            app.project_edit_task_board_auto_promote_tick_interval_seconds_input.clear();
            app.project_edit_failed_retry_minutes_input.clear();
            app.project_edit_failed_retry_max_attempts_input.clear();
            app.project_edit_running_timeout_minutes_input.clear();
            app.project_edit_pr_submitted_stall_timeout_seconds_input.clear();
            app.project_tools_menu_path = None;
            Some(iced::Task::batch(vec![
                save_project_worktree_task,
                iced::Task::perform(
                    async move {
                        let client = crate::app::gateway_client()?;
                        let project =
                            crate::app::message::project::helpers::resolve_gateway_project(&path)
                                .await?;
                        let scope =
                            crate::app::session_gateway::active_project_scope(project.id.clone())?;
                        client
                            .project_update(
                                &scope,
                                &vw_gateway_client::vw_api_types::project::UpdateProjectRequest {
                                    name: Some(final_name_for_runtime),
                                    active_worktree_id: None,
                                    icon: Some(
                                        vw_gateway_client::vw_api_types::project::IconUpdateDto {
                                            override_icon: icon,
                                            color: icon_color,
                                        },
                                    ),
                                    commands: Some(
                                        vw_gateway_client::vw_api_types::project::CommandsUpdateDto {
                                            start: start_command,
                                        },
                                    ),
                                },
                            )
                            .await
                            .map(|_| ())
                    },
                    |res| Message::Project(ProjectMessage::ProjectEditRuntimeSaved(res)),
                ),
            ]))
        }
        ProjectMessage::ProjectEditRuntimeSaved(res) => {
            if let Err(err) = res {
                app.error_message = Some(format!("保存项目扩展配置失败: {}", err));
            }
            Some(iced::Task::none())
        }
        ProjectMessage::ProjectEditCanceled => {
            app.project_edit_path = None;
            app.project_edit_tab = ProjectEditTab::General;
            app.project_edit_name.clear();
            app.project_edit_icon.clear();
            app.project_edit_icon_hovered = false;
            app.project_edit_icon_color.clear();
            app.project_edit_icon_color_picker_open = false;
            app.project_edit_icon_color_format =
                crate::app::components::color_picker::ColorFormat::Hex;
            app.project_edit_start_script.clear();
            app.project_edit_start_script_editor = iced::widget::text_editor::Content::new();
            app.project_edit_worktree_enabled = false;
            app.project_edit_task_board_settings = crate::app::task::TaskBoardSettings::new();
            app.project_edit_max_concurrent_input.clear();
            app.project_edit_task_board_auto_refresh = true;
            app.project_edit_session_auto_refresh = default_recent_project_session_auto_refresh();
            app.project_edit_session_refresh_interval_seconds_input.clear();
            app.project_edit_task_board_refresh_interval_seconds_input.clear();
            app.project_edit_task_board_scheduler_tick_interval_seconds_input.clear();
            app.project_edit_task_board_auto_promote_tick_interval_seconds_input.clear();
            app.project_edit_failed_retry_minutes_input.clear();
            app.project_edit_failed_retry_max_attempts_input.clear();
            app.project_edit_running_timeout_minutes_input.clear();
            app.project_edit_pr_submitted_stall_timeout_seconds_input.clear();
            Some(iced::Task::none())
        }
        ProjectMessage::RecentRevealPressed(_path) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = open::that(&_path);
            }
            Some(iced::Task::none())
        }
        ProjectMessage::RecentRemovePressed(path) => {
            app.recent_projects.retain(|p| p != &path);
            save_recent_projects_background(app.recent_projects.clone());

            if app.hovered_recent_project.as_ref() == Some(&path) {
                app.hovered_recent_project = None;
            }
            if app.project_tools_menu_path.as_ref() == Some(&path) {
                app.project_tools_menu_path = None;
            }
            if app.project_edit_path.as_ref() == Some(&path) {
                app.project_edit_path = None;
                app.project_edit_name.clear();
                app.project_edit_icon.clear();
                app.project_edit_icon_hovered = false;
                app.project_edit_icon_color.clear();
                app.project_edit_icon_color_picker_open = false;
                app.project_edit_icon_color_format =
                    crate::app::components::color_picker::ColorFormat::Hex;
                app.project_edit_start_script.clear();
                app.project_edit_start_script_editor = iced::widget::text_editor::Content::new();
                app.project_edit_worktree_enabled = false;
                app.project_edit_task_board_settings = crate::app::task::TaskBoardSettings::new();
                app.project_edit_max_concurrent_input.clear();
                app.project_edit_task_board_auto_refresh = true;
                app.project_edit_session_auto_refresh =
                    default_recent_project_session_auto_refresh();
                app.project_edit_session_refresh_interval_seconds_input.clear();
                app.project_edit_task_board_refresh_interval_seconds_input.clear();
                app.project_edit_task_board_scheduler_tick_interval_seconds_input.clear();
                app.project_edit_task_board_auto_promote_tick_interval_seconds_input.clear();
                app.project_edit_failed_retry_minutes_input.clear();
                app.project_edit_failed_retry_max_attempts_input.clear();
                app.project_edit_running_timeout_minutes_input.clear();
            }

            app.recent_projects_edits = app
                .recent_projects
                .iter()
                .map(|p| {
                    if let Some(m) = app.recent_projects_meta.iter().find(|m| &m.path == p) {
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

            Some(iced::Task::none())
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "edit_tests.rs"]
mod edit_tests;

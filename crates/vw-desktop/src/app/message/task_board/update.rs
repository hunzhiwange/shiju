//! 任务看板桌面适配：委派局部更新，映射会话、通知、持久化及原生操作效果。

use crate::app::Message;
use crate::app::state::MAIN_AGENT_KEY;
use crate::app::task::workflow::task_workflow_by_id;
use desktop_task_board as taskboard;
use taskboard::update::projection::*;

use super::helpers::*;
use super::*;

macro_rules! dispatch_task_board_messages {
    ($message:expr, $($pattern:pat => $body:block)*) => {
        match $message {
            $($pattern => $body,)*
            _ => iced::Task::none(),
        }
    };
}

#[path = "update/execution_results.rs"]
mod execution_results_update;
#[path = "update/execution.rs"]
mod execution_update;

fn task_board_project_matches(app: &crate::app::App, project_path: &str) -> bool {
    taskboard::update::projection::task_board_project_matches(
        app.project_path.as_deref(),
        project_path,
    )
}

fn initial_task_status_for_app(app: &crate::app::App) -> TaskStatus {
    initial_task_status_for_workflow_id(&app.task_board_settings.workflow_id)
}

fn sync_agent_scheduler_status_for_current_project(
    app: &crate::app::App,
) -> iced::Task<crate::app::Message> {
    app.project_path.clone().map(sync_agent_scheduler_status).unwrap_or_else(iced::Task::none)
}

fn agent_scheduler_gateway_unavailable(error: &str) -> bool {
    gateway_http::is_gateway_unavailable_error(error)
}

fn persist_task_metadata_patch_task(
    app: &crate::app::App,
    task: Task,
    action_label: &'static str,
) -> iced::Task<crate::app::Message> {
    let Some(project_path) = app.project_path.clone() else {
        return iced::Task::none();
    };
    let workflow_id = app.task_board_settings.workflow_id.clone();
    iced::Task::perform(
        async move {
            persist_updated_tasks_via_gateway(&project_path, &[task], action_label, &workflow_id)
                .await
        },
        |result| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(result)),
    )
}

fn apply_agent_scheduler_status(
    app: &mut crate::app::App,
    response: vw_api_agent_scheduler::AgentSchedulerTaskBoardStatusResponse,
) -> Vec<Task> {
    let (updated_tasks, updated_task_ids) =
        taskboard::update::projection::apply_agent_scheduler_status(
            &mut app.task_board_tasks,
            &app.task_board_tasks_pending_persistence,
            &mut app.task_board_executor,
            response,
        );
    app.task_board_loading = false;

    if !updated_tasks.is_empty() {
        prune_bulk_selection(app);
        for task_id in updated_task_ids {
            sync_viewing_logs(app, &task_id);
        }
    }

    updated_tasks
}

fn apply_agent_scheduler_logs(
    app: &mut crate::app::App,
    responses: Vec<vw_api_agent_scheduler::AgentSchedulerTaskLogsResponse>,
) -> Vec<Task> {
    let (updated_tasks, updated_task_ids) =
        taskboard::update::projection::apply_agent_scheduler_logs(
            &mut app.task_board_tasks,
            responses,
        );

    for task_id in updated_task_ids {
        sync_viewing_logs(app, &task_id);
    }

    updated_tasks
}

fn schedule_agent_scheduler_status_followup_ticks(
    app: &mut crate::app::App,
    updated_tasks: &[Task],
) -> iced::Task<crate::app::Message> {
    if !app.task_board_executor_running {
        return iced::Task::none();
    }

    let (should_schedule_review, should_schedule_merge) =
        taskboard::update::projection::scheduler_followup_ticks(
            &app.task_board_settings.workflow_id,
            updated_tasks,
        );
    let mut tasks = Vec::new();
    if should_schedule_review {
        tasks.push(schedule_auto_review_tick_with_deadline(app));
    }
    if should_schedule_merge {
        tasks.push(schedule_auto_merge_tick_with_deadline(app));
    }
    iced::Task::batch(tasks)
}

#[cfg(test)]
#[path = "update_tests.rs"]
mod update_tests;

/// 执行 update 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn update(
    app: &mut crate::app::App,
    message: TaskBoardMessage,
) -> iced::Task<crate::app::Message> {
    let model_options = taskboard::update::model::reasoning_model_for_message(&message)
        .map(|model| {
            crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(
                app, &model,
            )
        })
        .unwrap_or_default();
    let message = match taskboard::update::model::update_model(
        taskboard::update::model::ModelState {
            bulk_acp_agent: &mut app.task_board_bulk_acp_agent,
            bulk_model_input: &mut app.task_board_bulk_model_input,
            bulk_model_popover: &mut app.task_board_bulk_model_popover,
            bulk_reasoning_effort: &mut app.task_board_bulk_reasoning_effort,
            draft: &mut app.task_board_draft,
            last_model: &mut app.task_board_last_model,
            model_popover: &mut app.task_board_model_popover,
        },
        message,
        &model_options,
        !app.model_settings.loading && app.model_settings.providers.is_empty(),
    ) {
        Ok(effect) => {
            if effect.clear_hover {
                app.model_popover_hover = None;
            }
            if effect.refresh_models {
                return iced::Task::done(Message::Settings(
                    crate::app::message::SettingsMessage::ModelsRefresh,
                ));
            }
            return iced::Task::none();
        }
        Err(message) => message,
    };

    let message = match taskboard::update::logs::update_logs(
        taskboard::helpers::logs::TaskLogsState {
            task_board_logs_editor: &mut app.task_board_logs_editor,
            task_board_logs_auto_scroll: &mut app.task_board_logs_auto_scroll,
            task_board_logs_scroll_top_line: &mut app.task_board_logs_scroll_top_line,
            task_board_logs_scroll_remainder: &mut app.task_board_logs_scroll_remainder,
            task_board_logs_viewport_height: &mut app.task_board_logs_viewport_height,
            current_line_height: &mut app.current_line_height,
            desktop_locale: &mut app.desktop_locale,
            task_board_log_cache: &mut app.task_board_log_cache,
            task_board_tasks: &mut app.task_board_tasks,
            task_board_viewing_logs: &mut app.task_board_viewing_logs,
            task_board_logs_context_menu_open: &mut app.task_board_logs_context_menu_open,
            task_board_logs_context_menu_pos: &mut app.task_board_logs_context_menu_pos,
        },
        message,
    ) {
        Ok(()) => return iced::Task::none(),
        Err(message) => message,
    };

    let message = match taskboard::update::settings::update_settings(
        taskboard::update::settings::SettingsState {
            settings: &mut app.task_board_settings,
            worktree_desired_capacity: &mut app.task_board_worktree_desired_capacity,
            next_refresh_at_ms: &mut app.task_board_next_refresh_at_ms,
            next_scheduler_tick_at_ms: &mut app.task_board_next_scheduler_tick_at_ms,
            next_auto_promote_tick_at_ms: &mut app.task_board_next_auto_promote_tick_at_ms,
        },
        message,
    ) {
        Ok(()) => {
            save_settings(app);
            return iced::Task::none();
        }
        Err(message) => message,
    };

    let message = match taskboard::update::subtasks::update_subtasks(
        &mut app.task_board_tasks,
        message,
        crate::app::time::now_ms(),
    ) {
        Ok(changed_task) => {
            if let Some(task) = changed_task {
                sync_viewing_logs(app, &task.id);
                return persist_task_metadata_patch_task(app, task, "保存子任务");
            }
            return iced::Task::none();
        }
        Err(message) => message,
    };

    let message = match taskboard::update::local::update_local(
        taskboard::update::local::LocalState {
            last_model: &mut app.task_board_last_model,
            create_modal_open: &mut app.task_board_create_modal_open,
            tasks: &mut app.task_board_tasks,
            logs_auto_scroll: &mut app.task_board_logs_auto_scroll,
            editing_task_id: &mut app.task_board_editing_task_id,
            detail_tab: &mut app.task_board_detail_tab,
            bulk_acp_agent: &mut app.task_board_bulk_acp_agent,
            bulk_agent: &mut app.task_board_bulk_agent,
            bulk_executor_popover: &mut app.task_board_bulk_executor_popover,
            bulk_priority_input: &mut app.task_board_bulk_priority_input,
            bulk_reasoning_effort: &mut app.task_board_bulk_reasoning_effort,
            bulk_task_split_mode: &mut app.task_board_bulk_task_split_mode,
            column_has_vertical_scrollbar: &mut app.task_board_column_has_vertical_scrollbar,
            context_menu: &mut app.task_board_context_menu,
            create_submit_success: &mut app.task_board_create_submit_success,
            desc_editor: &mut app.task_board_desc_editor,
            draft: &mut app.task_board_draft,
            drag_pending: &mut app.task_board_drag_pending,
            dragging: &mut app.task_board_dragging,
            edit_submit_success: &mut app.task_board_edit_submit_success,
            executor_popover: &mut app.task_board_executor_popover,
            filter_priority: &mut app.task_board_filter_priority,
            filter_status: &mut app.task_board_filter_status,
            import_editor: &mut app.task_board_import_editor,
            import_prompt_collapsed: &mut app.task_board_import_prompt_collapsed,
            import_prompt_format: &mut app.task_board_import_prompt_format,
            is_import_mode: &mut app.task_board_is_import_mode,
            last_acp_agent: &mut app.task_board_last_acp_agent,
            logs_context_menu_open: &mut app.task_board_logs_context_menu_open,
            logs_context_menu_pos: &mut app.task_board_logs_context_menu_pos,
            new_subtask_content: &mut app.task_board_new_subtask_content,
            prompt_editor: &mut app.task_board_prompt_editor,
            selected_task: &mut app.task_board_selected_task,
            selected_tasks: &mut app.task_board_selected_tasks,
            settings_modal_open: &mut app.task_board_settings_modal_open,
            settings_modal_tab: &mut app.task_board_settings_modal_tab,
            sort_ascending: &mut app.task_board_sort_ascending,
            sort_by_priority: &mut app.task_board_sort_by_priority,
            worktree_manual_confirm_kind: &mut app.task_board_worktree_manual_confirm_kind,
        },
        message,
        MAIN_AGENT_KEY,
    ) {
        Ok(viewing_task) => {
            if let taskboard::update::local::LocalEffect::ViewLogs(task) = viewing_task {
                set_viewing_logs(app, task);
            }
            return iced::Task::none();
        }
        Err(message) => message,
    };

    match &message {
        TaskBoardMessage::StartExecution
        | TaskBoardMessage::StopExecution
        | TaskBoardMessage::ExecutionTick
        | TaskBoardMessage::AgentSchedulerExecutionWakeCompleted(_)
        | TaskBoardMessage::AgentSchedulerCommandInFlightStatusTick { .. }
        | TaskBoardMessage::AgentSchedulerMergeReadyPersisted { .. }
        | TaskBoardMessage::AutoCodeReviewTick
        | TaskBoardMessage::AutoMergeTick
        | TaskBoardMessage::AgentSchedulerAutoMergeWakeCompleted(_)
        | TaskBoardMessage::PromotePoolTasksTick
        | TaskBoardMessage::AgentSchedulerPoolWakeCompleted(_)
        | TaskBoardMessage::SimulateStep { .. }
        | TaskBoardMessage::ExecuteTask { .. } => {
            return execution_update::update(app, message);
        }
        TaskBoardMessage::TaskExecutionCompleted { .. }
        | TaskBoardMessage::TaskCodeReviewCompleted { .. }
        | TaskBoardMessage::TaskMergeCompleted { .. } => {
            return execution_results_update::update(app, message);
        }
        _ => {}
    }

    match message {
        TaskBoardMessage::ToggleBoard => {
            let board_is_open = app.navigation_tabs.screen == crate::app::Screen::TaskBoard;

            if board_is_open {
                app.show_task_board = false;
                app.navigation_tabs.screen = if app.project_path.is_some() {
                    crate::app::Screen::Project
                } else {
                    crate::app::Screen::Home
                };
                app.task_board_worktree_snapshot_loading = false;
                return iced::Task::none();
            }

            if app.project_path.is_none() {
                return iced::Task::none();
            }

            app.show_task_board = true;
            app.navigation_tabs.screen = crate::app::Screen::TaskBoard;
            if let Some(project_path) = &app.project_path {
                let path: String = project_path.clone();
                return iced::Task::batch(vec![
                    maybe_schedule_worktree_snapshot_refresh(app, true),
                    sync_task_board_gateway_snapshot(path),
                ]);
            }

            iced::Task::none()
        }
        TaskBoardMessage::CloseBoard => {
            app.show_task_board = false;
            app.navigation_tabs.screen = if app.project_path.is_some() {
                crate::app::Screen::Project
            } else {
                crate::app::Screen::Home
            };
            app.task_board_worktree_snapshot_loading = false;
            app.task_board_bulk_active_status = None;
            app.task_board_selected_tasks.clear();
            iced::Task::none()
        }
        TaskBoardMessage::LoadTasks => {
            app.task_board_next_refresh_at_ms =
                next_deadline_ms(task_board_refresh_interval_secs(app));
            let refresh_snapshot = maybe_schedule_worktree_snapshot_refresh(app, false);
            if let Some(project_path) = &app.project_path {
                let path = project_path.clone();
                return iced::Task::batch(vec![
                    refresh_snapshot,
                    sync_task_board_gateway_snapshot(path),
                ]);
            }
            refresh_snapshot
        }
        TaskBoardMessage::TasksLoaded { project_path, tasks } => {
            if !task_board_project_matches(app, &project_path) {
                return iced::Task::none();
            }
            app.task_board_tasks = merge_loaded_tasks_preserving_running_state(
                tasks,
                &app.task_board_tasks,
                &app.task_board_executor.running_tasks,
                &app.task_board_tasks_pending_persistence,
            );
            sync_task_log_cache_for_loaded_tasks(app);
            app.task_board_loading = false;
            prune_bulk_selection(app);
            app.task_board_settings =
                sanitized_task_board_settings(app.task_board_settings.clone());
            iced::Task::none()
        }

        TaskBoardMessage::TaskStatusChanged { task_id, new_status } => {
            if let Some(project_path) = &app.project_path {
                let path = project_path.clone();
                let task_id_clone = task_id.clone();
                let workflow_id = app.task_board_settings.workflow_id.clone();
                let target_status = new_status;
                return iced::Task::perform(
                    async move {
                        move_task_status_via_gateway(
                            &path,
                            &task_id_clone,
                            new_status,
                            &workflow_id,
                            None,
                        )
                        .await
                    },
                    move |result| {
                        Message::TaskBoard(TaskBoardMessage::MoveTaskStatusCompleted {
                            target_status,
                            result,
                        })
                    },
                );
            }
            iced::Task::none()
        }

        TaskBoardMessage::TaskDeleted(task_id) => {
            app.task_board_selected_tasks.remove(&task_id);
            if let Some(project_path) = &app.project_path {
                let path = project_path.clone();
                let task_id_clone = task_id.clone();
                let delete_task = iced::Task::perform(
                    async move { delete_tasks_via_gateway(&path, &[task_id_clone]).await },
                    |result| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(result)),
                );
                let worktree_task = recycle_assigned_task_worktree_for_removal(app, &task_id);
                return iced::Task::batch(vec![delete_task, worktree_task]);
            }
            iced::Task::none()
        }

        TaskBoardMessage::DropOnStatus { to_status, insert_index } => {
            let Some((task_id, _from_status)) = app.task_board_dragging.take() else {
                return iced::Task::none();
            };
            app.task_board_drag_pending = None;

            let Some(insert_at) = taskboard::update::rules::drop_insert_index(
                &app.task_board_tasks,
                &task_id,
                to_status,
                insert_index,
            ) else {
                return iced::Task::none();
            };

            if let Some(project_path) = &app.project_path {
                let path = project_path.clone();
                let task_id_clone = task_id.clone();
                let workflow_id = app.task_board_settings.workflow_id.clone();
                let order_no = Some(i64::try_from(insert_at).unwrap_or(i64::MAX));
                let target_status = to_status;
                return iced::Task::perform(
                    async move {
                        move_task_status_via_gateway(
                            &path,
                            &task_id_clone,
                            to_status,
                            &workflow_id,
                            order_no,
                        )
                        .await
                    },
                    move |result| {
                        Message::TaskBoard(TaskBoardMessage::MoveTaskStatusCompleted {
                            target_status,
                            result,
                        })
                    },
                );
            }
            iced::Task::none()
        }
        TaskBoardMessage::OpenTaskInSession { task_id } => {
            if let Some(task) = app.task_board_tasks.iter().find(|t| t.id == task_id) {
                let _prompt = if !task.prompt.is_empty() {
                    task.prompt.clone()
                } else {
                    format!("执行任务: {}\n\n{}", task.id, task.description)
                };

                app.show_task_board = false;
                return iced::Task::batch(vec![iced::Task::done(Message::TaskBoard(
                    TaskBoardMessage::TaskStatusChanged {
                        task_id: task_id.clone(),
                        new_status: TaskStatus::Pending,
                    },
                ))]);
            }
            iced::Task::none()
        }
        TaskBoardMessage::TaskArchived(task_id) => {
            app.task_board_selected_tasks.remove(&task_id);
            if let Some(project_path) = &app.project_path {
                let path = project_path.clone();
                let task_id_clone = task_id.clone();
                let archive_gateway_command = iced::Task::perform(
                    async move { send_archive_commands_via_gateway(&path, &[task_id_clone]).await },
                    |result| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(result)),
                );
                let worktree_task = recycle_assigned_task_worktree_for_removal(app, &task_id);
                return iced::Task::batch(vec![archive_gateway_command, worktree_task]);
            }
            iced::Task::none()
        }
        TaskBoardMessage::ArchiveCompletedTasks => {
            app.task_board_context_menu = None;
            if let Some(project_path) = &app.project_path {
                let task_ids = taskboard::update::rules::completed_task_ids(&app.task_board_tasks);
                let path = project_path.clone();
                let archive_ids = task_ids.clone();
                let archive_gateway_command = iced::Task::perform(
                    async move { send_archive_commands_via_gateway(&path, &archive_ids).await },
                    |result| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(result)),
                );
                let worktree_task = recycle_assigned_task_worktrees_for_removal(app, &task_ids);
                return iced::Task::batch(vec![archive_gateway_command, worktree_task]);
            }
            iced::Task::none()
        }
        TaskBoardMessage::DuplicateTask(task_id) => {
            let Some(source_task) = app.task_board_tasks.iter().find(|t| t.id == task_id).cloned()
            else {
                return iced::Task::none();
            };

            let copied_task = taskboard::update::forms::duplicate_task(
                source_task,
                initial_task_status_for_app(app),
            );

            if let Some(project_path) = &app.project_path {
                let path = project_path.clone();
                let result_project_path = path.clone();
                let workflow_id = app.task_board_settings.workflow_id.clone();
                return iced::Task::perform(
                    async move { create_task_via_agent_scheduler(path, copied_task, workflow_id).await },
                    move |result| match result {
                        Ok(created_task) => Message::TaskBoard(TaskBoardMessage::TaskCreated {
                            project_path: result_project_path.clone(),
                            task: created_task,
                        }),
                        Err(e) => {
                            eprintln!("Failed to duplicate task: {}", e);
                            Message::None
                        }
                    },
                );
            }
            iced::Task::none()
        }
        TaskBoardMessage::CreateTaskSubmitted => {
            let Some(task) = taskboard::update::forms::task_from_draft(
                &app.task_board_draft,
                initial_task_status_for_app(app),
                MAIN_AGENT_KEY,
            ) else {
                return iced::Task::none();
            };

            if let Some(project_path) = &app.project_path {
                let path = project_path.clone();
                let result_project_path = path.clone();
                let task_for_result = task.clone();
                let workflow_id = app.task_board_settings.workflow_id.clone();
                return iced::Task::perform(
                    async move {
                        create_task_via_agent_scheduler(path, task_for_result, workflow_id).await
                    },
                    move |result| match result {
                        Ok(created_task) => Message::TaskBoard(TaskBoardMessage::TaskCreated {
                            project_path: result_project_path.clone(),
                            task: created_task,
                        }),
                        Err(e) => {
                            eprintln!("Failed to create task: {}", e);
                            Message::None
                        }
                    },
                );
            }
            iced::Task::none()
        }
        TaskBoardMessage::AddTaskFromInput(content) => {
            let task = {
                let runtime = app.current_session_runtime();
                taskboard::update::forms::task_from_input(
                    &content,
                    app.task_board_settings.default_priority,
                    initial_task_status_for_app(app),
                    taskboard::update::forms::InputModelSelection {
                        auto_model: runtime.auto_model,
                        model: &runtime.model,
                        task_mode_enabled: runtime.task_mode_enabled,
                        executor: runtime.task_mode_executor.clone(),
                    },
                    None,
                )
            };
            let Some(task) = task else {
                return iced::Task::none();
            };

            if let Some(project_path) = &app.project_path {
                let path = project_path.clone();
                let result_project_path = path.clone();
                let task_for_result = task.clone();
                let workflow_id = app.task_board_settings.workflow_id.clone();
                let runtime = app.current_session_runtime_mut();
                runtime.input_editor = iced::widget::text_editor::Content::new();
                if app.active_session_id.is_none() {
                    let runtime = app.current_session_runtime();
                    app.input_editor = runtime.input_editor;
                }
                return iced::Task::perform(
                    async move {
                        create_task_via_agent_scheduler(path, task_for_result, workflow_id).await
                    },
                    move |result| match result {
                        Ok(created_task) => Message::TaskBoard(TaskBoardMessage::TaskCreated {
                            project_path: result_project_path.clone(),
                            task: created_task,
                        }),
                        Err(e) => {
                            eprintln!("Failed to create task: {}", e);
                            Message::None
                        }
                    },
                );
            }
            iced::Task::none()
        }
        TaskBoardMessage::AddTaskFromInputWithOptions {
            content,
            priority,
            model,
            reasoning_effort,
            task_split_mode,
            subtasks,
        } => {
            let task = {
                let runtime = app.current_session_runtime();
                taskboard::update::forms::task_from_input(
                    &content,
                    app.task_board_settings.default_priority,
                    initial_task_status_for_app(app),
                    taskboard::update::forms::InputModelSelection {
                        auto_model: runtime.auto_model,
                        model: &runtime.model,
                        task_mode_enabled: runtime.task_mode_enabled,
                        executor: runtime.task_mode_executor.clone(),
                    },
                    Some(taskboard::update::forms::InputTaskOptions {
                        priority,
                        model,
                        reasoning_effort,
                        task_split_mode,
                        subtasks,
                    }),
                )
            };
            let Some(task) = task else {
                return iced::Task::none();
            };

            if let Some(project_path) = &app.project_path {
                let path = project_path.clone();
                let result_project_path = path.clone();
                let task_for_result = task.clone();
                let workflow_id = app.task_board_settings.workflow_id.clone();
                let runtime = app.current_session_runtime_mut();
                runtime.input_editor = iced::widget::text_editor::Content::new();
                runtime.task_mode_subtasks = vec![String::new(), String::new(), String::new()];
                runtime.task_mode_subtask_editors = runtime
                    .task_mode_subtasks
                    .iter()
                    .map(|value| iced::widget::text_editor::Content::with_text(value))
                    .collect();
                if app.active_session_id.is_none() {
                    let runtime = app.current_session_runtime();
                    app.input_editor = runtime.input_editor;
                }
                return iced::Task::perform(
                    async move {
                        create_task_via_agent_scheduler(path, task_for_result, workflow_id).await
                    },
                    move |result| match result {
                        Ok(created_task) => Message::TaskBoard(TaskBoardMessage::TaskCreated {
                            project_path: result_project_path.clone(),
                            task: created_task,
                        }),
                        Err(e) => {
                            eprintln!("Failed to create task: {}", e);
                            Message::None
                        }
                    },
                );
            }
            iced::Task::none()
        }
        TaskBoardMessage::TaskCreated { project_path, task } => {
            if !task_board_project_matches(app, &project_path) {
                return iced::Task::none();
            }
            app.task_board_tasks.push(task);
            prune_bulk_selection(app);
            app.task_board_create_submit_success = true;
            let mut post_create_tasks = Vec::new();
            if app.task_board_clear_prompt_after_create {
                app.task_board_draft.prompt.clear();
                app.task_board_prompt_editor = iced::widget::text_editor::Content::new();
            }
            if app.task_board_close_after_create {
                app.task_board_create_modal_open = false;
                app.task_board_create_submit_success = false;
                reset_create_draft(app);
                return iced::Task::batch(post_create_tasks);
            }
            post_create_tasks.push(crate::app::message::after(
                std::time::Duration::from_secs(2),
                Message::TaskBoard(TaskBoardMessage::ClearCreateSubmitSuccess),
            ));
            iced::Task::batch(post_create_tasks)
        }
        TaskBoardMessage::ViewTaskLogs(task_id) => {
            if let Some(task) = app.task_board_tasks.iter().find(|t| t.id == task_id).cloned() {
                app.task_board_logs_auto_scroll = true;
                app.task_board_detail_tab = TaskBoardDetailTab::Logs;
                set_viewing_logs(app, Some(task));
                flush_running_task_logs_throttled(app, true, web_time::Instant::now());
                if let Some(project_path) = app.project_path.clone() {
                    return sync_agent_scheduler_logs_for_project_path(project_path, vec![task_id]);
                }
            }
            iced::Task::none()
        }

        TaskBoardMessage::SelectDetailTab(tab) => {
            app.task_board_detail_tab = tab;
            if tab == TaskBoardDetailTab::Logs {
                app.task_board_logs_auto_scroll = true;
                let viewing_task = app.task_board_viewing_logs.clone();
                sync_task_logs_editor(app, viewing_task.as_ref());
                if let (Some(project_path), Some(viewing_task)) =
                    (app.project_path.clone(), viewing_task)
                {
                    return sync_agent_scheduler_logs_for_project_path(
                        project_path,
                        vec![viewing_task.id],
                    );
                }
            }
            iced::Task::none()
        }

        TaskBoardMessage::SaveEditingTask => {
            if let Some(task_id) = &app.task_board_editing_task_id.clone()
                && let Some(task) = app.task_board_tasks.iter_mut().find(|t| &t.id == task_id)
            {
                taskboard::update::forms::apply_task_draft(
                    task,
                    &app.task_board_draft,
                    MAIN_AGENT_KEY,
                    crate::app::time::now_ms(),
                );
                if let Some(project_path) = &app.project_path {
                    let path = project_path.clone();
                    let task_clone = task.clone();
                    let workflow_id = app.task_board_settings.workflow_id.clone();
                    return iced::Task::perform(
                        async move {
                            persist_updated_tasks_via_gateway(
                                &path,
                                &[task_clone],
                                "保存任务编辑",
                                &workflow_id,
                            )
                            .await
                        },
                        move |result| {
                            Message::TaskBoard(TaskBoardMessage::EditingTaskSaved(result))
                        },
                    );
                }
            }
            iced::Task::none()
        }
        TaskBoardMessage::EditingTaskSaved(result) => {
            if result.is_ok() {
                if app.task_board_close_after_edit {
                    set_viewing_logs(app, None);
                    app.task_board_logs_auto_scroll = true;
                    app.task_board_editing_task_id = None;
                    app.task_board_edit_submit_success = false;
                    return iced::Task::none();
                }
                app.task_board_edit_submit_success = true;
                return crate::app::message::after(
                    std::time::Duration::from_secs(2),
                    Message::TaskBoard(TaskBoardMessage::ClearEditSubmitSuccess),
                );
            }
            iced::Task::none()
        }

        TaskBoardMessage::LogsViewerCloseContextMenu => {
            close_logs_context_menu(app);
            focus_editor_task(&app.task_board_logs_editor_id)
        }
        TaskBoardMessage::LogsViewerContextMenuCopy => {
            close_logs_context_menu(app);
            let (_outcome, task) =
                selection_copy_task(&app.task_board_logs_editor, &app.task_board_logs_editor_id);
            task
        }
        TaskBoardMessage::LogsViewerContextMenuCut => {
            close_logs_context_menu(app);
            let (_outcome, task) =
                selection_cut_task(&mut app.task_board_logs_editor, &app.task_board_logs_editor_id);
            task
        }
        TaskBoardMessage::LogsViewerContextMenuPaste => {
            close_logs_context_menu(app);
            paste_task(&app.task_board_logs_editor_id, |content| {
                Message::TaskBoard(TaskBoardMessage::LogsViewerEditorAction(paste_action(content)))
            })
        }
        TaskBoardMessage::LogsViewerContextMenuDelete => {
            close_logs_context_menu(app);
            let (_outcome, task) = selection_delete_task(
                &mut app.task_board_logs_editor,
                &app.task_board_logs_editor_id,
            );
            task
        }
        TaskBoardMessage::ToggleClearPromptAfterCreate(enabled) => {
            app.task_board_clear_prompt_after_create = enabled;
            app.save_desktop_preference(
                "task_board_clear_prompt_after_create",
                serde_json::Value::Bool(enabled),
            )
        }
        TaskBoardMessage::ToggleCloseAfterCreate(enabled) => {
            app.task_board_close_after_create = enabled;
            app.save_desktop_preference(
                "task_board_close_after_create",
                serde_json::Value::Bool(enabled),
            )
        }
        TaskBoardMessage::ToggleCloseAfterEdit(enabled) => {
            app.task_board_close_after_edit = enabled;
            app.save_desktop_preference(
                "task_board_close_after_edit",
                serde_json::Value::Bool(enabled),
            )
        }
        TaskBoardMessage::ToggleBulkSelectionMode(status) => {
            app.task_board_context_menu = None;
            if app.task_board_bulk_active_status == Some(status) {
                deactivate_bulk_selection_mode(app);
            } else {
                deactivate_bulk_selection_mode(app);
                app.task_board_bulk_active_status = Some(status);
                reset_bulk_operation_inputs(app);
            }
            iced::Task::none()
        }
        TaskBoardMessage::BulkSetPriorityInStatus(status) => {
            app.task_board_context_menu = None;
            let mut tasks = selected_tasks_for_status(app, status);
            if tasks.is_empty() {
                return iced::Task::none();
            }

            let priority = parse_priority_or_default(&app.task_board_bulk_priority_input, 999);
            app.task_board_bulk_priority_input = priority.to_string();

            taskboard::update::rules::set_bulk_priority(&mut tasks, priority);

            let task_ids = tasks.iter().map(|task| task.id.clone()).collect::<Vec<_>>();
            clear_selected_task_ids(app, &task_ids);

            if let Some(project_path) = &app.project_path {
                let path = project_path.clone();
                let workflow_id = app.task_board_settings.workflow_id.clone();
                return iced::Task::perform(
                    async move {
                        persist_updated_tasks_via_gateway(
                            &path,
                            &tasks,
                            "批量设置优先级",
                            &workflow_id,
                        )
                        .await
                    },
                    |result| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(result)),
                );
            }
            iced::Task::none()
        }
        TaskBoardMessage::BulkSetModelInStatus(status) => {
            app.task_board_context_menu = None;
            let mut tasks = selected_tasks_for_status(app, status);
            if tasks.is_empty() {
                return iced::Task::none();
            }

            let model = normalized_bulk_model_input(&app.task_board_bulk_model_input);
            app.task_board_bulk_model_input = model.clone();
            app.task_board_bulk_reasoning_effort = taskboard::update::model::validated_reasoning_effort(
                app.task_board_bulk_acp_agent.as_deref(), app.task_board_bulk_reasoning_effort,
                &crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(app, &app.task_board_bulk_model_input),
            );
            app.task_board_last_model = model.clone();

            taskboard::update::rules::set_bulk_model(&mut tasks, &model);

            let task_ids = tasks.iter().map(|task| task.id.clone()).collect::<Vec<_>>();
            clear_selected_task_ids(app, &task_ids);

            if let Some(project_path) = &app.project_path {
                let path = project_path.to_string();
                let workflow_id = app.task_board_settings.workflow_id.clone();
                return iced::Task::perform(
                    async move {
                        persist_updated_tasks_via_gateway(
                            &path,
                            &tasks,
                            "批量设置模型",
                            &workflow_id,
                        )
                        .await
                    },
                    |result| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(result)),
                );
            }
            iced::Task::none()
        }
        TaskBoardMessage::BulkSetAgentInStatus(status) => {
            app.task_board_context_menu = None;
            let mut tasks = selected_tasks_for_status(app, status);
            if tasks.is_empty() {
                return iced::Task::none();
            }

            let agent = taskboard::update::rules::normalized_bulk_agent(
                &app.task_board_bulk_agent,
                MAIN_AGENT_KEY,
            );
            app.task_board_bulk_agent = agent.clone();

            taskboard::update::rules::set_bulk_agent(&mut tasks, &agent, MAIN_AGENT_KEY);

            let task_ids = tasks.iter().map(|task| task.id.clone()).collect::<Vec<_>>();
            clear_selected_task_ids(app, &task_ids);

            if let Some(project_path) = &app.project_path {
                let path = project_path.to_string();
                let workflow_id = app.task_board_settings.workflow_id.clone();
                return iced::Task::perform(
                    async move {
                        persist_updated_tasks_via_gateway(
                            &path,
                            &tasks,
                            "批量设置代理",
                            &workflow_id,
                        )
                        .await
                    },
                    |result| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(result)),
                );
            }
            iced::Task::none()
        }
        TaskBoardMessage::BulkSetExecutorInStatus { status, executor } => {
            app.task_board_context_menu = None;
            let mut tasks = selected_tasks_for_status(app, status);
            if tasks.is_empty() {
                return iced::Task::none();
            }

            app.task_board_bulk_acp_agent = executor.clone();
            app.task_board_last_acp_agent = executor.clone();

            taskboard::update::rules::set_bulk_executor(&mut tasks, executor.clone());

            let task_ids = tasks.iter().map(|task| task.id.clone()).collect::<Vec<_>>();
            clear_selected_task_ids(app, &task_ids);

            if let Some(project_path) = &app.project_path {
                let path = project_path.to_string();
                let workflow_id = app.task_board_settings.workflow_id.clone();
                return iced::Task::perform(
                    async move {
                        persist_updated_tasks_via_gateway(
                            &path,
                            &tasks,
                            "批量设置 ACP 智能体",
                            &workflow_id,
                        )
                        .await
                    },
                    |result| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(result)),
                );
            }
            iced::Task::none()
        }
        TaskBoardMessage::BulkSetReasoningEffortInStatus { status, reasoning_effort } => {
            app.task_board_context_menu = None;
            let task_ids = selected_task_ids_for_status(app, status);
            if task_ids.is_empty() {
                return iced::Task::none();
            }

            app.task_board_bulk_reasoning_effort = reasoning_effort;
            let tasks = taskboard::update::rules::set_bulk_reasoning_effort(
                &mut app.task_board_tasks,
                &task_ids,
                reasoning_effort,
            );

            for task_id in &task_ids {
                sync_viewing_logs(app, task_id);
            }
            if app
                .task_board_editing_task_id
                .as_ref()
                .is_some_and(|editing_task_id| task_ids.contains(editing_task_id))
            {
                app.task_board_draft.reasoning_effort = reasoning_effort;
            }
            clear_selected_task_ids(app, &task_ids);

            if let Some(project_path) = &app.project_path {
                let path = project_path.to_string();
                let workflow_id = app.task_board_settings.workflow_id.clone();
                return iced::Task::perform(
                    async move {
                        persist_updated_tasks_via_gateway(
                            &path,
                            &tasks,
                            "批量设置推理强度",
                            &workflow_id,
                        )
                        .await
                    },
                    |result| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(result)),
                );
            }
            iced::Task::none()
        }
        TaskBoardMessage::BulkSetTaskSplitModeInStatus { status, task_split_mode } => {
            app.task_board_context_menu = None;
            let task_ids = selected_task_ids_for_status(app, status);
            if task_ids.is_empty() {
                return iced::Task::none();
            }

            app.task_board_bulk_task_split_mode = task_split_mode;

            let tasks = taskboard::update::rules::set_bulk_split_mode(
                &mut app.task_board_tasks,
                &task_ids,
                task_split_mode,
            );

            for task_id in &task_ids {
                sync_viewing_logs(app, task_id);
            }
            if app
                .task_board_editing_task_id
                .as_ref()
                .is_some_and(|editing_task_id| task_ids.contains(editing_task_id))
            {
                app.task_board_draft.task_split_mode = task_split_mode;
            }
            clear_selected_task_ids(app, &task_ids);

            if let Some(project_path) = &app.project_path {
                let path = project_path.to_string();
                let workflow_id = app.task_board_settings.workflow_id.clone();
                return iced::Task::perform(
                    async move {
                        persist_updated_tasks_via_gateway(
                            &path,
                            &tasks,
                            "批量设置任务拆分",
                            &workflow_id,
                        )
                        .await
                    },
                    |result| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(result)),
                );
            }
            iced::Task::none()
        }
        TaskBoardMessage::BulkArchiveTasksInStatus(status) => {
            app.task_board_context_menu = None;
            let task_ids = selected_task_ids_for_status(app, status);
            if task_ids.is_empty() {
                return iced::Task::none();
            }
            clear_selected_task_ids(app, &task_ids);
            if let Some(project_path) = &app.project_path {
                let path = project_path.to_string();
                let archive_ids = task_ids.clone();
                let archive_gateway_command = iced::Task::perform(
                    async move { send_archive_commands_via_gateway(&path, &archive_ids).await },
                    |result| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(result)),
                );
                let worktree_task = recycle_assigned_task_worktrees_for_removal(app, &task_ids);
                return iced::Task::batch(vec![archive_gateway_command, worktree_task]);
            }
            iced::Task::none()
        }
        TaskBoardMessage::BulkDeleteTasksInStatus(status) => {
            app.task_board_context_menu = None;
            let task_ids = selected_task_ids_for_status(app, status);
            if task_ids.is_empty() {
                return iced::Task::none();
            }
            clear_selected_task_ids(app, &task_ids);
            if let Some(project_path) = &app.project_path {
                let path = project_path.to_string();
                let delete_task_ids = task_ids.clone();
                let delete_task = iced::Task::perform(
                    async move { delete_tasks_via_gateway(&path, &delete_task_ids).await },
                    |result| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(result)),
                );
                let worktree_task = recycle_assigned_task_worktrees_for_removal(app, &task_ids);
                return iced::Task::batch(vec![delete_task, worktree_task]);
            }
            iced::Task::none()
        }
        TaskBoardMessage::BulkMoveTasksInStatus { from_status, to_status } => {
            app.task_board_context_menu = None;
            if from_status == to_status {
                return iced::Task::none();
            }
            let task_ids = selected_task_ids_for_status(app, from_status);
            if task_ids.is_empty() {
                return iced::Task::none();
            }
            clear_selected_task_ids(app, &task_ids);
            if let Some(project_path) = &app.project_path {
                let path = project_path.to_string();
                let workflow_id = app.task_board_settings.workflow_id.clone();
                return iced::Task::perform(
                    async move {
                        move_tasks_to_status_via_gateway(&path, &task_ids, to_status, &workflow_id)
                            .await
                    },
                    |result| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(result)),
                );
            }
            iced::Task::none()
        }
        TaskBoardMessage::BulkMergeTasksInStatus(status) => {
            app.task_board_context_menu = None;
            if status != TaskStatus::PrSubmitted {
                return iced::Task::none();
            }
            let mut tasks = selected_tasks_for_status(app, status);
            if tasks.is_empty() {
                return iced::Task::none();
            }
            let task_ids = taskboard::update::rules::prepare_manual_merge_queue(&mut tasks);
            app.task_board_manual_merge_queue = task_ids.clone();
            clear_selected_task_ids(app, &task_ids);

            let mut actions = Vec::new();
            if let Some(project_path) = &app.project_path {
                let path = project_path.to_string();
                let workflow_id = app.task_board_settings.workflow_id.clone();
                actions.push(iced::Task::perform(
                    async move {
                        persist_updated_tasks_via_gateway(
                            &path,
                            &tasks,
                            "手动批量合并入队",
                            &workflow_id,
                        )
                        .await
                    },
                    |result| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(result)),
                ));
            }
            actions
                .push(iced::Task::done(Message::TaskBoard(TaskBoardMessage::ManualMergeQueueTick)));
            iced::Task::batch(actions)
        }
        TaskBoardMessage::ManualMergeQueueTick => start_manual_merge_queue_task(app),
        TaskBoardMessage::BulkActionCompleted(result) => {
            if let Err(error) = result {
                app.push_notification(error);
            }
            if app.task_board_manual_merge_queue.is_empty() {
                iced::Task::done(Message::TaskBoard(TaskBoardMessage::LoadTasks))
            } else {
                iced::Task::none()
            }
        }
        TaskBoardMessage::MoveTaskStatusCompleted { target_status, result } => {
            if let Err(error) = result {
                app.push_notification(error);
                return iced::Task::done(Message::TaskBoard(TaskBoardMessage::LoadTasks));
            }
            let mut tasks = vec![iced::Task::done(Message::TaskBoard(TaskBoardMessage::LoadTasks))];
            if target_status == TaskStatus::Pending && app.task_board_executor_running {
                tasks.push(iced::Task::done(Message::TaskBoard(TaskBoardMessage::ExecutionTick)));
            }
            iced::Task::batch(tasks)
        }
        TaskBoardMessage::ToggleWorktreePixelOffice(enabled) => {
            app.task_board_worktree_pixel_office = enabled;
            app.save_desktop_preference(
                "task_board_worktree_pixel_office",
                serde_json::Value::Bool(enabled),
            )
        }
        TaskBoardMessage::CleanAllWorktreesPressed => {
            let Some(project_path) = app.project_path.clone() else {
                return iced::Task::none();
            };
            if app.task_board_worktree_manual_action_kind.is_some() {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Warning,
                    "已有 worktree 操作进行中",
                    vec![],
                );
            }
            if app.task_board_worktree_manual_confirm_kind != Some("cleanup") {
                app.task_board_worktree_manual_confirm_kind = Some("cleanup");
                app.push_localized_notification(
                    "一键清理风险较高，请再次点击“确认一键清理”继续，或点击“取消”返回",
                    vec![],
                );
                app.notifications_expanded = true;
                return iced::Task::none();
            }
            let (log_tx, log_rx) = std::sync::mpsc::channel();
            app.task_board_worktree_manual_confirm_kind = None;
            app.task_board_worktree_manual_action_kind = Some("cleanup");
            app.task_board_worktree_action_logs.clear();
            app.task_board_worktree_action_logs_visible_until_ms = None;
            app.task_board_worktree_action_log_rx = Some(log_rx);
            push_worktree_action_log(app, "准备开始一键清理...".to_string());
            iced::Task::batch(vec![
                iced::Task::perform(
                    async move {
                        drop(log_tx);
                        cleanup_office_contexts(project_path).await
                    },
                    |result| {
                        Message::TaskBoard(TaskBoardMessage::CleanAllWorktreesCompleted(result))
                    },
                ),
                schedule_worktree_action_log_tick(),
            ])
        }
        TaskBoardMessage::CleanAllWorktreesCompleted(result) => {
            poll_worktree_action_logs(app);
            match result {
                Ok(count) => app.push_localized_notification(
                    "已完成 worktree 一键清理，共处理 %s 个槽位",
                    vec![count.to_string().into()],
                ),
                Err(error) => {
                    push_worktree_action_log(app, error.clone());
                    app.push_localized_notification(
                        "worktree 一键清理有失败，请查看右下角通知：%s",
                        vec![truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS).into()],
                    );
                    app.notifications_expanded = true;
                }
            }
            finish_worktree_action_logs(app);
            maybe_schedule_worktree_snapshot_refresh(app, true)
        }
        TaskBoardMessage::DeleteAllWorktreesPressed => {
            let Some(_) = app.project_path.as_ref() else {
                return iced::Task::none();
            };
            if app.task_board_worktree_manual_action_kind.is_some() {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Warning,
                    "已有 worktree 操作进行中",
                    vec![],
                );
            }
            if app.task_board_worktree_manual_confirm_kind != Some("delete") {
                app.task_board_worktree_manual_confirm_kind = Some("delete");
                app.push_localized_notification("删除所有 worktree 风险极高，请再次点击“确认删除所有 worktree”继续，或点击“取消”返回", vec![]);
                app.notifications_expanded = true;
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Warning,
                    "再次点击确认按钮才会删除 worktree",
                    vec![],
                );
            }
            let (_log_tx, log_rx) = std::sync::mpsc::channel();
            app.task_board_worktree_manual_confirm_kind = None;
            app.task_board_worktree_manual_action_kind = Some("delete");
            app.task_board_worktree_action_logs.clear();
            app.task_board_worktree_action_logs_visible_until_ms = None;
            app.task_board_worktree_action_log_rx = Some(log_rx);
            push_worktree_action_log(app, "准备开始删除所有 worktree...".to_string());
            iced::Task::batch(vec![
                iced::Task::perform(
                    async {
                        Err("办公室不直接删除目录；请配置显式删除任务或工作流".to_string())
                    },
                    |result| {
                        Message::TaskBoard(TaskBoardMessage::DeleteAllWorktreesCompleted(result))
                    },
                ),
                schedule_worktree_action_log_tick(),
            ])
        }
        TaskBoardMessage::DeleteAllWorktreesCompleted(result) => {
            poll_worktree_action_logs(app);
            let toast_task = match result {
                Ok(0) => {
                    app.push_localized_notification("没有找到可删除的受管 worktree", vec![]);
                    app.show_localized_toast(
                        crate::app::state::ToastKind::Warning,
                        "没有找到可删除的受管 worktree",
                        vec![],
                    )
                }
                Ok(count) => {
                    app.push_localized_notification(
                        "已删除所有 worktree，共处理 %s 个槽位",
                        vec![(count).to_string().into()],
                    );
                    app.show_localized_toast(
                        crate::app::state::ToastKind::Success,
                        "已删除 %u 个 worktree",
                        vec![count.into()],
                    )
                }
                Err(error) => {
                    push_worktree_action_log(app, error.clone());
                    app.push_localized_notification(
                        "删除所有 worktree 有失败，请查看右下角通知：%s",
                        vec![
                            (truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS))
                                .to_string()
                                .into(),
                        ],
                    );
                    app.notifications_expanded = true;
                    app.show_localized_toast(
                        crate::app::state::ToastKind::Error,
                        "删除所有 worktree 失败",
                        vec![],
                    )
                }
            };
            finish_worktree_action_logs(app);
            iced::Task::batch(vec![toast_task, maybe_schedule_worktree_snapshot_refresh(app, true)])
        }
        TaskBoardMessage::RecycleWorktreeSlotPressed { slot_id } => {
            let Some(project_path) = app.project_path.clone() else {
                return iced::Task::none();
            };
            if app.task_board_worktree_manual_action_kind.is_some() {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Warning,
                    "已有 worktree 操作进行中",
                    vec![],
                );
            }
            if let Some(snapshot) = &app.task_board_worktree_snapshot
                && let Some(slot) = snapshot.slots.iter().find(|slot| slot.id == slot_id)
                && matches!(
                    slot.state,
                    crate::app::task::WorktreeState::Busy
                        | crate::app::task::WorktreeState::Recycling
                )
            {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Warning,
                    "该 worktree 正在使用中，暂不能回收",
                    vec![],
                );
            }

            let (_log_tx, log_rx) = std::sync::mpsc::channel();
            app.task_board_worktree_manual_confirm_kind = None;
            app.task_board_worktree_manual_action_kind = Some("slot_recycle");
            app.task_board_worktree_action_logs.clear();
            app.task_board_worktree_action_logs_visible_until_ms = None;
            app.task_board_worktree_action_log_rx = Some(log_rx);
            push_worktree_action_log(app, format!("准备回收 worktree 槽位 {}...", slot_id));
            iced::Task::batch(vec![
                iced::Task::perform(
                    recycle_worktree_slot_via_gateway(project_path, slot_id.clone()),
                    |(slot_id, result)| {
                        Message::TaskBoard(TaskBoardMessage::WorktreeSlotRecycled {
                            slot_id,
                            result,
                        })
                    },
                ),
                schedule_worktree_action_log_tick(),
            ])
        }
        TaskBoardMessage::WorktreeSlotRecycled { slot_id, result } => {
            poll_worktree_action_logs(app);
            let toast_task = match result {
                Ok(()) => {
                    app.push_localized_notification(
                        "worktree 槽位已回收: %s",
                        vec![(slot_id).to_string().into()],
                    );
                    app.show_localized_toast(
                        crate::app::state::ToastKind::Success,
                        "已回收 worktree %s",
                        vec![slot_id.into()],
                    )
                }
                Err(error) => {
                    app.push_localized_notification(
                        "worktree 槽位回收失败 slot=%s：%s",
                        vec![
                            (slot_id).to_string().into(),
                            (truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS))
                                .to_string()
                                .into(),
                        ],
                    );
                    app.notifications_expanded = true;
                    app.show_localized_toast(
                        crate::app::state::ToastKind::Error,
                        "worktree 槽位回收失败",
                        vec![],
                    )
                }
            };
            finish_worktree_action_logs(app);
            iced::Task::batch(vec![toast_task, maybe_schedule_worktree_snapshot_refresh(app, true)])
        }
        TaskBoardMessage::CommitMergeAllWorktreesPressed => {
            let Some(_) = app.project_path.as_ref() else {
                return iced::Task::none();
            };
            if app.task_board_worktree_manual_action_kind.is_some() {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Warning,
                    "已有 worktree 操作进行中",
                    vec![],
                );
            }
            if app.task_board_worktree_manual_confirm_kind != Some("merge") {
                app.task_board_worktree_manual_confirm_kind = Some("merge");
                app.push_localized_notification(
                    "一键合并风险较高，请再次点击“确认一键合并”继续，或点击“取消”返回",
                    vec![],
                );
                app.notifications_expanded = true;
                return iced::Task::none();
            }
            let (_log_tx, log_rx) = std::sync::mpsc::channel();
            app.task_board_worktree_manual_confirm_kind = None;
            app.task_board_worktree_manual_action_kind = Some("merge");
            app.task_board_worktree_action_logs.clear();
            app.task_board_worktree_action_logs_visible_until_ms = None;
            app.task_board_worktree_action_log_rx = Some(log_rx);
            push_worktree_action_log(app, "准备开始一键合并...".to_string());
            iced::Task::batch(vec![
                iced::Task::perform(
                    async {
                        Err("办公室不直接合并分支；请配置显式 Git 任务或工作流".to_string())
                    },
                    |result| {
                        Message::TaskBoard(TaskBoardMessage::CommitMergeAllWorktreesCompleted(
                            result,
                        ))
                    },
                ),
                schedule_worktree_action_log_tick(),
            ])
        }
        TaskBoardMessage::CommitMergeAllWorktreesCompleted(result) => {
            poll_worktree_action_logs(app);
            match result {
                Ok(count) => app.push_localized_notification(
                    "已完成 worktree 一键合并，共合并 %s 个分支",
                    vec![count.to_string().into()],
                ),
                Err(error) => {
                    push_worktree_action_log(app, error.clone());
                    app.push_localized_notification(
                        "worktree 一键合并有失败，请查看右下角通知：%s",
                        vec![truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS).into()],
                    );
                    app.notifications_expanded = true;
                }
            }
            finish_worktree_action_logs(app);
            maybe_schedule_worktree_snapshot_refresh(app, true)
        }
        TaskBoardMessage::CancelWorktreeManualConfirm => {
            let Some(kind) = app.task_board_worktree_manual_confirm_kind.take() else {
                return iced::Task::none();
            };
            let action_label = match kind {
                "cleanup" => "一键清理",
                "delete" => "一键删除",
                "merge" => "一键合并",
                _ => "worktree 操作",
            };
            app.push_localized_notification(
                "已取消 %s 确认，可重新发起操作",
                vec![(action_label).to_string().into()],
            );
            iced::Task::none()
        }
        TaskBoardMessage::WorktreeActionLogTick => {
            poll_worktree_action_logs(app);
            if app.task_board_worktree_action_log_rx.is_some() {
                return schedule_worktree_action_log_tick();
            }
            if let Some(deadline_ms) = app.task_board_worktree_action_logs_visible_until_ms
                && now_ms() >= deadline_ms
            {
                clear_worktree_action_logs(app);
            }
            iced::Task::none()
        }
        TaskBoardMessage::UiTick => {
            poll_worktree_action_logs(app);
            if app.task_board_worktree_action_log_rx.is_none()
                && let Some(deadline_ms) = app.task_board_worktree_action_logs_visible_until_ms
                && now_ms() >= deadline_ms
            {
                clear_worktree_action_logs(app);
            }
            iced::Task::batch(vec![maybe_schedule_worktree_snapshot_refresh(app, false)])
        }
        TaskBoardMessage::WorktreePoolMaintained { project_path, result } => {
            if !task_board_project_matches(app, &project_path) {
                return iced::Task::none();
            }
            if !app.project_is_git_repo {
                app.task_board_worktree_maintenance_in_flight = false;
                app.task_board_worktree_snapshot = None;
                app.task_board_worktree_snapshot_loading = false;
                app.task_board_last_worktree_snapshot_at_ms = 0;
                return iced::Task::none();
            }
            app.task_board_worktree_maintenance_in_flight = false;
            match result {
                Ok(outcome) => {
                    tracing::debug!(
                        idle_count = outcome.idle_count,
                        target_idle_count = outcome.target_idle_count,
                        "gateway worktree pool maintenance completed"
                    );
                    if let Some(snapshot) = outcome.snapshot {
                        app.task_board_worktree_snapshot_loading = false;
                        app.task_board_last_worktree_snapshot_at_ms = now_ms();
                        app.task_board_worktree_snapshot = Some(snapshot);
                        iced::Task::none()
                    } else {
                        maybe_schedule_worktree_snapshot_refresh(app, true)
                    }
                }
                Err(error) => {
                    tracing::warn!(error = %error, "gateway worktree pool maintenance failed");
                    if let Some(project_path) = &app.project_path {
                        let changed = taskboard::update::rules::log_worktree_maintenance_failure(
                            &mut app.task_board_tasks,
                            &error,
                        );
                        if changed {
                            let persist_tasks =
                                build_persist_tasks(project_path, &app.task_board_tasks);
                            if !persist_tasks.is_empty() {
                                return iced::Task::batch(persist_tasks);
                            }
                        }
                    }
                    iced::Task::none()
                }
            }
        }
        TaskBoardMessage::WorktreeSnapshotLoaded { project_path, result } => {
            if !task_board_project_matches(app, &project_path) {
                return iced::Task::none();
            }
            if !app.project_is_git_repo {
                app.task_board_worktree_snapshot_loading = false;
                app.task_board_worktree_snapshot = None;
                app.task_board_last_worktree_snapshot_at_ms = 0;
                return iced::Task::none();
            }
            app.task_board_worktree_snapshot_loading = false;
            match result {
                Ok(snapshot) => {
                    app.task_board_last_worktree_snapshot_at_ms = now_ms();
                    app.task_board_worktree_snapshot = snapshot;
                }
                Err(error) => {
                    app.task_board_last_worktree_snapshot_at_ms = now_ms();
                    if gateway_http::is_gateway_unavailable_error(&error) {
                        tracing::debug!(
                            error = %error,
                            "worktree pool snapshot refresh skipped because gateway is unavailable"
                        );
                    } else {
                        tracing::warn!(
                            error = %error,
                            "gateway worktree pool snapshot refresh failed"
                        );
                    }
                }
            }
            iced::Task::none()
        }
        TaskBoardMessage::TaskRemovalWorktreeRecycled { project_path, task_id, result } => {
            if !task_board_project_matches(app, &project_path) {
                return iced::Task::none();
            }
            if let Err(error) = result {
                app.push_localized_notification(
                    "任务 %s worktree 回收失败: %s",
                    vec![
                        (task_id).to_string().into(),
                        (truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS)).to_string().into(),
                    ],
                );
            }
            maybe_schedule_worktree_snapshot_refresh(app, true)
        }
        TaskBoardMessage::ToggleWorktreePanelExpanded => {
            app.task_board_worktree_panel_expanded = !app.task_board_worktree_panel_expanded;
            maybe_schedule_worktree_snapshot_refresh(app, true)
        }
        TaskBoardMessage::AgentSchedulerStatusLoaded { project_path, result } => {
            if !task_board_project_matches(app, &project_path) {
                return iced::Task::none();
            }
            let response = match result {
                Ok(response) => response,
                Err(error) => {
                    if agent_scheduler_gateway_unavailable(&error) {
                        tracing::debug!(
                            error = %error,
                            "agent scheduler status sync skipped because gateway is unavailable"
                        );
                        return iced::Task::none();
                    }
                    tracing::warn!(error = %error, "agent scheduler status sync failed");
                    return app.show_localized_toast(
                        crate::app::state::ToastKind::Warning,
                        "同步 Agent Scheduler 状态失败: %s",
                        vec![truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS).into()],
                    );
                }
            };
            let response_project_id = response.project_id.clone();
            let updated_tasks = apply_agent_scheduler_status(app, response);
            let task_ids = updated_tasks.iter().map(|task| task.id.clone()).collect::<Vec<_>>();
            let followup_ticks =
                schedule_agent_scheduler_status_followup_ticks(app, &updated_tasks);
            iced::Task::batch(vec![
                sync_agent_scheduler_logs(project_path, response_project_id, task_ids),
                followup_ticks,
            ])
        }
        TaskBoardMessage::TaskResultPersistenceCompleted { project_path, task_id, result } => {
            if !task_board_project_matches(app, &project_path) {
                return iced::Task::none();
            }
            app.task_board_tasks_pending_persistence.remove(&task_id);
            let sync = sync_agent_scheduler_status_for_current_project(app);
            if let Err(error) = result {
                return iced::Task::batch(vec![
                    app.show_localized_toast(
                        crate::app::state::ToastKind::Error,
                        "保存任务结果失败: %s",
                        vec![truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS).into()],
                    ),
                    sync,
                ]);
            }
            sync
        }
        TaskBoardMessage::AgentSchedulerLogsLoaded { project_path, result } => {
            if !task_board_project_matches(app, &project_path) {
                return iced::Task::none();
            }
            let responses = match result {
                Ok(response) => response,
                Err(error) => {
                    tracing::warn!(error = %error, "agent scheduler logs sync failed");
                    return iced::Task::none();
                }
            };
            let _updated_tasks = apply_agent_scheduler_logs(app, responses);
            iced::Task::none()
        }
        TaskBoardMessage::TaskPlanningCompleted { project_path, task_id, result } => {
            if !task_board_project_matches(app, &project_path) {
                return iced::Task::none();
            }
            let final_logs = app.task_board_executor.poll_task_logs_all(&task_id);
            app.task_board_executor.finish_task(&task_id);
            let Some(project_path) = app.project_path.clone() else {
                return iced::Task::none();
            };
            let mut task_to_persist: Option<Task> = None;
            let mut execute_task_to_start: Option<Task> = None;
            let mut plan_failed = false;

            if let Some(task) = app.task_board_tasks.iter_mut().find(|t| t.id == task_id) {
                match taskboard::update::rules::apply_planning_result(task, final_logs, result) {
                    taskboard::update::rules::PlanningEffect::AllocateWorktree => {
                        let _ = crate::app::task::write_task_plan_files(&project_path, task);

                        match crate::app::task::assign_task_execution_worktree(
                            &project_path,
                            task,
                            None,
                        ) {
                            Ok(assigned_worktree_path) => {
                                taskboard::update::rules::start_planned_task(
                                    task,
                                    assigned_worktree_path,
                                );
                                let _ =
                                    crate::app::task::write_task_plan_files(&project_path, task);
                                execute_task_to_start = Some(task.clone());
                                task_to_persist = Some(task.clone());
                            }
                            Err(error) => {
                                mark_task_paused_after_worktree_preallocation_failure(task, error);
                                plan_failed = true;
                                task_to_persist = Some(task.clone());
                            }
                        }
                    }
                    taskboard::update::rules::PlanningEffect::PersistFailure => {
                        plan_failed = true;
                        task_to_persist = Some(task.clone());
                    }
                    taskboard::update::rules::PlanningEffect::Ignored => {}
                }
            }

            sync_viewing_logs(app, &task_id);
            let persist_task = task_to_persist
                .clone()
                .map(|task| {
                    let path = project_path.clone();
                    let workflow_id = app.task_board_settings.workflow_id.clone();
                    iced::Task::perform(
                        async move {
                            persist_updated_tasks_via_gateway(
                                &path,
                                &[task],
                                "任务拆分结果同步",
                                &workflow_id,
                            )
                            .await
                        },
                        |_| Message::None,
                    )
                })
                .unwrap_or_else(iced::Task::none);
            let publish_task = iced::Task::none();
            if plan_failed {
                return iced::Task::batch(vec![
                    persist_task,
                    publish_task,
                    schedule_scheduler_tick_with_deadline(app),
                ]);
            }
            if let Some(execute_task_model) = execute_task_to_start {
                app.task_board_executor.start_task(&task_id);
                app.task_board_executor.register_log_channel(task_id.clone());
                let log_sender = app.task_board_executor.get_log_sender(&task_id);
                let path_clone = project_path.clone();
                let result_project_path = project_path.clone();
                let execute_task = iced::Task::perform(
                    crate::app::task::execute_task_async(
                        execute_task_model,
                        path_clone,
                        log_sender,
                    ),
                    move |(tid, result)| {
                        Message::TaskBoard(TaskBoardMessage::TaskExecutionCompleted {
                            project_path: result_project_path.clone(),
                            task_id: tid,
                            result,
                        })
                    },
                );
                return iced::Task::batch(vec![
                    persist_task,
                    publish_task,
                    execute_task,
                    schedule_scheduler_tick_with_deadline(app),
                ]);
            }
            iced::Task::batch(vec![
                persist_task,
                publish_task,
                schedule_scheduler_tick_with_deadline(app),
            ])
        }
        TaskBoardMessage::CopyImportPromptTemplate => {
            let selected_priority = parse_priority_or_default(
                &app.task_board_draft.priority,
                app.task_board_settings.default_priority,
            );
            let template = import_prompt_template(
                app.task_board_import_prompt_format,
                selected_priority,
                &app.task_board_draft.model,
                app.task_board_draft.acp_agent.as_deref(),
            );
            app.push_localized_notification("已复制导入提示词模板", vec![]);
            iced::clipboard::write(template).map(|_: ()| Message::None)
        }
        TaskBoardMessage::ImportFilePick => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                iced::Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .add_filter("导入文件", &["json", "csv", "tsv"])
                            .pick_file()
                            .await
                            .map(|handle| handle.path().to_string_lossy().to_string())
                    },
                    |picked| Message::TaskBoard(TaskBoardMessage::ImportFilePicked(picked)),
                )
            }
            #[cfg(target_arch = "wasm32")]
            {
                app.push_localized_notification("当前平台暂不支持文件选择，请粘贴内容导入", vec![]);
                iced::Task::none()
            }
        }
        TaskBoardMessage::ImportFilePicked(picked) => {
            if let Some(path) = picked {
                return iced::Task::perform(
                    async move {
                        std::fs::read_to_string(&path)
                            .map_err(|e| format!("读取导入文件失败: {}", e))
                    },
                    |result| Message::TaskBoard(TaskBoardMessage::ImportFileLoaded(result)),
                );
            }
            iced::Task::none()
        }
        TaskBoardMessage::ImportFileLoaded(result) => {
            match result {
                Ok(content) => {
                    app.task_board_import_editor =
                        iced::widget::text_editor::Content::with_text(&content);
                    app.push_localized_notification("已将文件内容填入导入表单", vec![]);
                }
                Err(err) => {
                    app.push_notification(err);
                }
            }
            iced::Task::none()
        }

        TaskBoardMessage::ImportTasksSubmitted => {
            let content = app.task_board_import_editor.text().to_string();
            let tasks_to_create = match taskboard::update::forms::tasks_from_import_content(
                &content,
                app.task_board_settings.default_priority,
                &app.task_board_draft,
                initial_task_status_for_app(app),
                MAIN_AGENT_KEY,
            ) {
                Ok(tasks) => tasks,
                Err(error) => {
                    app.push_notification(error.clone());
                    return app.show_error_toast(error);
                }
            };

            if tasks_to_create.is_empty() {
                return iced::Task::none();
            }

            if let Some(project_path) = &app.project_path {
                let path = project_path.to_string();
                let workflow_id = app.task_board_settings.workflow_id.clone();
                let mut tasks = Vec::new();
                for task in tasks_to_create {
                    let p = path.clone();
                    let result_project_path = p.clone();
                    let task_workflow_id = workflow_id.clone();
                    tasks.push(iced::Task::perform(
                        async move { create_task_via_agent_scheduler(p, task, task_workflow_id).await },
                        move |result| match result {
                            Ok(created_task) => Message::TaskBoard(TaskBoardMessage::TaskCreated {
                                project_path: result_project_path.clone(),
                                task: created_task,
                            }),
                            Err(e) => {
                                eprintln!("Failed to create task: {}", e);
                                Message::None
                            }
                        },
                    ));
                }
                return iced::Task::batch(tasks);
            }
            iced::Task::none()
        }
        _ => iced::Task::none(),
    }
}

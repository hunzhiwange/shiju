//! 任务看板桌面适配：委派局部更新，映射会话、通知、持久化及原生操作效果。

use crate::app::Message;
use crate::app::state::MAIN_AGENT_KEY;
use desktop_task_board as taskboard;
use taskboard::update::projection::*;

use super::helpers::*;
use super::*;

#[path = "update/execution_results.rs"]
mod execution_results_update;
#[path = "update/execution.rs"]
mod execution_update;

fn task_board_project_matches(app: &crate::app::App, project_path: &str) -> bool {
    taskboard::update::projection::task_board_project_matches(
        app.navigation_tabs.project_path.as_deref(),
        project_path,
    )
}

fn initial_task_status_for_app(app: &crate::app::App) -> TaskStatus {
    initial_task_status_for_workflow_id(&app.task_board_settings.workflow_id)
}

fn sync_agent_scheduler_status_for_current_project(
    app: &crate::app::App,
) -> iced::Task<crate::app::Message> {
    app.navigation_tabs
        .project_path
        .clone()
        .map(sync_agent_scheduler_status)
        .unwrap_or_else(iced::Task::none)
}

fn agent_scheduler_gateway_unavailable(error: &str) -> bool {
    gateway_http::is_gateway_unavailable_error(error)
}

fn persist_task_metadata_patch_task(
    app: &crate::app::App,
    task: Task,
    action_label: &'static str,
) -> iced::Task<crate::app::Message> {
    let Some(project_path) = app.navigation_tabs.project_path.clone() else {
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
    projection_state(app).status(response)
}
fn apply_agent_scheduler_logs(
    app: &mut crate::app::App,
    responses: Vec<vw_api_agent_scheduler::AgentSchedulerTaskLogsResponse>,
) -> Vec<Task> {
    projection_state(app).log_responses(responses)
}

fn schedule_agent_scheduler_status_followup_ticks(
    app: &mut crate::app::App,
    updated_tasks: &[Task],
) -> iced::Task<crate::app::Message> {
    let (should_schedule_review, should_schedule_merge) =
        taskboard::update::projection::scheduler_followups_for_runtime(
            &app.task_board_settings.workflow_id,
            updated_tasks,
            app.task_board_executor_running,
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
            worktree_panel_expanded: &mut app.task_board_worktree_panel_expanded,
            worktree_pixel_office: &mut app.task_board_worktree_pixel_office,
            project_path: app.navigation_tabs.project_path.as_deref(),
            clear_prompt_after_create: &mut app.task_board_clear_prompt_after_create,
            close_after_create: &mut app.task_board_close_after_create,
            close_after_edit: &mut app.task_board_close_after_edit,
            settings: &app.task_board_settings,
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
        Ok(effect) => return apply_local_effect(app, effect),
        Err(message) => message,
    };

    let bulk_model_options =
        crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(
            app,
            &taskboard::bulk::normalized_bulk_model_input(&app.task_board_bulk_model_input),
        );
    let message = match taskboard::update::bulk::update_bulk(
        bulk_state(app),
        message,
        &bulk_model_options,
        MAIN_AGENT_KEY,
    ) {
        Err(message) => message,
        Ok((effect, sync_logs)) => {
            for id in sync_logs {
                sync_viewing_logs(app, &id);
            }
            use taskboard::update::bulk::BulkEffect;
            if matches!(&effect, BulkEffect::None) {
                return iced::Task::none();
            }
            let merge = matches!(&effect, BulkEffect::Merge(_));
            let Some(path) = app.navigation_tabs.project_path.clone() else {
                return if merge {
                    iced::Task::done(Message::TaskBoard(TaskBoardMessage::ManualMergeQueueTick))
                } else {
                    iced::Task::none()
                };
            };
            let workflow_id = app.task_board_settings.workflow_id.clone();
            let recycle = match &effect {
                BulkEffect::Archive(ids) | BulkEffect::Delete(ids) => {
                    recycle_assigned_task_worktrees_for_removal(app, ids)
                }
                _ => iced::Task::none(),
            };
            let action = iced::Task::perform(
                async move {
                    match effect {
                        BulkEffect::None => return Ok(()),
                        BulkEffect::Persist(tasks, label) => {
                            persist_updated_tasks_via_gateway(&path, &tasks, label, &workflow_id)
                                .await
                        }
                        BulkEffect::Merge(tasks) => {
                            persist_updated_tasks_via_gateway(
                                &path,
                                &tasks,
                                "手动批量合并入队",
                                &workflow_id,
                            )
                            .await
                        }
                        BulkEffect::Archive(ids) => {
                            send_archive_commands_via_gateway(&path, &ids).await
                        }
                        BulkEffect::Delete(ids) => delete_tasks_via_gateway(&path, &ids).await,
                        BulkEffect::Move(ids, status) => {
                            move_tasks_to_status_via_gateway(&path, &ids, status, &workflow_id)
                                .await
                        }
                    }
                },
                |result| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(result)),
            );
            let tick = if merge {
                iced::Task::done(Message::TaskBoard(TaskBoardMessage::ManualMergeQueueTick))
            } else {
                iced::Task::none()
            };
            return iced::Task::batch([action, recycle, tick]);
        }
    };

    let project_path = app.navigation_tabs.project_path.clone();
    let message = match taskboard::update::manual_worktree::update_manual_worktree(
        taskboard::update::manual_worktree::ManualWorktreeState {
            task_board_worktree_action_log_rx: &mut app.task_board_worktree_action_log_rx,
            task_board_worktree_action_logs: &mut app.task_board_worktree_action_logs,
            task_board_worktree_action_logs_visible_until_ms: &mut app
                .task_board_worktree_action_logs_visible_until_ms,
            task_board_worktree_manual_action_kind: &mut app.task_board_worktree_manual_action_kind,
            task_board_worktree_manual_confirm_kind: &mut app
                .task_board_worktree_manual_confirm_kind,
            task_board_worktree_snapshot: &mut app.task_board_worktree_snapshot,
        },
        project_path,
        message,
    ) {
        Err(message) => message,
        Ok(effects) => return apply_worktree_effects(app, effects),
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
                app.navigation_tabs.screen = if app.navigation_tabs.project_path.is_some() {
                    crate::app::Screen::Project
                } else {
                    crate::app::Screen::Home
                };
                app.task_board_worktree_snapshot_loading = false;
                return iced::Task::none();
            }

            if app.navigation_tabs.project_path.is_none() {
                return iced::Task::none();
            }

            app.show_task_board = true;
            app.navigation_tabs.screen = crate::app::Screen::TaskBoard;
            if let Some(project_path) = &app.navigation_tabs.project_path {
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
            app.navigation_tabs.screen = if app.navigation_tabs.project_path.is_some() {
                crate::app::Screen::Project
            } else {
                crate::app::Screen::Home
            };
            taskboard::update::local::close_board(
                &mut app.task_board_worktree_snapshot_loading,
                &mut app.task_board_bulk_active_status,
                &mut app.task_board_selected_tasks,
            );
            iced::Task::none()
        }
        TaskBoardMessage::LoadTasks => {
            taskboard::helpers::settings::refresh_deadline(
                &mut app.task_board_next_refresh_at_ms,
                &app.task_board_settings,
            );
            let refresh_snapshot = maybe_schedule_worktree_snapshot_refresh(app, false);
            if let Some(project_path) = &app.navigation_tabs.project_path {
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
            projection_state(app).loaded(tasks);
            iced::Task::none()
        }

        TaskBoardMessage::TaskStatusChanged { task_id, new_status } => {
            if let Some(project_path) = &app.navigation_tabs.project_path {
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

            if let Some(project_path) = &app.navigation_tabs.project_path {
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

            if let Some(project_path) = &app.navigation_tabs.project_path {
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

        TaskBoardMessage::ManualMergeQueueTick => start_manual_merge_queue_task(app),
        TaskBoardMessage::BulkActionCompleted(result) => {
            let messages = taskboard::update::bulk::completion_messages(
                app.task_board_manual_merge_queue.is_empty(),
            );
            if let Err(error) = result {
                app.push_notification(error);
            }
            iced::Task::batch(
                messages.into_iter().map(|message| iced::Task::done(Message::TaskBoard(message))),
            )
        }
        TaskBoardMessage::MoveTaskStatusCompleted { target_status, result } => {
            let messages = taskboard::update::bulk::move_completion_messages(
                &result,
                target_status,
                app.task_board_executor_running,
            );
            if let Err(error) = result {
                app.push_notification(error);
            }
            iced::Task::batch(
                messages.into_iter().map(|message| iced::Task::done(Message::TaskBoard(message))),
            )
        }

        TaskBoardMessage::WorktreePoolMaintained { project_path, result } => {
            if !task_board_project_matches(app, &project_path) {
                return iced::Task::none();
            }
            let effect = taskboard::update::worktree::SnapshotState {
                snapshot: &mut app.task_board_worktree_snapshot,
                loading: &mut app.task_board_worktree_snapshot_loading,
                last_refresh: &mut app.task_board_last_worktree_snapshot_at_ms,
            }
            .maintained(
                &mut app.task_board_worktree_maintenance_in_flight,
                &mut app.task_board_tasks,
                app.project_is_git_repo,
                app.navigation_tabs.project_path.is_some(),
                result,
                now_ms(),
            );
            match effect {
                taskboard::update::worktree::MaintenanceEffect::None => iced::Task::none(),
                taskboard::update::worktree::MaintenanceEffect::Refresh => {
                    maybe_schedule_worktree_snapshot_refresh(app, true)
                }
                taskboard::update::worktree::MaintenanceEffect::Persist => app
                    .navigation_tabs
                    .project_path
                    .as_deref()
                    .map(|path| iced::Task::batch(build_persist_tasks(path, &app.task_board_tasks)))
                    .unwrap_or_else(iced::Task::none),
            }
        }

        TaskBoardMessage::WorktreeSnapshotLoaded { project_path, result } => {
            if !task_board_project_matches(app, &project_path) {
                return iced::Task::none();
            }
            let is_git = app.project_is_git_repo;
            snapshot_state(app).loaded(is_git, result, now_ms());
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
            projection_state(app).acknowledge(&task_id, &result);
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
            let Some(project_path) = app.navigation_tabs.project_path.clone() else {
                return iced::Task::none();
            };
            let outcome = taskboard::update::rules::finish_planning(
                &mut app.task_board_tasks,
                &task_id,
                final_logs,
                result,
                |task| {
                    let _ = crate::app::task::write_task_plan_files(&project_path, task);
                },
                |task| crate::app::task::assign_task_execution_worktree(&project_path, task, None),
            );
            let task_to_persist = outcome.persist;
            let execute_task_to_start = outcome.execute;
            let plan_failed = outcome.failed;
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

        _ => iced::Task::none(),
    }
}

fn apply_local_effect(
    app: &mut crate::app::App,
    effect: taskboard::update::local::LocalEffect,
) -> iced::Task<Message> {
    use taskboard::update::local::LocalEffect;
    match effect {
        LocalEffect::OpenSession(task_id) => {
            app.show_task_board = false;
            iced::Task::done(Message::TaskBoard(TaskBoardMessage::TaskStatusChanged {
                task_id,
                new_status: TaskStatus::Pending,
            }))
        }
        LocalEffect::OpenLogs(task) => {
            let id = task.id.clone();
            set_viewing_logs(app, Some(task));
            flush_running_task_logs_throttled(app, true, web_time::Instant::now());
            app.navigation_tabs
                .project_path
                .clone()
                .map(|path| sync_agent_scheduler_logs_for_project_path(path, vec![id]))
                .unwrap_or_else(iced::Task::none)
        }
        LocalEffect::SyncLogs => {
            let viewing = app.task_board_viewing_logs.clone();
            sync_task_logs_editor(app, viewing.as_ref());
            match (app.navigation_tabs.project_path.clone(), viewing) {
                (Some(path), Some(task)) => {
                    sync_agent_scheduler_logs_for_project_path(path, vec![task.id])
                }
                _ => iced::Task::none(),
            }
        }
        LocalEffect::Preference(key, value) => {
            app.save_desktop_preference(key, serde_json::Value::Bool(value))
        }
        LocalEffect::RefreshSnapshot => maybe_schedule_worktree_snapshot_refresh(app, true),
        LocalEffect::Clipboard(template) => {
            app.push_localized_notification("已复制导入提示词模板", vec![]);
            iced::clipboard::write(template).map(|_: ()| Message::None)
        }
        LocalEffect::ImportLoaded(result) => {
            match result {
                Ok(()) => app.push_localized_notification("已将文件内容填入导入表单", vec![]),
                Err(error) => app.push_notification(error),
            }
            iced::Task::none()
        }
        LocalEffect::Import(result) => match result {
            Err(error) => {
                app.push_notification(error.clone());
                app.show_error_toast(error)
            }
            Ok(tasks) => iced::Task::batch(
                tasks
                    .into_iter()
                    .map(|task| apply_local_effect(app, LocalEffect::Create(task)))
                    .collect::<Vec<_>>(),
            ),
        },
        LocalEffect::None => iced::Task::none(),
        LocalEffect::ViewLogs(task) => {
            set_viewing_logs(app, task);
            iced::Task::none()
        }
        LocalEffect::ClearSuccess(create) => crate::app::message::after(
            std::time::Duration::from_secs(2),
            Message::TaskBoard(if create {
                TaskBoardMessage::ClearCreateSubmitSuccess
            } else {
                TaskBoardMessage::ClearEditSubmitSuccess
            }),
        ),
        LocalEffect::Persist(task) => {
            let Some(path) = app.navigation_tabs.project_path.clone() else {
                return iced::Task::none();
            };
            let workflow_id = app.task_board_settings.workflow_id.clone();
            iced::Task::perform(
                async move {
                    persist_updated_tasks_via_gateway(&path, &[task], "保存任务编辑", &workflow_id)
                        .await
                },
                |result| Message::TaskBoard(TaskBoardMessage::EditingTaskSaved(result)),
            )
        }
        LocalEffect::Create(task) => {
            let Some(path) = app.navigation_tabs.project_path.clone() else {
                return iced::Task::none();
            };
            let result_project_path = path.clone();
            let workflow_id = app.task_board_settings.workflow_id.clone();
            iced::Task::perform(
                async move { create_task_via_agent_scheduler(path, task, workflow_id).await },
                move |result| match result {
                    Ok(task) => Message::TaskBoard(TaskBoardMessage::TaskCreated {
                        project_path: result_project_path.clone(),
                        task,
                    }),
                    Err(e) => {
                        eprintln!("Failed to create task: {}", e);
                        Message::None
                    }
                },
            )
        }
        LocalEffect::Delete(id) => {
            let Some(path) = app.navigation_tabs.project_path.clone() else {
                return iced::Task::none();
            };
            let recycle = recycle_assigned_task_worktree_for_removal(app, &id);
            let command = iced::Task::perform(
                async move { delete_tasks_via_gateway(&path, &[id]).await },
                |r| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(r)),
            );
            iced::Task::batch([command, recycle])
        }
        LocalEffect::Archive(ids) => {
            let Some(path) = app.navigation_tabs.project_path.clone() else {
                return iced::Task::none();
            };
            let recycle = recycle_assigned_task_worktrees_for_removal(app, &ids);
            let command = iced::Task::perform(
                async move { send_archive_commands_via_gateway(&path, &ids).await },
                |r| Message::TaskBoard(TaskBoardMessage::BulkActionCompleted(r)),
            );
            iced::Task::batch([command, recycle])
        }
        LocalEffect::Move(id, status, order_no) => {
            let Some(path) = app.navigation_tabs.project_path.clone() else {
                return iced::Task::none();
            };
            let workflow_id = app.task_board_settings.workflow_id.clone();
            iced::Task::perform(
                async move {
                    move_task_status_via_gateway(&path, &id, status, &workflow_id, order_no).await
                },
                move |result| {
                    Message::TaskBoard(TaskBoardMessage::MoveTaskStatusCompleted {
                        target_status: status,
                        result,
                    })
                },
            )
        }
    }
}

fn apply_worktree_effects(
    app: &mut crate::app::App,
    effects: Vec<taskboard::update::manual_worktree::WorktreeEffect>,
) -> iced::Task<Message> {
    use taskboard::update::manual_worktree::WorktreeEffect;
    let mut tasks = Vec::new();
    for effect in effects {
        let task = match effect {
            WorktreeEffect::Notify(text, args) => {
                app.push_localized_notification(text, args);
                iced::Task::none()
            }
            WorktreeEffect::Toast(kind, text, args) => app.show_localized_toast(kind, text, args),
            WorktreeEffect::ExpandNotifications => {
                app.notifications_expanded = true;
                iced::Task::none()
            }
            WorktreeEffect::Cleanup(path, sender) => iced::Task::perform(
                async move {
                    drop(sender);
                    cleanup_office_contexts(path).await
                },
                |r| Message::TaskBoard(TaskBoardMessage::CleanAllWorktreesCompleted(r)),
            ),
            WorktreeEffect::Delete => iced::Task::perform(
                async {
                    Err("办公室不直接删除目录；请配置显式删除任务或工作流".to_string())
                },
                |r| Message::TaskBoard(TaskBoardMessage::DeleteAllWorktreesCompleted(r)),
            ),
            WorktreeEffect::Merge => iced::Task::perform(
                async {
                    Err("办公室不直接合并分支；请配置显式 Git 任务或工作流".to_string())
                },
                |r| Message::TaskBoard(TaskBoardMessage::CommitMergeAllWorktreesCompleted(r)),
            ),
            WorktreeEffect::Recycle(path, id) => iced::Task::perform(
                recycle_worktree_slot_via_gateway(path, id),
                |(slot_id, result)| {
                    Message::TaskBoard(TaskBoardMessage::WorktreeSlotRecycled { slot_id, result })
                },
            ),
            WorktreeEffect::Refresh(force) => maybe_schedule_worktree_snapshot_refresh(app, force),
            WorktreeEffect::LogTick => schedule_worktree_action_log_tick(),
        };
        tasks.push(task);
    }
    iced::Task::batch(tasks)
}

fn projection_state(
    app: &mut crate::app::App,
) -> taskboard::update::projection::BoardProjection<'_> {
    taskboard::update::projection::BoardProjection {
        logs: desktop_task_board::helpers::logs::TaskLogsState {
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
        executor: &mut app.task_board_executor,
        pending: &mut app.task_board_tasks_pending_persistence,
        selected: &mut app.task_board_selected_tasks,
        loading: &mut app.task_board_loading,
        settings: &mut app.task_board_settings,
    }
}

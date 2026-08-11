//! 处理任务看板状态更新分支，将 UI 消息转换为应用状态变更和异步任务。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;
use vw_gateway_client::vw_api_types::agent_scheduler::{
    AgentSchedulerCommandKindDto, AgentSchedulerWakeResponse,
};
use vw_gateway_client::vw_api_types::common::TimestampMs;

fn should_continue_existing_session_for_manual_retry(task: &Task, manual: bool) -> bool {
    manual && task.status == TaskStatus::Failed
}

fn add_agent_scheduler_command_log_once(task: &mut Task, message: &str) {
    if !task.logs.iter().rev().take(16).any(|entry| entry.message == message) {
        task.add_log(message.to_string());
    }
}

fn wake_agent_scheduler(
    app: &crate::app::App,
    mapper: fn(Result<AgentSchedulerWakeResponse, String>) -> TaskBoardMessage,
) -> iced::Task<crate::app::Message> {
    let Some(project_path) = app.project_path.clone() else {
        return iced::Task::none();
    };
    let now_ms = crate::app::time::now_ms();
    let dispatch_mode = app.task_board_settings.agent_scheduler_dispatch_mode;
    let max_concurrent = Some(app.task_board_settings.max_concurrent.clamp(1, 10));
    iced::Task::perform(
        async move {
            let scope = resolve_task_board_scheduler_scope(&project_path).await?;
            let client = crate::app::config::gateway_client()?;
            let response = client
                .agent_scheduler_wake(
                    &scope,
                    &vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerWakeRequest {
                        now_ms: TimestampMs(i64::try_from(now_ms).unwrap_or(i64::MAX)),
                        dispatch_mode,
                        max_concurrent,
                        auto_promote_pool_tasks: true,
                    },
                )
                .await?;
            if response.outbox_message_id.is_some() {
                drain_agent_scheduler_outbox(&client, &scope, now_ms).await?;
            }
            Ok(response)
        },
        move |result| Message::TaskBoard(mapper(result)),
    )
}

fn agent_scheduler_lifecycle_error(app: &crate::app::App) -> Option<String> {
    let workflow_id = current_staff_workflow_id(app);
    task_workflow_by_id(&workflow_id)
        .is_none()
        .then(|| format!("Agent Scheduler 已阻塞：任务生命周期未加载: {workflow_id}"))
}

fn wake_agent_scheduler_after_registering_pending(
    app: &crate::app::App,
    mapper: fn(Result<AgentSchedulerWakeResponse, String>) -> TaskBoardMessage,
) -> iced::Task<crate::app::Message> {
    let Some(project_path) = app.project_path.clone() else {
        return iced::Task::none();
    };
    if let Some(error) = agent_scheduler_lifecycle_error(app) {
        tracing::warn!(error = %error, "agent scheduler pending registration skipped");
        return iced::Task::none();
    }
    let now_ms = crate::app::time::now_ms();
    let dispatch_mode = app.task_board_settings.agent_scheduler_dispatch_mode;
    let max_concurrent = Some(app.task_board_settings.max_concurrent.clamp(1, 10));
    let workflow_id = current_staff_workflow_id(app);
    let pending_create_requests =
        agent_scheduler_wake_create_task_requests(&app.task_board_tasks, &workflow_id);
    iced::Task::perform(
        async move {
            let scope = resolve_task_board_scheduler_scope(&project_path).await?;
            let client = crate::app::config::gateway_client()?;
            register_pending_agent_scheduler_tasks(&client, &scope, &pending_create_requests)
                .await?;
            let response = client
                .agent_scheduler_wake(
                    &scope,
                    &vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerWakeRequest {
                        now_ms: TimestampMs(i64::try_from(now_ms).unwrap_or(i64::MAX)),
                        dispatch_mode,
                        max_concurrent,
                        auto_promote_pool_tasks: true,
                    },
                )
                .await?;
            if response.outbox_message_id.is_some() {
                drain_agent_scheduler_outbox(&client, &scope, now_ms).await?;
            }
            Ok(response)
        },
        move |result| Message::TaskBoard(mapper(result)),
    )
}

fn dispatch_agent_scheduler_execution_wake(
    app: &mut crate::app::App,
    _response: AgentSchedulerWakeResponse,
) -> iced::Task<crate::app::Message> {
    iced::Task::batch(vec![
        sync_agent_scheduler_status_for_current_project(app),
        schedule_scheduler_tick_with_deadline(app),
    ])
}

fn dispatch_agent_scheduler_promote_wake(
    app: &mut crate::app::App,
    _response: AgentSchedulerWakeResponse,
) -> iced::Task<crate::app::Message> {
    iced::Task::batch(vec![
        sync_agent_scheduler_status_for_current_project(app),
        schedule_auto_promote_tick_with_deadline(app),
    ])
}

fn dispatch_agent_scheduler_merge_wake(
    app: &mut crate::app::App,
    _response: AgentSchedulerWakeResponse,
) -> iced::Task<crate::app::Message> {
    iced::Task::batch(vec![
        sync_agent_scheduler_status_for_current_project(app),
        schedule_auto_merge_tick_with_deadline(app),
    ])
}

/// 执行 update 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn update(
    app: &mut crate::app::App,
    message: TaskBoardMessage,
) -> iced::Task<crate::app::Message> {
    dispatch_task_board_messages!(message,
    TaskBoardMessage::StartExecution => {
        if !app.project_is_git_repo {
            return app.show_error_toast("当前项目不是 Git 仓库，无法启动依赖 worktree 的任务调度");
        }
        app.task_board_executor_running = true;
        app.task_board_last_log_flush_at_ms = 0;
        app.task_board_log_scan_cursor = 0;
        app.task_board_timeout_scan_cursor = 0;
        app.task_board_schedule_scan_cursor = 0;
        app.task_board_next_scheduler_tick_at_ms = next_deadline_ms(scheduler_tick_interval_secs(app));
        app.task_board_next_auto_review_tick_at_ms =
            next_deadline_ms(TASK_AUTO_CODE_REVIEW_TICK_INTERVAL_SECS);
        iced::Task::batch(vec![
            iced::Task::done(Message::TaskBoard(TaskBoardMessage::ExecutionTick)),
            schedule_auto_review_tick_with_deadline(app),
        ])
    }
    TaskBoardMessage::StopExecution => {
        app.task_board_executor_running = false;
        iced::Task::none()
    }
        TaskBoardMessage::ExecutionTick => {
        let tick_started_at = web_time::Instant::now();
        flush_running_task_logs_throttled(app, false, tick_started_at);

        if !app.task_board_executor_running {
            return iced::Task::none();
        }

            wake_agent_scheduler_after_registering_pending(
                app,
                TaskBoardMessage::AgentSchedulerExecutionWakeCompleted,
            )
        }
        TaskBoardMessage::AgentSchedulerExecutionWakeCompleted(result) => {
            let response = match result {
                Ok(response) => response,
                Err(error) => {
                    if agent_scheduler_gateway_unavailable(&error) {
                        tracing::debug!(
                            error = %error,
                            "agent scheduler execution wake skipped because gateway is unavailable"
                        );
                    } else {
                        tracing::warn!(error = %error, "agent scheduler execution wake failed");
                    }
                    return schedule_scheduler_tick_with_deadline(app);
                }
            };
            dispatch_agent_scheduler_execution_wake(app, response)
        }
    TaskBoardMessage::AgentSchedulerCommandInFlightStatusTick { project_path, task_id } => {
        if !task_board_project_matches(app, &project_path) {
            return iced::Task::none();
        }
        let Some(task) = app
            .task_board_tasks
            .iter_mut()
            .find(|task| task.id == task_id && task.status == TaskStatus::PrSubmitted)
        else {
            return iced::Task::none();
        };
        add_agent_scheduler_command_log_once(
            task,
            "Agent Scheduler 合并命令等待返回，正在同步网关状态/日志",
        );
        sync_viewing_logs(app, &task_id);
        iced::Task::batch(vec![
            sync_agent_scheduler_status(project_path.clone()),
            schedule_agent_scheduler_command_in_flight_status_sync(project_path, task_id),
        ])
    }
    TaskBoardMessage::AgentSchedulerMergeReadyPersisted { project_path, task, result } => {
        if !task_board_project_matches(app, &project_path) {
            return iced::Task::none();
        }
        let task_id = task.id.clone();
        if let Some(task_in_list) = app.task_board_tasks.iter_mut().find(|item| item.id == task_id)
        {
            match result {
                Ok(()) => {
                    add_agent_scheduler_command_log_once(
                        task_in_list,
                        "已发送 Agent Scheduler 合并命令",
                    );
                }
                Err(error) => {
                    task_in_list.mark_paused(format!(
                        "本地合并状态保存失败，未发送 Agent Scheduler 合并命令: {error}"
                    ));
                    sync_viewing_logs(app, &task_id);
                    return iced::Task::none();
                }
            }
        }
            sync_viewing_logs(app, &task_id);
            if let Some(error) = agent_scheduler_lifecycle_error(app) {
                if let Some(task) = app.task_board_tasks.iter_mut().find(|t| t.id == task_id) {
                    task.mark_paused(error.clone());
                }
                return app.show_error_toast(error);
            }
            iced::Task::batch(vec![
                send_agent_scheduler_command(
                    project_path.clone(),
                    task,
                    current_staff_workflow_id(app),
                AgentSchedulerCommandKindDto::Merge,
                app.task_board_settings.agent_scheduler_dispatch_mode,
                None,
                Some("自动审核已跳过，进入合并阶段".to_string()),
            ),
            schedule_agent_scheduler_command_in_flight_status_sync(project_path, task_id),
        ])
    }
    TaskBoardMessage::AutoCodeReviewTick => {
            if !app.task_board_executor_running {
                return iced::Task::none();
            }
            iced::Task::batch(vec![
                wake_agent_scheduler(app, TaskBoardMessage::AgentSchedulerExecutionWakeCompleted),
                schedule_auto_review_tick_with_deadline(app),
            ])
        }
    TaskBoardMessage::SimulateStep { task_id } => {
        if !app.task_board_executor_running {
            return iced::Task::none();
        }

        if let Some(task) = app.task_board_tasks.iter().find(|t| t.id == task_id) {
            let current_status = task.status;
            if let Some(new_status) = crate::app::task::simulate_task_execution_step(
                &app.project_path.clone().unwrap_or_default(),
                &task_id,
                current_status,
            ) {
                app.task_board_executor.finish_task(&task_id);
                return iced::Task::batch(vec![
                    iced::Task::done(Message::TaskBoard(TaskBoardMessage::TaskStatusChanged {
                        task_id: task_id.clone(),
                        new_status,
                    })),
                    schedule_scheduler_tick_with_deadline(app),
                ]);
            }
        }

        app.task_board_executor.finish_task(&task_id);
        schedule_scheduler_tick_with_deadline(app)
        }
        TaskBoardMessage::PromotePoolTasksTick => {
            if app.project_path.is_none() {
                return schedule_auto_promote_tick_with_deadline(app);
            }
            iced::Task::batch(vec![
                wake_agent_scheduler_after_registering_pending(
                    app,
                    TaskBoardMessage::AgentSchedulerPoolWakeCompleted,
                ),
            ])
        }
        TaskBoardMessage::AgentSchedulerPoolWakeCompleted(result) => {
            let response = match result {
                Ok(response) => response,
                Err(error) => {
                    tracing::warn!(error = %error, "agent scheduler promote wake failed");
                    return schedule_auto_promote_tick_with_deadline(app);
                }
            };
            dispatch_agent_scheduler_promote_wake(app, response)
        }
    TaskBoardMessage::AutoMergeTick => {
        if !app.task_board_executor_running || app.project_path.is_none() {
            return iced::Task::none();
        }
            wake_agent_scheduler(app, TaskBoardMessage::AgentSchedulerAutoMergeWakeCompleted)
        }
        TaskBoardMessage::AgentSchedulerAutoMergeWakeCompleted(result) => {
            let response = match result {
                Ok(response) => response,
                Err(error) => {
                    tracing::warn!(error = %error, "agent scheduler merge wake failed");
                    return schedule_auto_merge_tick_with_deadline(app);
                }
            };
            dispatch_agent_scheduler_merge_wake(app, response)
        }
        TaskBoardMessage::ExecuteTask { task_id } => {
            if !app.project_is_git_repo {
                return app.show_error_toast(
                    "当前项目不是 Git 仓库，无法执行依赖 worktree 的任务",
                );
            }
            if let Some(task) = app.task_board_tasks.iter().find(|t| t.id == task_id).cloned() {
                if let Some(project_path) = &app.project_path {
                    let path = project_path.clone();
                    if app.task_board_executor.is_running(&task_id) {
                        if let Some(task_in_list) =
                            app.task_board_tasks.iter_mut().find(|t| t.id == task_id)
                        {
                            task_in_list.add_log(
                                "任务已在执行或等待工作区，忽略重复启动".to_string(),
                            );
                        }
                        sync_viewing_logs(app, &task_id);
                        return iced::Task::none();
                    }
                    if let Some(task_in_list) =
                        app.task_board_tasks.iter_mut().find(|t| t.id == task_id)
                    {
                        let preparing_log =
                            if should_continue_existing_session_for_manual_retry(&task, true) {
                                "手工重试任务，已发送 Agent Scheduler 重试命令"
                            } else {
                                "已发送 Agent Scheduler 启动命令"
                            };
                        add_agent_scheduler_command_log_once(task_in_list, preparing_log);
                    }
                    sync_viewing_logs(app, &task_id);
                    let command =
                        if should_continue_existing_session_for_manual_retry(&task, true) {
                            AgentSchedulerCommandKindDto::Retry
                        } else {
                            AgentSchedulerCommandKindDto::Start
                        };
                    if let Some(error) = agent_scheduler_lifecycle_error(app) {
                        if let Some(task_in_list) =
                            app.task_board_tasks.iter_mut().find(|t| t.id == task_id)
                        {
                            task_in_list.mark_execution_failed(error.clone());
                            sync_viewing_logs(app, &task_id);
                        }
                        return app.show_error_toast(error);
                    }
                    return send_agent_scheduler_command(
                        path,
                        task,
                        current_staff_workflow_id(app),
                        command,
                        app.task_board_settings.agent_scheduler_dispatch_mode,
                        Some(app.task_board_settings.max_concurrent.clamp(1, 10)),
                        None,
                    );
                } else if let Some(task_in_list) =
                    app.task_board_tasks.iter_mut().find(|t| t.id == task_id)
                {
                    task_in_list.mark_execution_failed("缺少项目路径".to_string());
                    sync_viewing_logs(app, &task_id);
                }
            }
            iced::Task::none()
        }
        )
}
#[cfg(test)]
#[path = "execution_tests.rs"]
mod execution_tests;

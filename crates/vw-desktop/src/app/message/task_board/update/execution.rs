use super::*;
use taskboard::update::execution::{ExecutionState, ExecutionEffect, WakeKind};
use vw_api_agent_scheduler::AgentSchedulerWakeResponse;
use vw_api_transport::common::TimestampMs;
fn wake_agent_scheduler_after_registering_pending(
    app: &crate::app::App,
    mapper: fn(Result<AgentSchedulerWakeResponse, String>) -> TaskBoardMessage,
) -> iced::Task<crate::app::Message> {
    let Some(project_path) = app.navigation_tabs.project_path.clone() else {
        return iced::Task::none();
    };
    if let Some(error) = taskboard::update::execution::agent_scheduler_lifecycle_error(&app.task_board.task_board_settings.workflow_id) {
        tracing::warn!(error = %error, "agent scheduler pending registration skipped");
        return iced::Task::none();
    }
    let now_ms = crate::app::time::now_ms();
    let dispatch_mode = app.task_board.task_board_settings.agent_scheduler_dispatch_mode;
    let max_concurrent = Some(app.task_board.task_board_settings.max_concurrent.clamp(1, 10));
    let workflow_id = app.task_board.task_board_settings.workflow_id.clone();
    let pending_create_requests =
        agent_scheduler_wake_create_task_requests(&app.task_board.task_board_tasks, &workflow_id);
    iced::Task::perform(
        async move {
            let scope = resolve_task_board_scheduler_scope(&project_path).await?;
            let client = crate::app::config::gateway_client()?;
            register_pending_agent_scheduler_tasks(&client, &scope, &pending_create_requests)
                .await?;
            let response = gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
                .agent_scheduler_wake(
                    &scope,
                    &vw_api_agent_scheduler::AgentSchedulerWakeRequest {
                        now_ms: TimestampMs(i64::try_from(now_ms).unwrap_or(i64::MAX)),
                        dispatch_mode,
                        max_concurrent,
                        auto_promote_pool_tasks: true,
                    },
                )
                .await?;
            Ok(response)
        },
        move |result| Message::TaskBoard(mapper(result)),
    )
}

pub(super) fn update(app: &mut crate::app::App, message: TaskBoardMessage) -> iced::Task<Message> {
    if matches!(&message, TaskBoardMessage::ExecutionTick) {
        flush_running_task_logs_throttled(app, false, web_time::Instant::now());
    }
    let deadline = next_deadline_ms(scheduler_tick_interval_secs(app));
    let mut state = ExecutionState {
        project_path: &mut app.navigation_tabs.project_path,
        task_board_settings: &mut app.task_board.task_board_settings,
        task_board_tasks: &mut app.task_board.task_board_tasks,
        task_board_executor: &mut app.task_board.task_board_executor,
        task_board_executor_running: &mut app.task_board.task_board_executor_running,
        task_board_last_log_flush_at_ms: &mut app.task_board.task_board_last_log_flush_at_ms,
        task_board_log_scan_cursor: &mut app.task_board.task_board_log_scan_cursor,
        task_board_timeout_scan_cursor: &mut app.task_board.task_board_timeout_scan_cursor,
        task_board_schedule_scan_cursor: &mut app.task_board.task_board_schedule_scan_cursor,
        task_board_next_scheduler_tick_at_ms: &mut app.task_board.task_board_next_scheduler_tick_at_ms,
        next_deadline_ms: deadline, effects: vec![],
    };
    let result = taskboard::update::execution::update(&mut state, message);
    let mut effects = std::mem::take(&mut state.effects);
    effects.extend(result);
    drop(state);
    let mut tasks = Vec::new();
    for effect in effects {
        match effect {
            ExecutionEffect::Message(message) => tasks.push(iced::Task::done(Message::TaskBoard(message))),
            ExecutionEffect::Error(error) => tasks.push(app.show_error_toast(error)),
            ExecutionEffect::SyncLogs(task) => {
                if let Some(task) = task {
                    if app.task_board.task_board_viewing_logs.as_ref().is_some_and(|viewing| viewing.id == task.id) {
                        set_viewing_logs(app, Some(task));
                    } else {
                        refresh_task_log_cache(app, &task);
                    }
                }
            },
            ExecutionEffect::Maintain(path) => tasks.push(schedule_worktree_pool_maintenance(app, &path, 0)),
            ExecutionEffect::ScheduleTick => tasks.push(schedule_scheduler_tick_with_deadline(app)),
            ExecutionEffect::SchedulePromote => tasks.push(schedule_auto_promote_tick_with_deadline(app)),
            ExecutionEffect::ScheduleMerge => tasks.push(schedule_auto_merge_tick_with_deadline(app)),
            ExecutionEffect::SyncStatus(path) => tasks.push(sync_agent_scheduler_status(path)),
            ExecutionEffect::SyncInFlight(path, id) => tasks.push(schedule_agent_scheduler_command_in_flight_status_sync(path, id)),
            ExecutionEffect::Command(path, task, workflow, kind, mode, max, reason) => tasks.push(send_agent_scheduler_command(path, task, workflow, kind, mode, max, reason)),
            ExecutionEffect::Wake(kind) => tasks.push(wake_agent_scheduler_after_registering_pending(app, match kind {
                WakeKind::Execution => TaskBoardMessage::AgentSchedulerExecutionWakeCompleted,
                WakeKind::Promote => TaskBoardMessage::AgentSchedulerPoolWakeCompleted,
                WakeKind::Merge => TaskBoardMessage::AgentSchedulerAutoMergeWakeCompleted,
            })),
            ExecutionEffect::WakeCompleted(kind) => {
                tasks.push(sync_agent_scheduler_status_for_current_project(app));
                tasks.push(match kind {
                    WakeKind::Execution => schedule_scheduler_tick_with_deadline(app),
                    WakeKind::Promote => schedule_auto_promote_tick_with_deadline(app),
                    WakeKind::Merge => schedule_auto_merge_tick_with_deadline(app),
                });
            }
        }
    }
    iced::Task::batch(tasks)
}
#[cfg(test)]
#[path = "execution_tests.rs"]
mod execution_tests;

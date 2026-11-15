//! 提供任务看板消息处理过程中复用的局部辅助逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;

pub(crate) use desktop_task_board::helpers::execution::*;
pub(crate) fn push_worktree_action_log(app: &mut crate::app::App, line: String) {
    desktop_task_board::helpers::worktree_logs::push_worktree_action_log(&mut worktree_logs_state(app), line)
}

pub(crate) fn clear_worktree_action_logs(app: &mut crate::app::App) {
    desktop_task_board::helpers::worktree_logs::clear_worktree_action_logs(&mut worktree_logs_state(app))
}

pub(crate) fn finish_worktree_action_logs(app: &mut crate::app::App) {
    desktop_task_board::helpers::worktree_logs::finish_worktree_action_logs(&mut worktree_logs_state(app))
}

pub(crate) fn poll_worktree_action_logs(app: &mut crate::app::App) {
    desktop_task_board::helpers::worktree_logs::poll_worktree_action_logs(&mut worktree_logs_state(app))
}

pub(crate) fn flush_running_task_logs(
    app: &mut crate::app::App,
    tick_started_at: web_time::Instant,
) {
    let running = app.task_board_executor.running_tasks.clone();
    if running.is_empty() {
        app.task_board_log_scan_cursor = 0;
        return;
    }
    if app.task_board_log_scan_cursor >= running.len() {
        app.task_board_log_scan_cursor = 0;
    }
    let max_batch = TASK_RUNNING_LOG_SCAN_BATCH.min(running.len());
    let viewing_task_id = app.task_board_viewing_logs.as_ref().map(|task| task.id.clone());
    let mut scanned = 0usize;

    while scanned < max_batch {
        if scanned > 0
            && tick_started_at.elapsed()
                >= std::time::Duration::from_millis(TASK_EXECUTION_SCAN_BUDGET_MS)
        {
            break;
        }
        let idx = (app.task_board_log_scan_cursor + scanned) % running.len();
        let task_id = running[idx].clone();
        let task_logs = app.task_board_executor.poll_task_logs(&task_id);
        if let Some(task) = app.task_board_tasks.iter_mut().find(|t| t.id == task_id) {
            let mut plan_changed = false;
            for log in &task_logs {
                plan_changed |= append_task_log_stream(task, log);
            }
            if plan_changed && let Some(project_path) = app.project_path.as_deref() {
                let _ = crate::app::task::write_task_plan_files(project_path, task);
            }
        }
        if viewing_task_id.as_deref() == Some(task_id.as_str()) {
            sync_viewing_logs(app, &task_id);
        }
        scanned += 1;
    }
    if scanned > 0 {
        app.task_board_log_scan_cursor = (app.task_board_log_scan_cursor + scanned) % running.len();
    }
}

pub(crate) fn flush_running_task_logs_throttled(
    app: &mut crate::app::App,
    force: bool,
    tick_started_at: web_time::Instant,
) {
    let now = now_ms();
    if !force
        && now.saturating_sub(app.task_board_last_log_flush_at_ms) < TASK_LOG_FLUSH_INTERVAL_MS
    {
        return;
    }
    flush_running_task_logs(app, tick_started_at);
    app.task_board_last_log_flush_at_ms = now;
}

pub(crate) fn schedule_worktree_action_log_tick() -> iced::Task<crate::app::Message> {
    crate::app::message::after(
        std::time::Duration::from_millis(TASK_BOARD_WORKTREE_ACTION_LOG_TICK_MS),
        crate::app::Message::TaskBoard(TaskBoardMessage::WorktreeActionLogTick),
    )
}

pub(crate) async fn resolve_task_board_project_id(
    project_path: &str,
) -> Result<vw_api_transport::ProjectId, String> {
    let client = crate::app::config::gateway_client()?;
    let response = gateway_project::GatewayProjectClient::new(client.transport().clone()).project_resolve(
            &crate::app::session_gateway::active_tenant_scope(),
            &vw_api_project::ResolveProjectRequest {
                directory: project_path.to_string(),
                create_if_missing: false,
            },
        )
        .await?;
    Ok(response.project.id)
}

pub(crate) async fn resolve_task_board_scheduler_scope(
    project_path: &str,
) -> Result<vw_api_agent_scheduler::AgentSchedulerScopeDto, String> {
    let project_id = resolve_task_board_project_id(project_path).await?;
    crate::app::session_gateway::active_agent_scheduler_scope(project_id)
}

pub(crate) fn sync_agent_scheduler_status(project_path: String) -> iced::Task<crate::app::Message> {
    let result_project_path = project_path.clone();
    iced::Task::perform(
        async move {
            let scope = resolve_task_board_scheduler_scope(&project_path).await?;
            let client = crate::app::config::gateway_client()?;
            gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone()).agent_scheduler_status(&scope).await
        },
        move |result| {
            crate::app::Message::TaskBoard(TaskBoardMessage::AgentSchedulerStatusLoaded {
                project_path: result_project_path.clone(),
                result,
            })
        },
    )
}

async fn load_agent_scheduler_logs(
    project_id: vw_api_transport::ProjectId,
    task_ids: Vec<String>,
) -> Result<
    Vec<vw_api_agent_scheduler::AgentSchedulerTaskLogsResponse>,
    String,
> {
    let client = crate::app::config::gateway_client()?;
    let scope = crate::app::session_gateway::active_agent_scheduler_scope(project_id)?;
    let mut responses = Vec::with_capacity(task_ids.len());
    for task_id in task_ids {
        responses.push(gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone()).agent_scheduler_logs(&scope, &task_id).await?);
    }
    Ok(responses)
}

pub(crate) fn sync_agent_scheduler_logs_for_project_path(
    project_path: String,
    task_ids: Vec<String>,
) -> iced::Task<crate::app::Message> {
    if task_ids.is_empty() {
        return iced::Task::none();
    }

    let result_project_path = project_path.clone();
    iced::Task::perform(
        async move {
            let project_id = resolve_task_board_project_id(&project_path).await?;
            load_agent_scheduler_logs(project_id, task_ids).await
        },
        move |result| {
            crate::app::Message::TaskBoard(TaskBoardMessage::AgentSchedulerLogsLoaded {
                project_path: result_project_path.clone(),
                result,
            })
        },
    )
}

pub(crate) fn sync_agent_scheduler_logs(
    project_path: String,
    project_id: vw_api_transport::ProjectId,
    task_ids: Vec<String>,
) -> iced::Task<crate::app::Message> {
    if task_ids.is_empty() {
        return iced::Task::none();
    }

    let result_project_path = project_path.clone();
    iced::Task::perform(
        async move { load_agent_scheduler_logs(project_id, task_ids).await },
        move |result| {
            crate::app::Message::TaskBoard(TaskBoardMessage::AgentSchedulerLogsLoaded {
                project_path: result_project_path.clone(),
                result,
            })
        },
    )
}

pub(crate) fn schedule_agent_scheduler_command_in_flight_status_sync(
    project_path: String,
    task_id: String,
) -> iced::Task<crate::app::Message> {
    crate::app::message::after(
        std::time::Duration::from_secs(AGENT_SCHEDULER_COMMAND_IN_FLIGHT_SYNC_DELAY_SECS),
        crate::app::Message::TaskBoard(TaskBoardMessage::AgentSchedulerCommandInFlightStatusTick {
            project_path,
            task_id,
        }),
    )
}

pub(crate) fn sync_task_board_gateway_snapshot(
    project_path: String,
) -> iced::Task<crate::app::Message> {
    let result_project_path = project_path.clone();
    iced::Task::perform(
        async move {
            let scope = resolve_task_board_scheduler_scope(&project_path).await?;
            let client = crate::app::config::gateway_client()?;
            gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone()).agent_scheduler_status(&scope).await
        },
        move |result| {
            crate::app::Message::TaskBoard(TaskBoardMessage::AgentSchedulerStatusLoaded {
                project_path: result_project_path.clone(),
                result,
            })
        },
    )
}

pub(crate) async fn create_task_via_agent_scheduler(
    project_path: String,
    task: Task,
    workflow_id: String,
) -> Result<Task, String> {
    let request = agent_scheduler_create_task_request(&task, &workflow_id);
    let scope = resolve_task_board_scheduler_scope(&project_path).await?;
    let client = crate::app::config::gateway_client()?;
    gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone()).agent_scheduler_create_task(&scope, &request).await?;
    Ok(task)
}

pub(crate) async fn register_pending_agent_scheduler_tasks(
    client: &vw_gateway_client::GatewayClient,
    scope: &vw_api_agent_scheduler::AgentSchedulerScopeDto,
    requests: &[vw_api_agent_scheduler::AgentSchedulerCreateTaskRequest],
) -> Result<(), String> {
    for request in requests {
        gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone()).agent_scheduler_create_task(scope, request).await?;
    }
    Ok(())
}

pub(crate) fn send_agent_scheduler_command(
    project_path: String,
    task: Task,
    workflow_id: String,
    command: vw_api_agent_scheduler::AgentSchedulerCommandKindDto,
    dispatch_mode: vw_api_agent_scheduler::AgentSchedulerDispatchModeDto,
    max_concurrent: Option<u32>,
    reason: Option<String>,
) -> iced::Task<crate::app::Message> {
    let result_project_path = project_path.clone();
    iced::Task::perform(
        async move {
            let task_id = task.id.clone();
            let create_request = agent_scheduler_create_task_request(&task, &workflow_id);
            let scope = resolve_task_board_scheduler_scope(&project_path).await?;
            let client = crate::app::config::gateway_client()?;
            let create_response =
                gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone()).agent_scheduler_create_task(&scope, &create_request).await?;
            if !should_send_agent_scheduler_command_after_create(
                command,
                &create_response.task.status,
            ) {
                return gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone()).agent_scheduler_status(&scope).await;
            }
            let command_result = gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
                .agent_scheduler_command(
                    &scope,
	                    &vw_api_agent_scheduler::AgentSchedulerCommandRequest {
	                        task_id: vw_api_transport::TaskId(task_id.clone()),
                        command,
                        dispatch_mode,
                        max_concurrent,
                        reason,
                        payload: Default::default(),
                    },
                )
                .await;
            let response = match command_result {
                Ok(response) => response,
                Err(command_error) => {
                    return gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone()).agent_scheduler_status(&scope).await.map_err(
                        |status_error| {
                            format!(
                                "Agent Scheduler 命令失败: {command_error}; 状态同步失败: {status_error}"
                            )
                        },
                    );
                }
            };
            if !response.accepted {
                return Err(format!("agent scheduler command {:?} was rejected", command));
            }
            gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone()).agent_scheduler_status(&scope).await
        },
        move |result| {
            crate::app::Message::TaskBoard(TaskBoardMessage::AgentSchedulerStatusLoaded {
                project_path: result_project_path.clone(),
                result,
            })
        },
    )
}

fn worktree_snapshot_from_gateway(
    snapshot: vw_api_task::TaskPoolWorktreeSnapshotResponse,
) -> crate::app::task::WorktreePoolSnapshot {
    crate::app::task::WorktreePoolSnapshot::from_gateway_response(snapshot)
}

pub(crate) async fn cleanup_office_contexts(project_path: String) -> Result<usize, String> {
    let project_id = resolve_task_board_project_id(&project_path).await?;
    let client = crate::app::config::gateway_client()?;
    let snapshot = gateway_task_pool::GatewayTaskPoolClient::new(client.transport().clone())
        .task_pool_worktree_snapshot(
            &vw_api_task::TaskPoolWorktreeSnapshotRequest { project_id },
        )
        .await?;
    let snapshot = worktree_snapshot_from_gateway(snapshot);
    let mut count = 0;
    for slot in snapshot.slots {
        if matches!(
            slot.state,
            crate::app::task::WorktreeState::Tainted | crate::app::task::WorktreeState::Dead
        ) {
            let (_, result) =
                recycle_worktree_slot_via_gateway(project_path.clone(), slot.id).await;
            result?;
            count += 1;
        }
    }
    Ok(count)
}

pub(crate) async fn recycle_worktree_slot_via_gateway(
    project_path: String,
    slot_id: String,
) -> (String, Result<(), String>) {
    let result = async {
        let project_id = resolve_task_board_project_id(&project_path).await?;
        let client = crate::app::config::gateway_client()?;
        let response = gateway_task_pool::GatewayTaskPoolClient::new(client.transport().clone())
            .task_pool_worktree_slot_recycle(
                &vw_api_task::TaskPoolWorktreeSlotRecycleRequest {
                    project_id,
                    slot_id: slot_id.clone(),
                },
            )
            .await?;
        if response.recycled {
            Ok(())
        } else {
            Err("gateway 未回收 worktree 槽位".to_string())
        }
    }
    .await;
    (slot_id, result)
}

async fn recycle_task_worktree_via_gateway(
    project_path: String,
    task_id: String,
    taint_reason: Option<String>,
) -> (String, Result<(), String>) {
    let result = async {
        let project_id = resolve_task_board_project_id(&project_path).await?;
        let client = crate::app::config::gateway_client()?;
        let response = gateway_task_pool::GatewayTaskPoolClient::new(client.transport().clone())
            .task_pool_worktree_recycle(
                &vw_api_task::TaskPoolWorktreeRecycleRequest {
                    project_id,
                    task_id: vw_api_transport::TaskId(task_id.clone()),
                    taint_reason,
                },
            )
            .await?;
        if response.recycled { Ok(()) } else { Err("gateway 未回收任务 worktree".to_string()) }
    }
    .await;
    (task_id, result)
}

#[allow(dead_code)]
pub(crate) fn schedule_worktree_pool_maintenance(
    app: &mut crate::app::App,
    project_path: &str,
    _running_tasks: usize,
) -> iced::Task<crate::app::Message> {
    use crate::app::Message;

    let desired_capacity = app
        .task_board_worktree_desired_capacity
        .unwrap_or(app.task_board_settings.max_concurrent)
        .max(1) as usize;
    if app.task_board_worktree_maintenance_in_flight
        || app.task_board_worktree_manual_action_kind.is_some()
    {
        return iced::Task::none();
    }

    app.task_board_worktree_maintenance_in_flight = true;
    let path = project_path.to_string();
    let result_project_path = path.clone();
    iced::Task::perform(
        async move {
            let project_id = resolve_task_board_project_id(&path).await?;
            let client = crate::app::config::gateway_client()?;
            let response = gateway_task_pool::GatewayTaskPoolClient::new(client.transport().clone())
                .task_pool_worktree_maintain(
                    &vw_api_task::TaskPoolWorktreeMaintenanceRequest {
                        project_id,
                        desired_capacity: desired_capacity as u32,
                    },
                )
                .await?;
            Ok(WorktreePoolMaintenanceOutcome {
                idle_count: response.idle_count as usize,
                target_idle_count: response.target_idle_count as usize,
                snapshot: response.snapshot.map(worktree_snapshot_from_gateway),
            })
        },
        move |result| {
            Message::TaskBoard(TaskBoardMessage::WorktreePoolMaintained {
                project_path: result_project_path.clone(),
                result,
            })
        },
    )
}

pub(crate) fn worktree_snapshot_refresh_interval_ms(app: &crate::app::App) -> u64 {
    if app.task_board_worktree_panel_expanded {
        TASK_BOARD_WORKTREE_SNAPSHOT_EXPANDED_INTERVAL_MS
    } else {
        TASK_BOARD_WORKTREE_SNAPSHOT_INTERVAL_MS
    }
}

pub(crate) fn maybe_schedule_worktree_snapshot_refresh(
    app: &mut crate::app::App,
    force: bool,
) -> iced::Task<crate::app::Message> {
    use crate::app::Message;

    let Some(project_path) = app.project_path.clone() else {
        app.task_board_worktree_snapshot = None;
        app.task_board_worktree_snapshot_loading = false;
        app.task_board_last_worktree_snapshot_at_ms = 0;
        return iced::Task::none();
    };

    if app.task_board_worktree_snapshot_loading {
        return iced::Task::none();
    }

    let now = now_ms();
    let interval_ms = worktree_snapshot_refresh_interval_ms(app);
    if !force && now.saturating_sub(app.task_board_last_worktree_snapshot_at_ms) < interval_ms {
        return iced::Task::none();
    }

    app.task_board_worktree_snapshot_loading = true;
    let path = project_path.clone();
    let result_project_path = path.clone();
    iced::Task::perform(
        async move {
            let project_id = resolve_task_board_project_id(&path).await?;
            let client = crate::app::config::gateway_client()?;
            let snapshot = gateway_task_pool::GatewayTaskPoolClient::new(client.transport().clone())
                .task_pool_worktree_snapshot(
                    &vw_api_task::TaskPoolWorktreeSnapshotRequest {
                        project_id,
                    },
                )
                .await?;
            Ok(Some(worktree_snapshot_from_gateway(snapshot)))
        },
        move |result| {
            Message::TaskBoard(TaskBoardMessage::WorktreeSnapshotLoaded {
                project_path: result_project_path.clone(),
                result,
            })
        },
    )
}

#[allow(dead_code)]
pub(crate) fn apply_execution_timeouts(app: &mut crate::app::App, tick_started_at: web_time::Instant) -> (Vec<Task>, Vec<iced::Task<crate::app::Message>>) {
    let locks = app.task_board_worktree_snapshot.as_ref().map(|s| s.merge_target_locks.clone()).unwrap_or_default();
    let (changed, finished) = desktop_task_board::helpers::execution::apply_execution_timeouts(
        &mut app.task_board_tasks, &app.task_board_settings, &app.task_board_executor.running_tasks,
        &locks, app.project_path.as_deref(), &mut app.task_board_timeout_scan_cursor, tick_started_at);
    for id in finished { app.task_board_executor.finish_task(&id); }
    for task in &changed { sync_viewing_logs(app, &task.id); }
    (changed, Vec::new())
}

#[allow(dead_code)]
pub(crate) fn pick_next_code_review_task(app: &crate::app::App, exclude_ids: &[String]) -> Option<String> {
 desktop_task_board::helpers::execution::pick_next_code_review_task(&app.task_board_tasks, &app.task_board_settings, exclude_ids)
}

pub(crate) fn build_persist_tasks(
    _project_path: &str,
    _tasks: &[Task],
) -> Vec<iced::Task<crate::app::Message>> {
    Vec::new()
}

pub(crate) fn recycle_assigned_task_worktree_for_removal(
    app: &crate::app::App,
    task_id: &str,
) -> iced::Task<crate::app::Message> {
    let Some(project_path) = &app.project_path else {
        return iced::Task::none();
    };

    let path = project_path.clone();
    let result_project_path = path.clone();
    let task_id = task_id.to_string();
    iced::Task::perform(
        recycle_task_worktree_via_gateway(path, task_id.clone(), None),
        move |(task_id, result)| {
            crate::app::Message::TaskBoard(TaskBoardMessage::TaskRemovalWorktreeRecycled {
                project_path: result_project_path.clone(),
                task_id,
                result,
            })
        },
    )
}

pub(crate) fn recycle_assigned_task_worktrees_for_removal(
    app: &crate::app::App,
    task_ids: &[String],
) -> iced::Task<crate::app::Message> {
    let tasks = task_ids
        .iter()
        .map(|task_id| recycle_assigned_task_worktree_for_removal(app, task_id))
        .collect::<Vec<_>>();
    iced::Task::batch(tasks)
}
#[cfg(test)]
#[path = "execution_tests.rs"]
mod execution_tests;

fn worktree_logs_state(app: &mut crate::app::App) -> desktop_task_board::helpers::worktree_logs::WorktreeActionLogs<'_> {
    desktop_task_board::helpers::worktree_logs::WorktreeActionLogs {
        task_board_worktree_manual_action_kind: &mut app.task_board_worktree_manual_action_kind,
        task_board_worktree_manual_confirm_kind: &mut app.task_board_worktree_manual_confirm_kind,
        task_board_worktree_action_logs: &mut app.task_board_worktree_action_logs,
        task_board_worktree_action_logs_visible_until_ms: &mut app.task_board_worktree_action_logs_visible_until_ms,
        task_board_worktree_action_log_rx: &mut app.task_board_worktree_action_log_rx,
    }
}

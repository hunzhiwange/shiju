//! 提供任务看板消息处理过程中复用的局部辅助逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;

pub(crate) use desktop_task_board::helpers::execution::*;
pub(crate) fn flush_running_task_logs_throttled(
    app: &mut crate::app::App,
    force: bool,
    tick_started_at: web_time::Instant,
) {
    let project_path = app.navigation_tabs.project_path.clone();
    let mut logs = desktop_task_board::helpers::logs::TaskLogsState {
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
    };
    desktop_task_board::helpers::execution::flush_running_task_logs_throttled(
        &mut logs,
        &mut app.task_board_executor,
        &mut app.task_board_log_scan_cursor,
        &mut app.task_board_last_log_flush_at_ms,
        force,
        tick_started_at,
        |task| {
            if let Some(path) = project_path.as_deref() {
                let _ = crate::app::task::write_task_plan_files(path, task);
            }
        },
    );
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
    let response = gateway_project::GatewayProjectClient::new(client.transport().clone())
        .project_resolve(
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
            gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
                .agent_scheduler_status(&scope)
                .await
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
) -> Result<Vec<vw_api_agent_scheduler::AgentSchedulerTaskLogsResponse>, String> {
    let client = crate::app::config::gateway_client()?;
    let scope = crate::app::session_gateway::active_agent_scheduler_scope(project_id)?;
    let mut responses = Vec::with_capacity(task_ids.len());
    for task_id in task_ids {
        responses.push(
            gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
                .agent_scheduler_logs(&scope, &task_id)
                .await?,
        );
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
            gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
                .agent_scheduler_status(&scope)
                .await
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
    gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
        .agent_scheduler_create_task(&scope, &request)
        .await?;
    Ok(task)
}

pub(crate) async fn register_pending_agent_scheduler_tasks(
    client: &vw_gateway_client::GatewayClient,
    scope: &vw_api_agent_scheduler::AgentSchedulerScopeDto,
    requests: &[vw_api_agent_scheduler::AgentSchedulerCreateTaskRequest],
) -> Result<(), String> {
    for request in requests {
        gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
            .agent_scheduler_create_task(scope, request)
            .await?;
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
                gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
                    .agent_scheduler_create_task(&scope, &create_request)
                    .await?;
            if !should_send_agent_scheduler_command_after_create(
                command,
                &create_response.task.status,
            ) {
                return gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
                    .agent_scheduler_status(&scope)
                    .await;
            }
            let command_result =
                gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
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
            gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
                .agent_scheduler_status(&scope)
                .await
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
        .task_pool_worktree_snapshot(&vw_api_task::TaskPoolWorktreeSnapshotRequest { project_id })
        .await?;
    let snapshot = worktree_snapshot_from_gateway(snapshot);
    let mut count = 0;
    for slot_id in desktop_task_board::update::worktree::cleanup_slot_ids(&snapshot) {
        let (_, result) = recycle_worktree_slot_via_gateway(project_path.clone(), slot_id).await;
        result?;
        count += 1;
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
            .task_pool_worktree_slot_recycle(&vw_api_task::TaskPoolWorktreeSlotRecycleRequest {
                project_id,
                slot_id: slot_id.clone(),
            })
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
            .task_pool_worktree_recycle(&vw_api_task::TaskPoolWorktreeRecycleRequest {
                project_id,
                task_id: vw_api_transport::TaskId(task_id.clone()),
                taint_reason,
            })
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

    let Some(desired_capacity) = desktop_task_board::update::worktree::begin_maintenance(
        &mut app.task_board_worktree_maintenance_in_flight,
        app.task_board_worktree_manual_action_kind,
        app.task_board_worktree_desired_capacity,
        app.task_board_settings.max_concurrent,
    ) else {
        return iced::Task::none();
    };
    let path = project_path.to_string();
    let result_project_path = path.clone();
    iced::Task::perform(
        async move {
            let project_id = resolve_task_board_project_id(&path).await?;
            let client = crate::app::config::gateway_client()?;
            let response =
                gateway_task_pool::GatewayTaskPoolClient::new(client.transport().clone())
                    .task_pool_worktree_maintain(&vw_api_task::TaskPoolWorktreeMaintenanceRequest {
                        project_id,
                        desired_capacity: desired_capacity as u32,
                    })
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

pub(crate) fn maybe_schedule_worktree_snapshot_refresh(
    app: &mut crate::app::App,
    force: bool,
) -> iced::Task<crate::app::Message> {
    use crate::app::Message;

    let project_path = app.navigation_tabs.project_path.clone();
    let expanded = app.task_board_worktree_panel_expanded;
    if !snapshot_state(app).request(project_path.as_deref(), expanded, force, now_ms()) {
        return iced::Task::none();
    }
    let Some(path) = project_path else {
        return iced::Task::none();
    };
    let result_project_path = path.clone();
    iced::Task::perform(
        async move {
            let project_id = resolve_task_board_project_id(&path).await?;
            let client = crate::app::config::gateway_client()?;
            let snapshot =
                gateway_task_pool::GatewayTaskPoolClient::new(client.transport().clone())
                    .task_pool_worktree_snapshot(&vw_api_task::TaskPoolWorktreeSnapshotRequest {
                        project_id,
                    })
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
    let Some(project_path) = &app.navigation_tabs.project_path else {
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

pub(crate) fn snapshot_state(
    app: &mut crate::app::App,
) -> desktop_task_board::update::worktree::SnapshotState<'_> {
    desktop_task_board::update::worktree::SnapshotState {
        snapshot: &mut app.task_board_worktree_snapshot,
        loading: &mut app.task_board_worktree_snapshot_loading,
        last_refresh: &mut app.task_board_last_worktree_snapshot_at_ms,
    }
}

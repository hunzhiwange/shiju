//! 提供任务看板消息处理过程中复用的局部辅助逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;
use crate::app::task::workflow::{
    DEFAULT_TASK_WORKFLOW_ID, StatusRole, TaskWorkflow, task_workflow_by_id,
};

/// TASK_LOG_UI_MAX_ENTRY_CHARS 使用的固定配置值。
pub(crate) const TASK_LOG_UI_MAX_ENTRY_CHARS: usize = 1200;
/// TASK_LOG_UI_MAX_DETAIL_CHARS 使用的固定配置值。
pub(crate) const TASK_LOG_UI_MAX_DETAIL_CHARS: usize = 2000;
/// TASK_LOG_FLUSH_INTERVAL_MS 使用的固定配置值。
pub(crate) const TASK_LOG_FLUSH_INTERVAL_MS: u64 = 800;
/// TASK_EXECUTION_SCAN_BUDGET_MS 使用的固定配置值。
pub(crate) const TASK_EXECUTION_SCAN_BUDGET_MS: u64 = 8;
/// TASK_RUNNING_LOG_SCAN_BATCH 使用的固定配置值。
pub(crate) const TASK_RUNNING_LOG_SCAN_BATCH: usize = 6;
/// TASK_TIMEOUT_SCAN_BATCH 使用的固定配置值。
#[allow(dead_code)]
pub(crate) const TASK_TIMEOUT_SCAN_BATCH: usize = 32;
#[allow(dead_code)]
/// TASK_BOARD_REFRESH_INTERVAL_SECS 使用的固定配置值。
pub(crate) const TASK_BOARD_REFRESH_INTERVAL_SECS: u64 = 60;
#[allow(dead_code)]
/// TASK_SCHEDULER_TICK_INTERVAL_SECS 使用的固定配置值。
pub(crate) const TASK_SCHEDULER_TICK_INTERVAL_SECS: u64 = 1;
/// TASK_AUTO_CODE_REVIEW_TICK_INTERVAL_SECS 使用的固定配置值。
pub(crate) const TASK_AUTO_CODE_REVIEW_TICK_INTERVAL_SECS: u64 = 30;
/// TASK_AUTO_MERGE_TICK_INTERVAL_SECS 使用的固定配置值。
pub(crate) const TASK_AUTO_MERGE_TICK_INTERVAL_SECS: u64 = 10;
/// Agent Scheduler 命令飞行中状态同步延迟，避免命令卡住时日志面板也停住。
pub(crate) const AGENT_SCHEDULER_COMMAND_IN_FLIGHT_SYNC_DELAY_SECS: u64 = 2;
#[allow(dead_code)]
/// TASK_AUTO_PROMOTE_TICK_INTERVAL_SECS 使用的固定配置值。
pub(crate) const TASK_AUTO_PROMOTE_TICK_INTERVAL_SECS: u64 = 30;
/// TASK_BOARD_WORKTREE_SNAPSHOT_INTERVAL_MS 使用的固定配置值。
pub(crate) const TASK_BOARD_WORKTREE_SNAPSHOT_INTERVAL_MS: u64 = 15_000;
/// TASK_BOARD_WORKTREE_SNAPSHOT_EXPANDED_INTERVAL_MS 使用的固定配置值。
pub(crate) const TASK_BOARD_WORKTREE_SNAPSHOT_EXPANDED_INTERVAL_MS: u64 = 5_000;
/// TASK_BOARD_WORKTREE_ACTION_LOG_TICK_MS 使用的固定配置值。
pub(crate) const TASK_BOARD_WORKTREE_ACTION_LOG_TICK_MS: u64 = 120;
/// TASK_BOARD_WORKTREE_ACTION_LOG_HIDE_DELAY_MS 使用的固定配置值。
pub(crate) const TASK_BOARD_WORKTREE_ACTION_LOG_HIDE_DELAY_MS: u64 = 1200;
/// TASK_BOARD_WORKTREE_ACTION_LOG_MAX_LINES 使用的固定配置值。
pub(crate) const TASK_BOARD_WORKTREE_ACTION_LOG_MAX_LINES: usize = 14;
/// TASK_BOARD_WORKTREE_ACTION_LOG_MAX_ENTRY_CHARS 使用的固定配置值。
pub(crate) const TASK_BOARD_WORKTREE_ACTION_LOG_MAX_ENTRY_CHARS: usize = 320;
#[allow(dead_code)]
const TASK_RUNNING_TIMEOUT_GRACE_LOG_MARKER: &str = "超时保活";

/// 执行 truncate_for_ui 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn truncate_for_ui(value: &str, max_chars: usize) -> String {
    let total = value.chars().count();
    if total <= max_chars {
        return value.to_string();
    }
    let mut truncated = value.chars().take(max_chars).collect::<String>();
    truncated.push_str(&format!(" ...(已截断，共{}字符)", total));
    truncated
}

/// 执行 format_task_log_stream_for_ui 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn format_task_log_stream_for_ui(
    log: &TaskLogStream,
    max_chars: usize,
) -> Option<String> {
    match log {
        TaskLogStream::Stdout(line) | TaskLogStream::Stderr(line) => {
            if line.trim().is_empty() {
                return None;
            }
            Some(truncate_for_ui(line, max_chars))
        }
        TaskLogStream::SubTaskStarted { content, .. } => {
            Some(truncate_for_ui(&format!("子任务开始: {}", content), max_chars))
        }
        TaskLogStream::SubTaskCompleted { subtask_id, .. } => {
            Some(format!("子任务完成: {}", subtask_id))
        }
        TaskLogStream::SubTaskFailed { subtask_id, error, .. } => {
            Some(truncate_for_ui(&format!("子任务失败: {} {}", subtask_id, error), max_chars))
        }
        TaskLogStream::ExitStatus { success, code, signal, .. } => Some(if *success {
            format!("执行成功 code={:?}", code)
        } else {
            match signal {
                Some(signal) => format!("执行失败 code={:?} signal={}", code, signal),
                None => format!("执行失败 code={:?}", code),
            }
        }),
    }
}

/// 执行 push_worktree_action_log 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn push_worktree_action_log(app: &mut crate::app::App, line: String) {
    app.task_board_worktree_action_logs.push(line);
    if app.task_board_worktree_action_logs.len() > TASK_BOARD_WORKTREE_ACTION_LOG_MAX_LINES {
        let overflow =
            app.task_board_worktree_action_logs.len() - TASK_BOARD_WORKTREE_ACTION_LOG_MAX_LINES;
        app.task_board_worktree_action_logs.drain(0..overflow);
    }
}

/// 执行 clear_worktree_action_logs 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn clear_worktree_action_logs(app: &mut crate::app::App) {
    app.task_board_worktree_manual_action_kind = None;
    app.task_board_worktree_manual_confirm_kind = None;
    app.task_board_worktree_action_logs.clear();
    app.task_board_worktree_action_logs_visible_until_ms = None;
    app.task_board_worktree_action_log_rx = None;
}

/// 执行 finish_worktree_action_logs 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn finish_worktree_action_logs(app: &mut crate::app::App) {
    app.task_board_worktree_manual_action_kind = None;
    app.task_board_worktree_manual_confirm_kind = None;
    app.task_board_worktree_action_log_rx = None;
    if app.task_board_worktree_action_logs.is_empty() {
        app.task_board_worktree_action_logs_visible_until_ms = None;
    } else {
        app.task_board_worktree_action_logs_visible_until_ms =
            Some(now_ms().saturating_add(TASK_BOARD_WORKTREE_ACTION_LOG_HIDE_DELAY_MS));
    }
}

/// 执行 poll_worktree_action_logs 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn poll_worktree_action_logs(app: &mut crate::app::App) {
    let Some(receiver) = app.task_board_worktree_action_log_rx.take() else {
        return;
    };

    loop {
        match receiver.try_recv() {
            Ok(log) => {
                if let Some(line) = format_task_log_stream_for_ui(
                    &log,
                    TASK_BOARD_WORKTREE_ACTION_LOG_MAX_ENTRY_CHARS,
                ) {
                    push_worktree_action_log(app, line);
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                app.task_board_worktree_action_log_rx = Some(receiver);
                break;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                app.task_board_worktree_action_log_rx = None;
                break;
            }
        }
    }
}

/// 执行 append_task_log_stream 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn append_task_log_stream(task: &mut Task, log: &TaskLogStream) -> bool {
    let mut plan_changed = false;
    match log {
        TaskLogStream::SubTaskStarted { subtask_id, timestamp_ms, .. } => {
            if let Some(subtask) = task.subtasks.iter_mut().find(|item| item.id == *subtask_id) {
                subtask.start_execution_at(*timestamp_ms);
                plan_changed = true;
            }
        }
        TaskLogStream::SubTaskCompleted { subtask_id, timestamp_ms } => {
            if let Some(subtask) = task.subtasks.iter_mut().find(|item| item.id == *subtask_id) {
                subtask.mark_completed_at(*timestamp_ms);
                plan_changed = true;
            }
        }
        TaskLogStream::SubTaskFailed { subtask_id, timestamp_ms, .. } => {
            if let Some(subtask) = task.subtasks.iter_mut().find(|item| item.id == *subtask_id) {
                subtask.mark_failed_at(*timestamp_ms);
                plan_changed = true;
            }
        }
        _ => {}
    }
    if let Some(line) = format_task_log_stream_for_ui(log, TASK_LOG_UI_MAX_ENTRY_CHARS) {
        task.add_log(line);
    }
    plan_changed
}

/// 执行 flush_running_task_logs 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
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

/// 执行 flush_running_task_logs_throttled 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
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

/// 执行 schedule_worktree_action_log_tick 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn schedule_worktree_action_log_tick() -> iced::Task<crate::app::Message> {
    crate::app::message::after(
        std::time::Duration::from_millis(TASK_BOARD_WORKTREE_ACTION_LOG_TICK_MS),
        crate::app::Message::TaskBoard(TaskBoardMessage::WorktreeActionLogTick),
    )
}

pub(crate) async fn resolve_task_board_project_id(
    project_path: &str,
) -> Result<vw_gateway_client::vw_api_types::id::ProjectId, String> {
    let client = crate::app::config::gateway_client()?;
    let response = client
        .project_resolve(
            &crate::app::session_gateway::active_tenant_scope(),
            &vw_gateway_client::vw_api_types::project::ResolveProjectRequest {
                directory: project_path.to_string(),
                create_if_missing: false,
            },
        )
        .await?;
    Ok(response.project.id)
}

pub(crate) async fn resolve_task_board_scheduler_scope(
    project_path: &str,
) -> Result<vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerScopeDto, String> {
    let project_id = resolve_task_board_project_id(project_path).await?;
    crate::app::session_gateway::active_agent_scheduler_scope(project_id)
}

pub(crate) fn sync_agent_scheduler_status(project_path: String) -> iced::Task<crate::app::Message> {
    let result_project_path = project_path.clone();
    iced::Task::perform(
        async move {
            let scope = resolve_task_board_scheduler_scope(&project_path).await?;
            let client = crate::app::config::gateway_client()?;
            client.agent_scheduler_status(&scope).await
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
    project_id: vw_gateway_client::vw_api_types::id::ProjectId,
    task_ids: Vec<String>,
) -> Result<
    Vec<vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerTaskLogsResponse>,
    String,
> {
    let client = crate::app::config::gateway_client()?;
    let scope = crate::app::session_gateway::active_agent_scheduler_scope(project_id)?;
    let mut responses = Vec::with_capacity(task_ids.len());
    for task_id in task_ids {
        responses.push(client.agent_scheduler_logs(&scope, &task_id).await?);
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
    project_id: vw_gateway_client::vw_api_types::id::ProjectId,
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

fn task_workflow_for_agent_scheduler(workflow_id: &str) -> Option<TaskWorkflow> {
    let workflow_id = workflow_id.trim();
    let workflow_id = if workflow_id.is_empty() { DEFAULT_TASK_WORKFLOW_ID } else { workflow_id };
    task_workflow_by_id(workflow_id).or_else(|| task_workflow_by_id(DEFAULT_TASK_WORKFLOW_ID))
}

fn workflow_status_key(workflow: Option<&TaskWorkflow>, status: TaskStatus) -> String {
    workflow
        .and_then(|workflow| workflow.status_definition(status))
        .map(|definition| definition.key().to_string())
        .unwrap_or_else(|| status.to_string_key().to_string())
}

pub(crate) fn agent_scheduler_status_key_for_task_status(
    status: TaskStatus,
    workflow_id: &str,
) -> String {
    let workflow = task_workflow_for_agent_scheduler(workflow_id);
    workflow_status_key(workflow.as_ref(), status)
}

fn task_pool_workflow_status_role_dto(
    role: StatusRole,
) -> vw_gateway_client::vw_api_types::task::TaskPoolWorkflowStatusRoleDto {
    use vw_gateway_client::vw_api_types::task::TaskPoolWorkflowStatusRoleDto;

    match role {
        StatusRole::Backlog => TaskPoolWorkflowStatusRoleDto::Backlog,
        StatusRole::Queued => TaskPoolWorkflowStatusRoleDto::Queued,
        StatusRole::Planning => TaskPoolWorkflowStatusRoleDto::Planning,
        StatusRole::Executing => TaskPoolWorkflowStatusRoleDto::Executing,
        StatusRole::Failed => TaskPoolWorkflowStatusRoleDto::Failed,
        StatusRole::Paused => TaskPoolWorkflowStatusRoleDto::Paused,
        StatusRole::ReviewReady => TaskPoolWorkflowStatusRoleDto::ReviewReady,
        StatusRole::Review => TaskPoolWorkflowStatusRoleDto::Review,
        StatusRole::Merging => TaskPoolWorkflowStatusRoleDto::Merging,
        StatusRole::Done => TaskPoolWorkflowStatusRoleDto::Done,
        StatusRole::Archived => TaskPoolWorkflowStatusRoleDto::Archived,
        StatusRole::Custom => TaskPoolWorkflowStatusRoleDto::Custom,
    }
}

pub(crate) fn agent_scheduler_status_role_for_task_status(
    status: TaskStatus,
    workflow_id: &str,
) -> Option<vw_gateway_client::vw_api_types::task::TaskPoolWorkflowStatusRoleDto> {
    let workflow = task_workflow_for_agent_scheduler(workflow_id)?;
    workflow.status_role(status).map(task_pool_workflow_status_role_dto)
}

fn status_key_with_role(workflow: &TaskWorkflow, role: StatusRole) -> Option<String> {
    workflow
        .statuses()
        .iter()
        .find(|definition| definition.role() == role)
        .map(|definition| definition.key().to_string())
}

fn status_role_metadata_key(role: StatusRole) -> &'static str {
    match role {
        StatusRole::Backlog => "backlog",
        StatusRole::Queued => "queued",
        StatusRole::Planning => "planning",
        StatusRole::Executing => "executing",
        StatusRole::Failed => "failed",
        StatusRole::Paused => "paused",
        StatusRole::ReviewReady => "review_ready",
        StatusRole::Review => "review",
        StatusRole::Merging => "merging",
        StatusRole::Done => "done",
        StatusRole::Archived => "archived",
        StatusRole::Custom => "custom",
    }
}

fn status_key_for_agent_status(
    workflow: &TaskWorkflow,
    current_status: TaskStatus,
    roles: &[StatusRole],
) -> Option<String> {
    let current_role = workflow.status_role(current_status);
    if current_role.is_some_and(|role| roles.contains(&role)) {
        return Some(workflow_status_key(Some(workflow), current_status));
    }
    roles.iter().find_map(|role| status_key_with_role(workflow, *role))
}

fn insert_agent_status_keys(
    map: &mut serde_json::Map<String, serde_json::Value>,
    agent_statuses: &[&str],
    status_key: Option<String>,
) {
    let Some(status_key) = status_key else {
        return;
    };
    for agent_status in agent_statuses {
        map.insert((*agent_status).to_string(), serde_json::Value::String(status_key.clone()));
    }
}

fn agent_scheduler_status_key_map(
    workflow: Option<&TaskWorkflow>,
    current_status: TaskStatus,
) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    let Some(workflow) = workflow else {
        return map;
    };

    insert_agent_status_keys(
        &mut map,
        &["draft"],
        status_key_for_agent_status(workflow, current_status, &[StatusRole::Backlog]),
    );
    insert_agent_status_keys(
        &mut map,
        &["queued", "active"],
        status_key_for_agent_status(workflow, current_status, &[StatusRole::Queued]),
    );
    insert_agent_status_keys(
        &mut map,
        &["assigned", "dispatched", "acknowledged"],
        status_key_for_agent_status(workflow, current_status, &[StatusRole::Queued]),
    );
    insert_agent_status_keys(
        &mut map,
        &["running"],
        status_key_for_agent_status(workflow, current_status, &[StatusRole::Executing]),
    );
    insert_agent_status_keys(
        &mut map,
        &["submitted"],
        status_key_for_agent_status(
            workflow,
            current_status,
            &[StatusRole::ReviewReady, StatusRole::Review, StatusRole::Merging],
        ),
    );
    insert_agent_status_keys(
        &mut map,
        &["revision"],
        status_key_for_agent_status(workflow, current_status, &[StatusRole::Review]),
    );
    insert_agent_status_keys(
        &mut map,
        &["blocked", "failed"],
        status_key_for_agent_status(workflow, current_status, &[StatusRole::Failed]),
    );
    insert_agent_status_keys(
        &mut map,
        &["paused"],
        status_key_for_agent_status(workflow, current_status, &[StatusRole::Paused]),
    );
    insert_agent_status_keys(
        &mut map,
        &["completed"],
        status_key_for_agent_status(workflow, current_status, &[StatusRole::Done]),
    );
    insert_agent_status_keys(
        &mut map,
        &["cancelled", "archived"],
        status_key_for_agent_status(workflow, current_status, &[StatusRole::Archived]),
    );
    map
}

fn task_status_to_agent_scheduler_status(
    status: TaskStatus,
    workflow: Option<&TaskWorkflow>,
) -> vw_gateway_client::vw_api_types::agent_scheduler::AgentTaskStatusDto {
    use vw_gateway_client::vw_api_types::agent_scheduler::AgentTaskStatusDto;

    if let Some(role) = workflow.and_then(|workflow| workflow.status_role(status)) {
        return match role {
            StatusRole::Backlog => AgentTaskStatusDto::Draft,
            StatusRole::Queued | StatusRole::Planning => AgentTaskStatusDto::Queued,
            StatusRole::Executing => AgentTaskStatusDto::Running,
            StatusRole::Failed => AgentTaskStatusDto::Failed,
            StatusRole::Paused => AgentTaskStatusDto::Paused,
            StatusRole::ReviewReady | StatusRole::Review | StatusRole::Merging => {
                AgentTaskStatusDto::Submitted
            }
            StatusRole::Done => AgentTaskStatusDto::Completed,
            StatusRole::Archived => AgentTaskStatusDto::Archived,
            StatusRole::Custom => AgentTaskStatusDto::Queued,
        };
    }

    match status {
        TaskStatus::Pool => AgentTaskStatusDto::Draft,
        TaskStatus::Pending | TaskStatus::Planning | TaskStatus::Custom(_) => {
            AgentTaskStatusDto::Queued
        }
        TaskStatus::Running => AgentTaskStatusDto::Running,
        TaskStatus::Failed => AgentTaskStatusDto::Failed,
        TaskStatus::Paused => AgentTaskStatusDto::Paused,
        TaskStatus::CodeComplete | TaskStatus::CodeReview | TaskStatus::PrSubmitted => {
            AgentTaskStatusDto::Submitted
        }
        TaskStatus::Completed => AgentTaskStatusDto::Completed,
        TaskStatus::Archived => AgentTaskStatusDto::Archived,
    }
}

pub(crate) fn agent_scheduler_status_for_task_status(
    status: TaskStatus,
    workflow_id: &str,
) -> vw_gateway_client::vw_api_types::agent_scheduler::AgentTaskStatusDto {
    let workflow = task_workflow_for_agent_scheduler(workflow_id);
    task_status_to_agent_scheduler_status(status, workflow.as_ref())
}

fn task_title_for_agent_scheduler(task: &Task) -> String {
    task.prompt
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(task.id.as_str())
        .chars()
        .take(120)
        .collect()
}

fn agent_scheduler_create_task_request(
    task: &Task,
    workflow_id: &str,
) -> vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCreateTaskRequest {
    let task_id = task.id.clone();
    let workflow = task_workflow_for_agent_scheduler(workflow_id);
    let workflow_ref = workflow.as_ref();
    let workflow_metadata_id =
        workflow_ref.map(|workflow| workflow.id().to_string()).unwrap_or_else(|| {
            if workflow_id.trim().is_empty() {
                DEFAULT_TASK_WORKFLOW_ID.to_string()
            } else {
                workflow_id.trim().to_string()
            }
        });
    let status_key = workflow_status_key(workflow_ref, task.status);
    let status_role = workflow_ref
        .and_then(|workflow| workflow.status_role(task.status))
        .map(status_role_metadata_key)
        .unwrap_or("custom");
    let status_key_map = agent_scheduler_status_key_map(workflow_ref, task.status);
    vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCreateTaskRequest {
        task_id: task_id.clone(),
        title: task_title_for_agent_scheduler(task),
        kind: vw_gateway_client::vw_api_types::agent_scheduler::AgentTaskKindDto::Finite,
        status: Some(task_status_to_agent_scheduler_status(task.status, workflow_ref)),
        spec_uri: format!("task-board/{task_id}/spec.md"),
        meta_uri: format!("task-board/{task_id}/meta.json"),
        assigned_to: None,
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        metadata: serde_json::json!({
            "priority": task.priority,
            "order_no": task.order,
            "model": task.model,
            "agent": task.agent,
            "acp_agent": task.acp_agent,
            "reasoning_effort": task.reasoning_effort,
            "prompt": task.prompt,
            "last_error": task.last_error,
            "pause_reason": task.pause_reason,
            "merge_source_branch": task.merge_source_branch,
            "merge_target_branch": task.merge_target_branch,
            "selected_worktree_path": task.selected_worktree_path,
            "selected_worktree_slot_id": task.selected_worktree_slot_id,
            "selected_worktree_branch": task.selected_worktree_branch,
            "task_board.workflow_id": workflow_metadata_id,
            "task_board.status_key": status_key,
            "task_board.status_role": status_role,
            "task_board.status_key_map": status_key_map,
        }),
        created_at_ms: vw_gateway_client::vw_api_types::common::TimestampMs(
            i64::try_from(task.created_at_ms).unwrap_or(i64::MAX),
        ),
    }
}

pub(crate) fn sync_task_board_gateway_snapshot(
    project_path: String,
) -> iced::Task<crate::app::Message> {
    let result_project_path = project_path.clone();
    iced::Task::perform(
        async move {
            let scope = resolve_task_board_scheduler_scope(&project_path).await?;
            let client = crate::app::config::gateway_client()?;
            client.agent_scheduler_status(&scope).await
        },
        move |result| {
            crate::app::Message::TaskBoard(TaskBoardMessage::AgentSchedulerStatusLoaded {
                project_path: result_project_path.clone(),
                result,
            })
        },
    )
}

pub(crate) fn agent_scheduler_wake_create_task_requests(
    tasks: &[Task],
    workflow_id: &str,
) -> Vec<vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCreateTaskRequest> {
    let workflow = task_workflow_for_agent_scheduler(workflow_id);
    tasks
        .iter()
        .filter(|task| {
            workflow.as_ref().and_then(|workflow| workflow.status_role(task.status)).is_some_and(
                |role| {
                    matches!(role, StatusRole::Backlog | StatusRole::Queued | StatusRole::Planning)
                },
            )
        })
        .map(|task| agent_scheduler_create_task_request(task, workflow_id))
        .collect()
}

pub(crate) async fn create_task_via_agent_scheduler(
    project_path: String,
    task: Task,
    workflow_id: String,
) -> Result<Task, String> {
    let request = agent_scheduler_create_task_request(&task, &workflow_id);
    let scope = resolve_task_board_scheduler_scope(&project_path).await?;
    let client = crate::app::config::gateway_client()?;
    client.agent_scheduler_create_task(&scope, &request).await?;
    Ok(task)
}

pub(crate) async fn register_pending_agent_scheduler_tasks(
    client: &vw_gateway_client::GatewayClient,
    scope: &vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerScopeDto,
    requests: &[vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCreateTaskRequest],
) -> Result<(), String> {
    for request in requests {
        client.agent_scheduler_create_task(scope, request).await?;
    }
    Ok(())
}

fn should_send_agent_scheduler_command_after_create(
    command: vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandKindDto,
    status: &vw_gateway_client::vw_api_types::agent_scheduler::AgentTaskStatusDto,
) -> bool {
    use vw_gateway_client::vw_api_types::agent_scheduler::{
        AgentSchedulerCommandKindDto, AgentTaskStatusDto,
    };

    match command {
        AgentSchedulerCommandKindDto::Start | AgentSchedulerCommandKindDto::Promote => !matches!(
            status,
            AgentTaskStatusDto::Assigned
                | AgentTaskStatusDto::Dispatched
                | AgentTaskStatusDto::Acknowledged
                | AgentTaskStatusDto::Running
                | AgentTaskStatusDto::Submitted
        ),
        _ => true,
    }
}

pub(crate) fn send_agent_scheduler_command(
    project_path: String,
    task: Task,
    workflow_id: String,
    command: vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandKindDto,
    dispatch_mode: vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerDispatchModeDto,
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
                client.agent_scheduler_create_task(&scope, &create_request).await?;
            if !should_send_agent_scheduler_command_after_create(
                command,
                &create_response.task.status,
            ) {
                return client.agent_scheduler_status(&scope).await;
            }
            let command_result = client
                .agent_scheduler_command(
                    &scope,
	                    &vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandRequest {
	                        task_id: vw_gateway_client::vw_api_types::id::TaskId(task_id.clone()),
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
                    return client.agent_scheduler_status(&scope).await.map_err(
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
            client.agent_scheduler_status(&scope).await
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
    snapshot: vw_gateway_client::vw_api_types::task::TaskPoolWorktreeSnapshotResponse,
) -> crate::app::task::WorktreePoolSnapshot {
    crate::app::task::WorktreePoolSnapshot::from_gateway_response(snapshot)
}

pub(crate) async fn recycle_worktree_slot_via_gateway(
    project_path: String,
    slot_id: String,
) -> (String, Result<(), String>) {
    let result = async {
        let project_id = resolve_task_board_project_id(&project_path).await?;
        let client = crate::app::config::gateway_client()?;
        let response = client
            .task_pool_worktree_slot_recycle(
                &vw_gateway_client::vw_api_types::task::TaskPoolWorktreeSlotRecycleRequest {
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
        let response = client
            .task_pool_worktree_recycle(
                &vw_gateway_client::vw_api_types::task::TaskPoolWorktreeRecycleRequest {
                    project_id,
                    task_id: vw_gateway_client::vw_api_types::id::TaskId(task_id.clone()),
                    taint_reason,
                },
            )
            .await?;
        if response.recycled { Ok(()) } else { Err("gateway 未回收任务 worktree".to_string()) }
    }
    .await;
    (task_id, result)
}

/// 执行 schedule_worktree_pool_maintenance 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
#[allow(dead_code)]
pub(crate) fn schedule_worktree_pool_maintenance(
    app: &mut crate::app::App,
    project_path: &str,
    _running_tasks: usize,
) -> iced::Task<crate::app::Message> {
    use crate::app::Message;

    if !app.project_is_git_repo {
        app.task_board_worktree_snapshot = None;
        app.task_board_worktree_snapshot_loading = false;
        app.task_board_last_worktree_snapshot_at_ms = 0;
        app.task_board_worktree_maintenance_in_flight = false;
        return iced::Task::none();
    }

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
            let response = client
                .task_pool_worktree_maintain(
                    &vw_gateway_client::vw_api_types::task::TaskPoolWorktreeMaintenanceRequest {
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

/// 执行 worktree_snapshot_refresh_interval_ms 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn worktree_snapshot_refresh_interval_ms(app: &crate::app::App) -> u64 {
    if app.task_board_worktree_panel_expanded {
        TASK_BOARD_WORKTREE_SNAPSHOT_EXPANDED_INTERVAL_MS
    } else {
        TASK_BOARD_WORKTREE_SNAPSHOT_INTERVAL_MS
    }
}

/// 执行 maybe_schedule_worktree_snapshot_refresh 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
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

    if !app.project_is_git_repo {
        app.task_board_worktree_snapshot = None;
        app.task_board_worktree_snapshot_loading = false;
        app.task_board_last_worktree_snapshot_at_ms = 0;
        return iced::Task::none();
    }

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
            let snapshot = client
                .task_pool_worktree_snapshot(
                    &vw_gateway_client::vw_api_types::task::TaskPoolWorktreeSnapshotRequest {
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

/// 执行 minutes_to_ms 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn minutes_to_ms(minutes: u32) -> u64 {
    (minutes.max(1) as u64).saturating_mul(60_000)
}

#[allow(dead_code)]
pub(crate) fn retry_backoff_minutes(base_minutes: u32, retry_count: u32) -> u32 {
    let base = base_minutes.clamp(1, 1440) as u64;
    let shift = retry_count.min(31);
    let factor = 1_u64.checked_shl(shift).unwrap_or(u64::MAX);
    base.saturating_mul(factor).min(1440) as u32
}

#[allow(dead_code)]
pub(crate) fn failed_retry_max_attempts(settings: &TaskBoardSettings) -> u32 {
    settings.failed_retry_max_attempts.clamp(0, 10)
}

#[allow(dead_code)]
fn running_timeout_anchor_ms(task: &Task) -> u64 {
    task.execution_started_at_ms.unwrap_or(task.last_active_at_ms).max(task.last_active_at_ms)
}

#[allow(dead_code)]
fn task_has_current_timeout_grace(task: &Task) -> bool {
    let started_at = task.execution_started_at_ms.unwrap_or(0);
    task.logs.iter().rev().any(|entry| {
        entry.timestamp_ms >= started_at
            && entry.message.contains(TASK_RUNNING_TIMEOUT_GRACE_LOG_MARKER)
    })
}

pub(crate) fn record_task_worktree_recycle_after_failure(
    task: &mut Task,
    reason: impl Into<String>,
) {
    task.add_log(format!("{}，等待网关运行时清理 worktree 槽位", reason.into()));
}

pub(crate) fn is_model_response_timeout_error(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    normalized.contains("模型响应超时")
        || normalized.contains("llm response timed out")
        || normalized.contains("llm response timeout")
        || normalized.contains("model response timed out")
        || normalized.contains("model response timeout")
        || normalized.contains("request timed out while waiting for the model")
        || normalized.contains("gateway stream idle timeout")
        || normalized.contains("gateway 响应超时")
}

#[allow(dead_code)]
fn model_timeout_retry_wait_elapsed(
    task: &Task,
    now: u64,
    model_timeout_retry_wait_ms: u64,
    max_attempts: u32,
) -> bool {
    let Some(error) = task.last_error.as_deref() else {
        return true;
    };
    if !is_model_response_timeout_error(error) {
        return true;
    }
    if task.retry_count >= max_attempts.clamp(0, 10) {
        return false;
    }
    now.saturating_sub(task.last_active_at_ms) >= model_timeout_retry_wait_ms
}

/// 执行 apply_execution_timeouts 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
#[allow(dead_code)]
pub(crate) fn apply_execution_timeouts(
    app: &mut crate::app::App,
    tick_started_at: web_time::Instant,
) -> (Vec<Task>, Vec<iced::Task<crate::app::Message>>) {
    let now = now_ms();
    let running_timeout_ms = minutes_to_ms(app.task_board_settings.running_timeout_minutes);
    let pr_stall_timeout_secs = pr_submitted_stall_timeout_secs(app);
    let pr_stall_timeout_ms = pr_stall_timeout_secs.saturating_mul(1000);
    let mut changed_tasks = Vec::new();
    let recycle_tasks = Vec::new();
    let mut changed_task_ids = Vec::new();
    let mut timed_out_running_ids = Vec::new();
    let mut stalled_merge_ids = Vec::new();

    if app.task_board_tasks.is_empty() {
        app.task_board_timeout_scan_cursor = 0;
        return (changed_tasks, recycle_tasks);
    }
    if app.task_board_timeout_scan_cursor >= app.task_board_tasks.len() {
        app.task_board_timeout_scan_cursor = 0;
    }
    let max_batch = TASK_TIMEOUT_SCAN_BATCH.min(app.task_board_tasks.len());
    let mut scanned = 0usize;
    let merge_target_locks = app
        .task_board_worktree_snapshot
        .as_ref()
        .map(|snapshot| snapshot.merge_target_locks.clone())
        .unwrap_or_default();

    while scanned < max_batch {
        if scanned > 0
            && tick_started_at.elapsed()
                >= std::time::Duration::from_millis(TASK_EXECUTION_SCAN_BUDGET_MS)
        {
            break;
        }
        let idx = (app.task_board_timeout_scan_cursor + scanned) % app.task_board_tasks.len();
        let task = &mut app.task_board_tasks[idx];
        if task.deleted || task.archived {
            scanned += 1;
            continue;
        }
        match task.status {
            TaskStatus::Failed => {}
            TaskStatus::Planning | TaskStatus::Running => {
                let timeout_anchor = running_timeout_anchor_ms(task);
                if now.saturating_sub(timeout_anchor) >= running_timeout_ms {
                    if !task_has_current_timeout_grace(task) {
                        task.execution_started_at_ms = Some(now);
                        task.add_log(format!(
                            "{}: {}超过 {} 分钟未完成，本次不标记失败，继续等待一个周期",
                            TASK_RUNNING_TIMEOUT_GRACE_LOG_MARKER,
                            task.status.label(),
                            app.task_board_settings.running_timeout_minutes.max(1)
                        ));
                    } else {
                        let timeout_reason = format!(
                            "{}连续超过 {} 分钟仍未完成，已自动标记失败",
                            task.status.label(),
                            app.task_board_settings.running_timeout_minutes.max(1)
                        );
                        task.mark_execution_failed(timeout_reason.clone());
                        record_task_worktree_recycle_after_failure(task, "任务执行连续超时");
                        timed_out_running_ids.push(task.id.clone());
                    }
                    changed_task_ids.push(task.id.clone());
                    changed_tasks.push(task.clone());
                }
            }
            TaskStatus::PrSubmitted => {
                if !app.task_board_executor.running_tasks.contains(&task.id) {
                    scanned += 1;
                    continue;
                }
                let inactive_for_ms = now.saturating_sub(task.last_active_at_ms);
                if inactive_for_ms >= pr_stall_timeout_ms {
                    if let Some(project_path) = app.project_path.as_deref() {
                        let holder = task
                            .merge_target_branch
                            .as_deref()
                            .and_then(|target_branch| {
                                merge_target_locks
                                    .iter()
                                    .find(|(target, _)| target == target_branch.trim())
                                    .map(|(_, task_id)| task_id.clone())
                            })
                            .unwrap_or_else(|| "none".to_string());
                        let selected_worktree = task
                            .selected_worktree_path
                            .clone()
                            .unwrap_or_else(|| "none".to_string());
                        let target_branch =
                            task.merge_target_branch.as_deref().unwrap_or("none").to_string();
                        task.add_log(format!(
                            "合并阶段超时检测: inactive_ms={} worktree={} target={} lock_holder={}",
                            inactive_for_ms, selected_worktree, target_branch, holder
                        ));
                        task.add_log(format!(
                            "合并阶段超时暂停: target={} project_path={}，gateway 锁由执行守卫释放",
                            target_branch, project_path
                        ));
                    }
                    let pause_reason =
                        format!("合并阶段静默超过 {} 秒未完成，已自动暂停", pr_stall_timeout_secs);
                    task.mark_paused(pause_reason.clone());
                    record_task_worktree_recycle_after_failure(task, "合并阶段超时暂停");
                    stalled_merge_ids.push(task.id.clone());
                    changed_task_ids.push(task.id.clone());
                    changed_tasks.push(task.clone());
                }
            }
            _ => {}
        }
        scanned += 1;
    }
    if scanned > 0 {
        app.task_board_timeout_scan_cursor =
            (app.task_board_timeout_scan_cursor + scanned) % app.task_board_tasks.len();
    }

    for task_id in timed_out_running_ids {
        app.task_board_executor.finish_task(&task_id);
    }
    for task_id in stalled_merge_ids {
        app.task_board_executor.finish_task(&task_id);
    }
    for task_id in changed_task_ids {
        sync_viewing_logs(app, &task_id);
    }
    if !app.task_board_tasks.is_empty()
        && app.task_board_timeout_scan_cursor >= app.task_board_tasks.len()
    {
        app.task_board_timeout_scan_cursor = 0;
    }

    (changed_tasks, recycle_tasks)
}

/// 执行 pick_next_code_review_task 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
#[allow(dead_code)]
pub(crate) fn pick_next_code_review_task(
    app: &crate::app::App,
    exclude_ids: &[String],
) -> Option<String> {
    let now = now_ms();
    let max_attempts = failed_retry_max_attempts(&app.task_board_settings);

    app.task_board_tasks
        .iter()
        .filter(|task| {
            !task.deleted
                && !task.archived
                && task.status == TaskStatus::CodeReview
                && !exclude_ids.contains(&task.id)
                && model_timeout_retry_wait_elapsed(
                    task,
                    now,
                    minutes_to_ms(retry_backoff_minutes(
                        app.task_board_settings.failed_retry_minutes,
                        task.retry_count,
                    )),
                    max_attempts,
                )
        })
        .min_by_key(|task| (task.priority, task.order))
        .map(|task| task.id.clone())
}

/// 执行 build_persist_tasks 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn build_persist_tasks(
    _project_path: &str,
    _tasks: &[Task],
) -> Vec<iced::Task<crate::app::Message>> {
    Vec::new()
}

/// 执行 recycle_assigned_task_worktree_for_removal 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn recycle_assigned_task_worktree_for_removal(
    app: &crate::app::App,
    task_id: &str,
) -> iced::Task<crate::app::Message> {
    if !app.project_is_git_repo {
        return iced::Task::none();
    }

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

/// 执行 recycle_assigned_task_worktrees_for_removal 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
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

use desktop_task_model::{Task, TaskStatus, TaskBoardSettings};
use desktop_task_model::workflow::{DEFAULT_TASK_WORKFLOW_ID, StatusRole, TaskWorkflow, task_workflow_by_id};
use desktop_task_executor::TaskLogStream;
/// TASK_LOG_UI_MAX_ENTRY_CHARS 使用的固定配置值。
pub const TASK_LOG_UI_MAX_ENTRY_CHARS: usize = 1200;
/// TASK_LOG_UI_MAX_DETAIL_CHARS 使用的固定配置值。
pub const TASK_LOG_UI_MAX_DETAIL_CHARS: usize = 2000;
/// TASK_LOG_FLUSH_INTERVAL_MS 使用的固定配置值。
pub const TASK_LOG_FLUSH_INTERVAL_MS: u64 = 800;
/// TASK_EXECUTION_SCAN_BUDGET_MS 使用的固定配置值。
pub const TASK_EXECUTION_SCAN_BUDGET_MS: u64 = 8;
/// TASK_RUNNING_LOG_SCAN_BATCH 使用的固定配置值。
pub const TASK_RUNNING_LOG_SCAN_BATCH: usize = 6;
/// TASK_TIMEOUT_SCAN_BATCH 使用的固定配置值。
#[allow(dead_code)]
pub const TASK_TIMEOUT_SCAN_BATCH: usize = 32;
#[allow(dead_code)]
/// TASK_BOARD_REFRESH_INTERVAL_SECS 使用的固定配置值。
pub const TASK_BOARD_REFRESH_INTERVAL_SECS: u64 = 60;
#[allow(dead_code)]
/// TASK_SCHEDULER_TICK_INTERVAL_SECS 使用的固定配置值。
pub const TASK_SCHEDULER_TICK_INTERVAL_SECS: u64 = 1;
/// TASK_AUTO_CODE_REVIEW_TICK_INTERVAL_SECS 使用的固定配置值。
pub const TASK_AUTO_CODE_REVIEW_TICK_INTERVAL_SECS: u64 = 30;
/// TASK_AUTO_MERGE_TICK_INTERVAL_SECS 使用的固定配置值。
pub const TASK_AUTO_MERGE_TICK_INTERVAL_SECS: u64 = 10;
/// Agent Scheduler 命令飞行中状态同步延迟，避免命令卡住时日志面板也停住。
pub const AGENT_SCHEDULER_COMMAND_IN_FLIGHT_SYNC_DELAY_SECS: u64 = 2;
#[allow(dead_code)]
/// TASK_AUTO_PROMOTE_TICK_INTERVAL_SECS 使用的固定配置值。
pub const TASK_AUTO_PROMOTE_TICK_INTERVAL_SECS: u64 = 30;
/// TASK_BOARD_WORKTREE_SNAPSHOT_INTERVAL_MS 使用的固定配置值。
pub const TASK_BOARD_WORKTREE_SNAPSHOT_INTERVAL_MS: u64 = 15_000;
/// TASK_BOARD_WORKTREE_SNAPSHOT_EXPANDED_INTERVAL_MS 使用的固定配置值。
pub const TASK_BOARD_WORKTREE_SNAPSHOT_EXPANDED_INTERVAL_MS: u64 = 5_000;
/// TASK_BOARD_WORKTREE_ACTION_LOG_TICK_MS 使用的固定配置值。
pub const TASK_BOARD_WORKTREE_ACTION_LOG_TICK_MS: u64 = 120;
/// TASK_BOARD_WORKTREE_ACTION_LOG_HIDE_DELAY_MS 使用的固定配置值。
pub const TASK_BOARD_WORKTREE_ACTION_LOG_HIDE_DELAY_MS: u64 = 1200;
/// TASK_BOARD_WORKTREE_ACTION_LOG_MAX_LINES 使用的固定配置值。
pub const TASK_BOARD_WORKTREE_ACTION_LOG_MAX_LINES: usize = 14;
/// TASK_BOARD_WORKTREE_ACTION_LOG_MAX_ENTRY_CHARS 使用的固定配置值。
pub const TASK_BOARD_WORKTREE_ACTION_LOG_MAX_ENTRY_CHARS: usize = 320;
#[allow(dead_code)]
pub(super) const TASK_RUNNING_TIMEOUT_GRACE_LOG_MARKER: &str = "超时保活";

pub fn truncate_for_ui(value: &str, max_chars: usize) -> String {
    let total = value.chars().count();
    if total <= max_chars {
        return value.to_string();
    }
    let mut truncated = value.chars().take(max_chars).collect::<String>();
    truncated.push_str(&format!(" ...(已截断，共{}字符)", total));
    truncated
}

pub fn format_task_log_stream_for_ui(
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

pub fn append_task_log_stream(task: &mut Task, log: &TaskLogStream) -> bool {
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

pub fn agent_scheduler_status_key_for_task_status(
    status: TaskStatus,
    workflow_id: &str,
) -> String {
    let workflow = task_workflow_for_agent_scheduler(workflow_id);
    workflow_status_key(workflow.as_ref(), status)
}

fn task_pool_workflow_status_role_dto(
    role: StatusRole,
) -> vw_api_task::TaskPoolWorkflowStatusRoleDto {
    use vw_api_task::TaskPoolWorkflowStatusRoleDto;

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

pub fn agent_scheduler_status_role_for_task_status(
    status: TaskStatus,
    workflow_id: &str,
) -> Option<vw_api_task::TaskPoolWorkflowStatusRoleDto> {
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
) -> vw_api_agent_scheduler::AgentTaskStatusDto {
    use vw_api_agent_scheduler::AgentTaskStatusDto;

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

pub fn agent_scheduler_status_for_task_status(
    status: TaskStatus,
    workflow_id: &str,
) -> vw_api_agent_scheduler::AgentTaskStatusDto {
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

pub fn agent_scheduler_create_task_request(
    task: &Task,
    workflow_id: &str,
) -> vw_api_agent_scheduler::AgentSchedulerCreateTaskRequest {
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
    vw_api_agent_scheduler::AgentSchedulerCreateTaskRequest {
        task_id: task_id.clone(),
        title: task_title_for_agent_scheduler(task),
        kind: vw_api_agent_scheduler::AgentTaskKindDto::Finite,
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
        created_at_ms: vw_api_transport::TimestampMs(
            i64::try_from(task.created_at_ms).unwrap_or(i64::MAX),
        ),
    }
}

pub fn agent_scheduler_wake_create_task_requests(
    tasks: &[Task],
    workflow_id: &str,
) -> Vec<vw_api_agent_scheduler::AgentSchedulerCreateTaskRequest> {
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

pub fn should_send_agent_scheduler_command_after_create(
    command: vw_api_agent_scheduler::AgentSchedulerCommandKindDto,
    status: &vw_api_agent_scheduler::AgentTaskStatusDto,
) -> bool {
    use vw_api_agent_scheduler::{
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

pub fn minutes_to_ms(minutes: u32) -> u64 {
    (minutes.max(1) as u64).saturating_mul(60_000)
}

pub fn retry_backoff_minutes(base_minutes: u32, retry_count: u32) -> u32 {
    let base = base_minutes.clamp(1, 1440) as u64;
    let shift = retry_count.min(31);
    let factor = 1_u64.checked_shl(shift).unwrap_or(u64::MAX);
    base.saturating_mul(factor).min(1440) as u32
}

pub fn failed_retry_max_attempts(settings: &TaskBoardSettings) -> u32 {
    settings.failed_retry_max_attempts.clamp(0, 10)
}

pub(super) fn running_timeout_anchor_ms(task: &Task) -> u64 {
    task.execution_started_at_ms.unwrap_or(task.last_active_at_ms).max(task.last_active_at_ms)
}

pub(super) fn task_has_current_timeout_grace(task: &Task) -> bool {
    let started_at = task.execution_started_at_ms.unwrap_or(0);
    task.logs.iter().rev().any(|entry| {
        entry.timestamp_ms >= started_at
            && entry.message.contains(TASK_RUNNING_TIMEOUT_GRACE_LOG_MARKER)
    })
}

pub fn record_task_worktree_recycle_after_failure(
    task: &mut Task,
    reason: impl Into<String>,
) {
    task.add_localized_log(
        "%s，等待网关运行时清理 worktree 槽位",
        vec![(reason.into()).to_string().into()],
    );
}

pub fn is_model_response_timeout_error(error: &str) -> bool {
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

pub(super) fn model_timeout_retry_wait_elapsed(
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

#[cfg(test)]
#[path = "execution_tests.rs"]
mod execution_tests;

pub use super::timeouts::{apply_execution_timeouts, pick_next_code_review_task};

/// 轮询运行任务日志；计划文件写入通过窄回调交给宿主。
pub fn flush_running_task_logs(
    logs: &mut super::logs::TaskLogsState<'_>,
    executor: &mut desktop_task_executor::executor::state::TaskExecutorState,
    cursor: &mut usize,
    tick_started_at: web_time::Instant,
    mut on_plan_changed: impl FnMut(&Task),
) {
    let running = executor.running_tasks.clone();
    if running.is_empty() {
        (*cursor) = 0;
        return;
    }
    if (*cursor) >= running.len() {
        (*cursor) = 0;
    }
    let max_batch = TASK_RUNNING_LOG_SCAN_BATCH.min(running.len());
    let viewing_task_id = logs.task_board_viewing_logs.as_ref().map(|task| task.id.clone());
    let mut scanned = 0usize;

    while scanned < max_batch {
        if scanned > 0
            && tick_started_at.elapsed()
                >= std::time::Duration::from_millis(TASK_EXECUTION_SCAN_BUDGET_MS)
        {
            break;
        }
        let idx = ((*cursor) + scanned) % running.len();
        let task_id = running[idx].clone();
        let task_logs = executor.poll_task_logs(&task_id);
        if let Some(task) = logs.task_board_tasks.iter_mut().find(|t| t.id == task_id) {
            let mut plan_changed = false;
            for log in &task_logs {
                plan_changed |= append_task_log_stream(task, log);
            }
            if plan_changed { on_plan_changed(task); }
        }
        if viewing_task_id.as_deref() == Some(task_id.as_str()) {
            super::logs::sync_viewing_logs(logs, &task_id);
        }
        scanned += 1;
    }
    if scanned > 0 {
        (*cursor) = ((*cursor) + scanned) % running.len();
    }

}

pub fn log_flush_due(last: u64, force: bool, now: u64) -> bool {
    force || now.saturating_sub(last) >= TASK_LOG_FLUSH_INTERVAL_MS
}

/// 扫描超时并释放对应的本地运行标记。
pub fn apply_execution_timeouts_and_finish(
    tasks: &mut [Task], settings: &TaskBoardSettings,
    executor: &mut desktop_task_executor::executor::state::TaskExecutorState,
    locks: &[(String, String)], project_path: Option<&str>, cursor: &mut usize,
    tick_started_at: web_time::Instant,
) -> (Vec<Task>, Vec<iced::Task<crate::message::TaskBoardMessage>>) {
    let (changed, finished) = apply_execution_timeouts(tasks, settings, &executor.running_tasks, locks, project_path, cursor, tick_started_at);
    for id in finished { executor.finish_task(&id); }
    (changed, Vec::new())
}

pub fn flush_running_task_logs_throttled(
    logs: &mut super::logs::TaskLogsState<'_>,
    executor: &mut desktop_task_executor::executor::state::TaskExecutorState,
    cursor: &mut usize, last_flush: &mut u64, force: bool,
    tick_started_at: web_time::Instant, on_plan_changed: impl FnMut(&Task),
) {
    let now = runtime_time::now_ms();
    if !log_flush_due(*last_flush, force, now) { return; }
    flush_running_task_logs(logs, executor, cursor, tick_started_at, on_plan_changed);
    *last_flush = now;
}

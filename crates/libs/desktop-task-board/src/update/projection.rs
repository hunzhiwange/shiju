//! 调度快照、日志与本地运行状态的任务投影。
use desktop_task_model::{Task, TaskStatus, TaskLogEntry, SubTask, StatusRole};

fn task_status_from_agent_scheduler_key(status_key: &str) -> Option<TaskStatus> {
    match status_key.trim() {
        "draft" => Some(TaskStatus::Pool),
        "queued" | "active" | "assigned" | "dispatched" | "acknowledged" => {
            Some(TaskStatus::Pending)
        }
        "running" => Some(TaskStatus::Running),
        "submitted" | "revision" => Some(TaskStatus::CodeReview),
        "blocked" | "paused" => Some(TaskStatus::Paused),
        "completed" => Some(TaskStatus::Completed),
        "failed" => Some(TaskStatus::Failed),
        "cancelled" | "archived" => Some(TaskStatus::Archived),
        value => TaskStatus::parse_key(value),
    }
}

fn task_status_from_agent_scheduler_status(
    status: vw_api_agent_scheduler::AgentTaskStatusDto,
) -> TaskStatus {
    use vw_api_agent_scheduler::AgentTaskStatusDto;

    match status {
        AgentTaskStatusDto::Draft => TaskStatus::Pool,
        AgentTaskStatusDto::Queued
        | AgentTaskStatusDto::Active
        | AgentTaskStatusDto::Assigned
        | AgentTaskStatusDto::Dispatched
        | AgentTaskStatusDto::Acknowledged => TaskStatus::Pending,
        AgentTaskStatusDto::Running => TaskStatus::Running,
        AgentTaskStatusDto::Submitted | AgentTaskStatusDto::Revision => TaskStatus::CodeReview,
        AgentTaskStatusDto::Blocked | AgentTaskStatusDto::Paused => TaskStatus::Paused,
        AgentTaskStatusDto::Completed => TaskStatus::Completed,
        AgentTaskStatusDto::Failed => TaskStatus::Failed,
        AgentTaskStatusDto::Cancelled | AgentTaskStatusDto::Archived => TaskStatus::Archived,
    }
}

fn task_status_from_agent_scheduler_snapshot_status(
    status: vw_api_agent_scheduler::AgentTaskStatusDto,
) -> TaskStatus {
    use vw_api_agent_scheduler::AgentTaskStatusDto;

    match status {
        AgentTaskStatusDto::Submitted => TaskStatus::CodeComplete,
        other => task_status_from_agent_scheduler_status(other),
    }
}

fn status_key_is_stale_pending_projection(
    status_key_status: TaskStatus,
    agent_status: TaskStatus,
) -> bool {
    status_key_status == TaskStatus::Pending && agent_status != TaskStatus::Pending
}

fn run_stage_is_running(run_stage: Option<&str>) -> bool {
    matches!(run_stage.map(str::trim), Some("running"))
}

fn task_status_from_agent_scheduler_snapshot(
    snapshot: &vw_api_agent_scheduler::AgentSchedulerTaskBoardTaskDto,
) -> TaskStatus {
    let agent_status =
        snapshot.status.clone().map(task_status_from_agent_scheduler_snapshot_status);
    let status_key = snapshot.status_key.trim();
    if !status_key.is_empty()
        && let Some(status) = task_status_from_agent_scheduler_key(status_key)
    {
        if status == TaskStatus::Pending && run_stage_is_running(snapshot.run_stage.as_deref()) {
            return TaskStatus::Running;
        }
        if let Some(agent_status) = agent_status
            && status_key_is_stale_pending_projection(status, agent_status)
        {
            return agent_status;
        }
        return status;
    }
    agent_status.unwrap_or(TaskStatus::Pool)
}

fn task_status_from_agent_scheduler_log(
    status_key: Option<&str>,
    status: Option<vw_api_agent_scheduler::AgentTaskStatusDto>,
) -> Option<TaskStatus> {
    status_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(task_status_from_agent_scheduler_key)
        .or_else(|| status.map(task_status_from_agent_scheduler_status))
}

fn timestamp_ms_to_u64(timestamp_ms: vw_api_transport::TimestampMs) -> u64 {
    u64::try_from(timestamp_ms.0).unwrap_or_default()
}

fn task_log_entry_from_agent_scheduler(
    log: &vw_api_agent_scheduler::AgentSchedulerTaskLogDto,
) -> TaskLogEntry {
    TaskLogEntry {
        localized_message: None,
        timestamp_ms: timestamp_ms_to_u64(log.timestamp_ms),
        status_from: task_status_from_agent_scheduler_log(
            log.from_status_key.as_deref(),
            log.status_from.clone(),
        ),
        status_to: task_status_from_agent_scheduler_log(
            log.to_status_key.as_deref(),
            log.status_to.clone(),
        ),
        message: log.message.clone(),
    }
}

fn task_log_entry_matches(left: &TaskLogEntry, right: &TaskLogEntry) -> bool {
    left.timestamp_ms == right.timestamp_ms
        && left.status_from == right.status_from
        && left.status_to == right.status_to
        && left.message == right.message
}

fn merge_agent_scheduler_task_logs(
    task: &mut Task,
    logs: &[vw_api_agent_scheduler::AgentSchedulerTaskLogDto],
) -> bool {
    let mut changed = false;
    for log in logs {
        let entry = task_log_entry_from_agent_scheduler(log);
        if !task.logs.iter().any(|existing| task_log_entry_matches(existing, &entry)) {
            task.logs.push(entry);
            changed = true;
        }
    }
    changed
}

/// 按批次汇总子任务，生成形如「批次 1: 2 个; 批次 2(依赖 1): 1 个」的简短摘要。
///
/// 仅用于规划完成后的任务日志展示，按批次号升序输出；依赖批次取该批次内任一子任务声明的
/// 依赖批次（同批次依赖批次一致），无依赖时省略括号。
pub(super) fn summarize_subtask_batches(subtasks: &[SubTask]) -> String {
    use std::collections::BTreeMap;

    let mut counts: BTreeMap<u32, usize> = BTreeMap::new();
    let mut depends: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for subtask in subtasks {
        *counts.entry(subtask.batch).or_default() += 1;
        depends.entry(subtask.batch).or_insert_with(|| subtask.depends_on_batches.clone());
    }

    counts
        .into_iter()
        .map(|(batch, count)| {
            let dependency = depends.get(&batch).filter(|values| !values.is_empty());
            match dependency {
                Some(values) => {
                    let joined = values.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
                    format!("批次 {batch}(依赖 {joined}): {count} 个")
                }
                None => format!("批次 {batch}: {count} 个"),
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn runtime_order_no_to_task_order(order_no: i64) -> u32 {
    u32::try_from(order_no.max(0)).unwrap_or(u32::MAX)
}

fn apply_agent_scheduler_task_snapshot(
    task: &mut Task,
    snapshot: &vw_api_agent_scheduler::AgentSchedulerTaskBoardTaskDto,
) {
    let next_status = task_status_from_agent_scheduler_snapshot(snapshot);
    if task.status != next_status {
        task.set_status(next_status);
    }

    task.priority = snapshot.priority;
    task.order = runtime_order_no_to_task_order(snapshot.order_no);
    task.model = snapshot.model.clone();
    task.agent = snapshot.agent.clone();
    task.acp_agent = snapshot.acp_agent.clone();
    task.reasoning_effort = snapshot.reasoning_effort;
    task.prompt = snapshot.prompt.clone();
    task.retry_count = snapshot.retry_count;
    task.last_error = snapshot.last_error.clone();
    task.pause_reason = snapshot.pause_reason.clone();
    task.merge_source_branch = snapshot.merge_source_branch.clone();
    task.merge_target_branch = snapshot.merge_target_branch.clone();

    let workspace = snapshot.workspace.as_ref();
    task.selected_worktree_slot_id = snapshot
        .selected_worktree_slot_id
        .clone()
        .or_else(|| workspace.and_then(|workspace| workspace.slot_id.clone()));
    task.selected_worktree_path = snapshot.selected_worktree_path.clone().or_else(|| {
        workspace
            .filter(|workspace| workspace.slot_id.is_some())
            .map(|workspace| workspace.path.clone())
    });
    task.selected_worktree_branch = snapshot
        .selected_worktree_branch
        .clone()
        .or_else(|| workspace.and_then(|workspace| workspace.branch.clone()));

    let updated_at_ms = timestamp_ms_to_u64(snapshot.updated_at_ms);
    if updated_at_ms > 0 {
        task.updated_at_ms = updated_at_ms;
        task.last_active_at_ms = updated_at_ms;
    }

    if let Some(stage) =
        snapshot.run_stage.as_deref().map(str::trim).filter(|stage| !stage.is_empty())
    {
        let message = format!("Agent Scheduler 阶段: {stage}");
        if !task.logs.iter().rev().take(8).any(|entry| entry.message == message) {
            task.add_log(message);
        }
    }
}

fn task_from_agent_scheduler_snapshot(
    snapshot: &vw_api_agent_scheduler::AgentSchedulerTaskBoardTaskDto,
) -> Task {
    let mut task = Task::default();
    task.id = snapshot.task_id.as_ref().to_string();
    task.created_at_ms = timestamp_ms_to_u64(snapshot.updated_at_ms);
    task.updated_at_ms = task.created_at_ms;
    task.last_active_at_ms = task.created_at_ms;
    task.description = snapshot
        .read_model
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            snapshot
                .prompt
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or(snapshot.task_id.as_ref())
        })
        .to_string();
    task.logs.clear();
    apply_agent_scheduler_task_snapshot(&mut task, snapshot);
    task.archived = task.status == TaskStatus::Archived;
    task
}

pub fn mark_task_paused_after_worktree_preallocation_failure(task: &mut Task, error: String) {
    if let Some(started_at) = task.execution_started_at_ms {
        task.last_execution_duration_ms = Some(crate::helpers::settings::now_ms().saturating_sub(started_at));
    }
    task.execution_started_at_ms = None;
    task.add_localized_log("[WORKTREE] 执行前预分配失败: %s", vec![(error).to_string().into()]);
    task.mark_paused(format!("Git/worktree 预分配失败: {}", error));
}

pub fn merge_loaded_tasks_preserving_running_state(
    loaded_tasks: Vec<Task>,
    current_tasks: &[Task],
    running_task_ids: &[String],
    pending_persistence_task_ids: &std::collections::HashSet<String>,
) -> Vec<Task> {
    let mut merged = loaded_tasks
        .into_iter()
        .map(|loaded_task| {
            if (running_task_ids.iter().any(|id| id == &loaded_task.id)
                || pending_persistence_task_ids.contains(&loaded_task.id))
                && let Some(current_task) =
                    current_tasks.iter().find(|task| task.id == loaded_task.id)
            {
                return current_task.clone();
            }
            loaded_task
        })
        .collect::<Vec<_>>();

    for current_task in current_tasks {
        if (running_task_ids.iter().any(|id| id == &current_task.id)
            || pending_persistence_task_ids.contains(&current_task.id))
            && !merged.iter().any(|task| task.id == current_task.id)
        {
            merged.push(current_task.clone());
        }
    }

    merged
}


/// 合并权威快照，保留尚未确认持久化的本地结果。
/// 返回更新任务和需刷新日志的 ID；执行器清理保持原有时序。
pub fn apply_agent_scheduler_status(
    tasks: &mut Vec<Task>,
    pending_persistence_task_ids: &std::collections::HashSet<String>,
    executor: &mut desktop_task_executor::executor::state::TaskExecutorState,
    response: vw_api_agent_scheduler::AgentSchedulerTaskBoardStatusResponse,
) -> (Vec<Task>, Vec<String>) {
    let mut updated_tasks = Vec::new();
    let mut updated_task_ids = Vec::new();
    let previous_tasks = std::mem::take(tasks);
    let mut next_tasks = Vec::with_capacity(response.tasks.len());

    for snapshot in &response.tasks {
        let task_id = snapshot.task_id.as_ref().to_string();
        if pending_persistence_task_ids.contains(&task_id)
            && let Some(task) = previous_tasks.iter().find(|task| task.id == task_id).cloned()
        {
            updated_task_ids.push(task_id);
            updated_tasks.push(task.clone());
            next_tasks.push(task);
            continue;
        }
        executor.finish_task(&task_id);
        let mut task = previous_tasks
            .iter()
            .find(|task| task.id == task_id)
            .cloned()
            .unwrap_or_else(|| task_from_agent_scheduler_snapshot(snapshot));
        apply_agent_scheduler_task_snapshot(&mut task, snapshot);
        task.archived = task.status == TaskStatus::Archived;
        updated_task_ids.push(task_id);
        updated_tasks.push(task.clone());
        next_tasks.push(task);
    }

    for task in previous_tasks {
        if pending_persistence_task_ids.contains(&task.id)
            && !next_tasks.iter().any(|next_task| next_task.id == task.id)
        {
            updated_task_ids.push(task.id.clone());
            updated_tasks.push(task.clone());
            next_tasks.push(task);
        }
    }

    *tasks = next_tasks;


    (updated_tasks, updated_task_ids)
}

#[cfg(test)]
#[path = "projection_tests.rs"]
mod tests;

pub fn apply_agent_scheduler_logs(tasks: &mut [Task], responses: Vec<vw_api_agent_scheduler::AgentSchedulerTaskLogsResponse>) -> (Vec<Task>, Vec<String>) {
    let mut updated_tasks = Vec::new();
    let mut updated_task_ids = Vec::new();

    for response in responses {
        let task_id = response.task_id.as_ref().to_string();
        if let Some(task) = tasks.iter_mut().find(|task| task.id == task_id)
            && merge_agent_scheduler_task_logs(task, &response.logs)
        {
            updated_task_ids.push(task_id);
            updated_tasks.push(task.clone());
        }
    }

    (updated_tasks, updated_task_ids)
}

pub fn scheduler_followup_ticks(workflow_id: &str, updated_tasks: &[Task]) -> (bool, bool) {
    let workflow = desktop_task_model::task_workflow_by_id(workflow_id);
    let should_schedule_review = updated_tasks.iter().any(|task| {
        workflow
            .as_ref()
            .and_then(|workflow| workflow.status_role(task.status))
            .is_some_and(|role| matches!(role, StatusRole::ReviewReady | StatusRole::Review))
    });
    let should_schedule_merge = updated_tasks.iter().any(|task| {
        workflow
            .as_ref()
            .and_then(|workflow| workflow.status_role(task.status))
            .is_some_and(|role| matches!(role, StatusRole::Merging))
    });
    (should_schedule_review, should_schedule_merge)
}

pub fn task_board_project_matches(current_project_path: Option<&str>, project_path: &str) -> bool {
    current_project_path == Some(project_path)
}

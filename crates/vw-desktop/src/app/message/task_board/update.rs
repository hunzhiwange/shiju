//! 处理任务看板状态更新分支，将 UI 消息转换为应用状态变更和异步任务。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use crate::app::Message;
use crate::app::state::MAIN_AGENT_KEY;
use crate::app::task::SubTaskStatus;
use crate::app::task::models::parse_task_import_content;
use crate::app::task::workflow::{StatusRole, task_workflow_by_id};

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
    app.project_path.as_deref() == Some(project_path)
}

fn initial_task_status_for_app(app: &crate::app::App) -> TaskStatus {
    initial_task_status_for_workflow_id(&app.task_board_settings.workflow_id)
}

fn workflow_for_task_board_settings(
    app: &crate::app::App,
) -> Option<crate::app::task::TaskWorkflow> {
    task_workflow_by_id(&app.task_board_settings.workflow_id)
}

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
    status: vw_gateway_client::vw_api_types::agent_scheduler::AgentTaskStatusDto,
) -> TaskStatus {
    use vw_gateway_client::vw_api_types::agent_scheduler::AgentTaskStatusDto;

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
    status: vw_gateway_client::vw_api_types::agent_scheduler::AgentTaskStatusDto,
) -> TaskStatus {
    use vw_gateway_client::vw_api_types::agent_scheduler::AgentTaskStatusDto;

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
    snapshot: &vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerTaskBoardTaskDto,
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
    status: Option<vw_gateway_client::vw_api_types::agent_scheduler::AgentTaskStatusDto>,
) -> Option<TaskStatus> {
    status_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(task_status_from_agent_scheduler_key)
        .or_else(|| status.map(task_status_from_agent_scheduler_status))
}

fn timestamp_ms_to_u64(timestamp_ms: vw_gateway_client::vw_api_types::common::TimestampMs) -> u64 {
    u64::try_from(timestamp_ms.0).unwrap_or_default()
}

fn task_log_entry_from_agent_scheduler(
    log: &vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerTaskLogDto,
) -> TaskLogEntry {
    TaskLogEntry {
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
    logs: &[vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerTaskLogDto],
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
fn summarize_subtask_batches(subtasks: &[SubTask]) -> String {
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

fn sync_agent_scheduler_status_for_current_project(
    app: &crate::app::App,
) -> iced::Task<crate::app::Message> {
    app.project_path.clone().map(sync_agent_scheduler_status).unwrap_or_else(iced::Task::none)
}

fn agent_scheduler_gateway_unavailable(error: &str) -> bool {
    vw_gateway_client::is_gateway_unavailable_error(error)
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

fn runtime_order_no_to_task_order(order_no: i64) -> u32 {
    u32::try_from(order_no.max(0)).unwrap_or(u32::MAX)
}

fn apply_agent_scheduler_task_snapshot(
    task: &mut Task,
    snapshot: &vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerTaskBoardTaskDto,
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
        .or_else(|| workspace.map(|workspace| workspace.slot_id.clone()));
    task.selected_worktree_path = snapshot
        .selected_worktree_path
        .clone()
        .or_else(|| workspace.map(|workspace| workspace.path.clone()));
    task.selected_worktree_branch = snapshot
        .selected_worktree_branch
        .clone()
        .or_else(|| workspace.map(|workspace| workspace.branch.clone()));

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
    snapshot: &vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerTaskBoardTaskDto,
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

fn apply_agent_scheduler_status(
    app: &mut crate::app::App,
    response: vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerTaskBoardStatusResponse,
) -> Vec<Task> {
    let mut updated_tasks = Vec::new();
    let mut updated_task_ids = Vec::new();
    let previous_tasks = std::mem::take(&mut app.task_board_tasks);
    let mut next_tasks = Vec::with_capacity(response.tasks.len());

    for snapshot in &response.tasks {
        let task_id = snapshot.task_id.as_ref().to_string();
        if app.task_board_tasks_pending_persistence.contains(&task_id)
            && let Some(task) = previous_tasks.iter().find(|task| task.id == task_id).cloned()
        {
            updated_task_ids.push(task_id);
            updated_tasks.push(task.clone());
            next_tasks.push(task);
            continue;
        }
        app.task_board_executor.finish_task(&task_id);
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
        if app.task_board_tasks_pending_persistence.contains(&task.id)
            && !next_tasks.iter().any(|next_task| next_task.id == task.id)
        {
            updated_task_ids.push(task.id.clone());
            updated_tasks.push(task.clone());
            next_tasks.push(task);
        }
    }

    app.task_board_tasks = next_tasks;
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
    responses: Vec<
        vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerTaskLogsResponse,
    >,
) -> Vec<Task> {
    let mut updated_tasks = Vec::new();
    let mut updated_task_ids = Vec::new();

    for response in responses {
        let task_id = response.task_id.as_ref().to_string();
        if let Some(task) = app.task_board_tasks.iter_mut().find(|task| task.id == task_id)
            && merge_agent_scheduler_task_logs(task, &response.logs)
        {
            updated_task_ids.push(task_id);
            updated_tasks.push(task.clone());
        }
    }

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

    let workflow = workflow_for_task_board_settings(app);
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
    let mut tasks = Vec::new();
    if should_schedule_review {
        tasks.push(schedule_auto_review_tick_with_deadline(app));
    }
    if should_schedule_merge {
        tasks.push(schedule_auto_merge_tick_with_deadline(app));
    }
    iced::Task::batch(tasks)
}

fn mark_task_paused_after_worktree_preallocation_failure(task: &mut Task, error: String) {
    if let Some(started_at) = task.execution_started_at_ms {
        task.last_execution_duration_ms = Some(now_ms().saturating_sub(started_at));
    }
    task.execution_started_at_ms = None;
    task.add_log(format!("[WORKTREE] 执行前预分配失败: {}", error));
    task.mark_paused(format!("Git/worktree 预分配失败: {}", error));
}

fn merge_loaded_tasks_preserving_running_state(
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
            let board_is_open = app.screen == crate::app::Screen::TaskBoard;

            if board_is_open {
                app.show_task_board = false;
                app.screen = if app.project_path.is_some() {
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
            app.screen = crate::app::Screen::TaskBoard;
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
            app.screen = if app.project_path.is_some() {
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
        TaskBoardMessage::TaskUpdated(task) => {
            if let Some(existing) = app.task_board_tasks.iter_mut().find(|t| t.id == task.id) {
                *existing = task.clone();
            }
            prune_bulk_selection(app);
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
        TaskBoardMessage::DragStarted { task_id, from_status } => {
            app.task_board_dragging = Some((task_id, from_status));
            app.task_board_drag_pending = None;
            iced::Task::none()
        }
        TaskBoardMessage::DragPending { task_id, from_status, press_position } => {
            app.task_board_drag_pending = Some((task_id, from_status, press_position));
            iced::Task::none()
        }
        TaskBoardMessage::DragEnded => {
            app.task_board_dragging = None;
            app.task_board_drag_pending = None;
            iced::Task::none()
        }
        TaskBoardMessage::CardReleased { task_id } => {
            if app.task_board_dragging.is_some() {
                app.task_board_dragging = None;
                app.task_board_drag_pending = None;
            } else if app.task_board_drag_pending.is_some() {
                app.task_board_drag_pending = None;
                if let Some(task) = app.task_board_tasks.iter().find(|t| t.id == task_id).cloned() {
                    app.task_board_editing_task_id = Some(task_id);
                    app.task_board_detail_tab = TaskBoardDetailTab::Basic;
                    set_viewing_logs(app, Some(task.clone()));
                    app.task_board_draft.priority = task.priority.to_string();
                    app.task_board_draft.model = task.model.clone();
                    if app.task_board_draft.acp_agent.is_none() && !crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(app, &app.task_board_draft.model).contains(&app.task_board_draft.reasoning_effort) {
                app.task_board_draft.reasoning_effort = None;
            }
                    app.task_board_draft.reasoning_effort = task.reasoning_effort;
                    set_draft_executor_selection(&mut app.task_board_draft, task.acp_agent.clone());
                    app.task_board_draft.prompt = task.prompt.clone();
                    app.task_board_draft.subtasks =
                        task.subtasks.iter().map(|subtask| subtask.content.clone()).collect();
                    app.task_board_prompt_editor =
                        iced::widget::text_editor::Content::with_text(&task.prompt);
                }
            }
            iced::Task::none()
        }
        TaskBoardMessage::DropOnStatus { to_status, insert_index } => {
            let Some((task_id, _from_status)) = app.task_board_dragging.take() else {
                return iced::Task::none();
            };
            app.task_board_drag_pending = None;

            let mut status_tasks: Vec<_> = app
                .task_board_tasks
                .iter()
                .filter(|t| t.status == to_status && t.id != task_id)
                .collect();
            status_tasks.sort_by(|a, b| {
                a.order.cmp(&b.order).then_with(|| a.created_at_ms.cmp(&b.created_at_ms))
            });
            let insert_at = insert_index.unwrap_or(status_tasks.len()).min(status_tasks.len());
            let Some(drag_task) = app.task_board_tasks.iter().find(|t| t.id == task_id) else {
                return iced::Task::none();
            };
            status_tasks.insert(insert_at, drag_task);

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
        TaskBoardMessage::SelectTask(task_id) => {
            app.task_board_selected_task = Some(task_id);
            iced::Task::none()
        }
        TaskBoardMessage::DeselectTask => {
            app.task_board_selected_task = None;
            iced::Task::none()
        }
        TaskBoardMessage::FilterByStatus(status) => {
            app.task_board_filter_status = status;
            iced::Task::none()
        }
        TaskBoardMessage::FilterByPriority(range) => {
            app.task_board_filter_priority = range;
            iced::Task::none()
        }
        TaskBoardMessage::SortByPriority(ascending) => {
            app.task_board_sort_by_priority = true;
            app.task_board_sort_ascending = ascending;
            iced::Task::none()
        }
        TaskBoardMessage::SortByDate(ascending) => {
            app.task_board_sort_by_priority = false;
            app.task_board_sort_ascending = ascending;
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
                let task_ids = app
                    .task_board_tasks
                    .iter()
                    .filter(|task| {
                        !task.deleted && !task.archived && task.status == TaskStatus::Completed
                    })
                    .map(|task| task.id.clone())
                    .collect::<Vec<_>>();
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

            let mut copied_task = Task::new(source_task.priority);
            copied_task.assignee = source_task.assignee;
            copied_task.model = source_task.model;
            copied_task.acp_agent = source_task.acp_agent;
            copied_task.reasoning_effort = source_task.reasoning_effort;
            copied_task.description = source_task.description;
            copied_task.prompt = source_task.prompt;
            copied_task.status = initial_task_status_for_app(app);
            copied_task.auto_promote_delay_ms = source_task.auto_promote_delay_ms;
            copied_task.task_split_mode = source_task.task_split_mode;
            copied_task.subtasks = source_task
                .subtasks
                .iter()
                .map(|subtask| SubTask::new(subtask.content.clone()))
                .collect();

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
        TaskBoardMessage::ContextMenuOpened { task_id, x, y } => {
            app.task_board_context_menu = Some((task_id, x, y));
            iced::Task::none()
        }
        TaskBoardMessage::ContextMenuClosed => {
            app.task_board_context_menu = None;
            app.task_board_worktree_manual_confirm_kind = None;
            iced::Task::none()
        }
        TaskBoardMessage::CreateTask => {
            app.task_board_create_modal_open = true;
            app.task_board_create_submit_success = false;
            reset_create_draft(app);
            iced::Task::none()
        }
        TaskBoardMessage::CreateTaskCancelled => {
            app.task_board_create_modal_open = false;
            app.task_board_create_submit_success = false;
            reset_create_draft(app);
            iced::Task::none()
        }
        TaskBoardMessage::UpdateDraftPriority(v) => {
            app.task_board_draft.priority = v;
            iced::Task::none()
        }
        TaskBoardMessage::UpdateDraftAssignee(v) => {
            app.task_board_draft.assignee = v;
            iced::Task::none()
        }
        TaskBoardMessage::UpdateDraftModel(v) => {
            app.task_board_draft.model = v;
            if app.task_board_draft.acp_agent.is_none() && !crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(app, &app.task_board_draft.model).contains(&app.task_board_draft.reasoning_effort) {
                app.task_board_draft.reasoning_effort = None;
            }
            app.task_board_last_model = app.task_board_draft.model.clone();
            iced::Task::none()
        }
        TaskBoardMessage::UpdateDraftAgent(agent) => {
            app.task_board_draft.agent = agent.or_else(|| Some(MAIN_AGENT_KEY.to_string()));
            iced::Task::none()
        }
        TaskBoardMessage::UpdateDraftExecutor(executor) => {
            set_draft_executor_selection(&mut app.task_board_draft, executor.clone());
            app.task_board_last_acp_agent = executor;
            app.task_board_executor_popover = false;
            iced::Task::none()
        }
        TaskBoardMessage::UpdateDraftReasoningEffort(reasoning_effort) => {
            app.task_board_draft.reasoning_effort = reasoning_effort;
            iced::Task::none()
        }
        TaskBoardMessage::UpdateDraftTaskSplitMode(mode) => {
            app.task_board_draft.task_split_mode = mode;
            iced::Task::none()
        }
        TaskBoardMessage::UpdateDraftDescription(v) => {
            app.task_board_draft.description = v;
            iced::Task::none()
        }
        TaskBoardMessage::UpdateDraftPrompt(v) => {
            app.task_board_draft.prompt = v;
            iced::Task::none()
        }
        TaskBoardMessage::UpdateDraftSubtask { index, value } => {
            if let Some(subtask) = app.task_board_draft.subtasks.get_mut(index) {
                *subtask = value;
            }
            iced::Task::none()
        }
        TaskBoardMessage::AddDraftSubtask => {
            app.task_board_draft.subtasks.push(String::new());
            iced::Task::none()
        }
        TaskBoardMessage::RemoveDraftSubtask(index) => {
            if app.task_board_draft.subtasks.len() > 1
                && index < app.task_board_draft.subtasks.len()
            {
                app.task_board_draft.subtasks.remove(index);
            } else if let Some(subtask) = app.task_board_draft.subtasks.get_mut(index) {
                subtask.clear();
            }
            iced::Task::none()
        }
        TaskBoardMessage::MoveDraftSubtaskUp(index) => {
            if index > 0 && index < app.task_board_draft.subtasks.len() {
                app.task_board_draft.subtasks.swap(index, index - 1);
            }
            iced::Task::none()
        }
        TaskBoardMessage::MoveDraftSubtaskDown(index) => {
            if index + 1 < app.task_board_draft.subtasks.len() {
                app.task_board_draft.subtasks.swap(index, index + 1);
            }
            iced::Task::none()
        }
        TaskBoardMessage::UpdateDraftAutoPromoteDelay(v) => {
            app.task_board_draft.auto_promote_delay_seconds = v;
            iced::Task::none()
        }
        TaskBoardMessage::CreateTaskSubmitted => {
            if app.task_board_draft.prompt.trim().is_empty() {
                return iced::Task::none();
            }

            let priority = app.task_board_draft.priority.parse().unwrap_or(999);
            let auto_promote_delay_seconds: u64 =
                app.task_board_draft.auto_promote_delay_seconds.parse().unwrap_or(0);
            let auto_promote_delay_ms = auto_promote_delay_seconds.saturating_mul(1000);
            let model = normalize_task_model(&app.task_board_draft.model);

            let mut task = Task::new(priority);
            task.status = initial_task_status_for_app(app);
            task.model = model;
            task.agent =
                app.task_board_draft.agent.clone().or_else(|| Some(MAIN_AGENT_KEY.to_string()));
            task.acp_agent = app.task_board_draft.acp_agent.clone();
            task.reasoning_effort = app.task_board_draft.reasoning_effort;
            task.task_split_mode = app.task_board_draft.task_split_mode;
            task.prompt = app.task_board_draft.prompt.clone();
            task.subtasks = app
                .task_board_draft
                .subtasks
                .iter()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .map(SubTask::new)
                .collect::<Vec<_>>();
            task.auto_promote_delay_ms =
                if auto_promote_delay_ms > 0 { Some(auto_promote_delay_ms) } else { None };

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
            let raw = content.trim();
            if raw.is_empty() {
                return iced::Task::none();
            }

            let priority = app.task_board_settings.default_priority;
            let mut task = Task::new(priority);
            task.status = initial_task_status_for_app(app);
            task.prompt = build_task_prompt_from_input(raw, Some(priority), &[]);

            {
                let runtime = app.current_session_runtime();
                if !runtime.auto_model && !runtime.model.trim().is_empty() {
                    task.model = runtime.model.trim().to_string();
                }
                set_task_executor_selection(&mut task, None);
            }

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
            let raw = content.trim();
            if raw.is_empty() {
                return iced::Task::none();
            }

            let parsed_priority = priority.trim().parse::<u32>().ok().filter(|p| *p > 0);
            let priority_value =
                parsed_priority.unwrap_or(app.task_board_settings.default_priority);
            let mut task = Task::new(priority_value);
            task.status = initial_task_status_for_app(app);

            let parsed_subtasks = subtasks
                .into_iter()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>();
            task.prompt = build_task_prompt_from_input(raw, Some(priority_value), &parsed_subtasks);
            task.subtasks = parsed_subtasks.into_iter().map(SubTask::new).collect::<Vec<_>>();
            task.reasoning_effort = reasoning_effort;
            task.task_split_mode = task_split_mode;

            {
                let runtime = app.current_session_runtime();
                if runtime.task_mode_enabled {
                    task.model = normalize_task_model(&model);
                } else if !runtime.auto_model && !runtime.model.trim().is_empty() {
                    task.model = runtime.model.trim().to_string();
                }
                set_task_executor_selection(&mut task, runtime.task_mode_executor.clone());
            }

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
        TaskBoardMessage::CloseTaskLogs => {
            set_viewing_logs(app, None);
            app.task_board_detail_tab = TaskBoardDetailTab::Basic;
            app.task_board_logs_auto_scroll = true;
            app.task_board_editing_task_id = None;
            app.task_board_edit_submit_success = false;
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
        TaskBoardMessage::EditTask(task_id) => {
            if let Some(task) = app.task_board_tasks.iter().find(|t| t.id == task_id).cloned() {
                app.task_board_logs_auto_scroll = true;
                app.task_board_editing_task_id = Some(task_id);
                app.task_board_detail_tab = TaskBoardDetailTab::Basic;
                app.task_board_edit_submit_success = false;
                set_viewing_logs(app, Some(task));
                app.task_board_draft.priority =
                    app.task_board_viewing_logs.as_ref().unwrap().priority.to_string();
                app.task_board_draft.model =
                    app.task_board_viewing_logs.as_ref().unwrap().model.clone();
                app.task_board_draft.reasoning_effort =
                    app.task_board_viewing_logs.as_ref().unwrap().reasoning_effort;
                app.task_board_draft.agent = app
                    .task_board_viewing_logs
                    .as_ref()
                    .unwrap()
                    .agent
                    .clone()
                    .or_else(|| Some(MAIN_AGENT_KEY.to_string()));
                set_draft_executor_selection(
                    &mut app.task_board_draft,
                    app.task_board_viewing_logs.as_ref().unwrap().acp_agent.clone(),
                );
                app.task_board_draft.task_split_mode =
                    app.task_board_viewing_logs.as_ref().unwrap().task_split_mode;
                app.task_board_draft.prompt =
                    app.task_board_viewing_logs.as_ref().unwrap().prompt.clone();
                app.task_board_draft.subtasks = app
                    .task_board_viewing_logs
                    .as_ref()
                    .unwrap()
                    .subtasks
                    .iter()
                    .map(|subtask| subtask.content.clone())
                    .collect();
                app.task_board_prompt_editor =
                    iced::widget::text_editor::Content::with_text(&app.task_board_draft.prompt);
            }
            iced::Task::none()
        }
        TaskBoardMessage::UpdateEditingTaskPriority(priority) => {
            app.task_board_draft.priority = priority;
            iced::Task::none()
        }
        TaskBoardMessage::UpdateEditingTaskModel(model) => {
            app.task_board_draft.model = model;
            if app.task_board_draft.acp_agent.is_none() && !crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(app, &app.task_board_draft.model).contains(&app.task_board_draft.reasoning_effort) {
                app.task_board_draft.reasoning_effort = None;
            }
            app.task_board_last_model = normalize_task_model(&app.task_board_draft.model);
            app.task_board_model_popover = false;
            iced::Task::none()
        }
        TaskBoardMessage::UpdateEditingTaskModelInput(model) => {
            app.task_board_draft.model = model;
            if app.task_board_draft.acp_agent.is_none() && !crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(app, &app.task_board_draft.model).contains(&app.task_board_draft.reasoning_effort) {
                app.task_board_draft.reasoning_effort = None;
            }
            app.task_board_last_model = normalize_task_model(&app.task_board_draft.model);
            iced::Task::none()
        }
        TaskBoardMessage::UpdateEditingTaskAgent(agent) => {
            app.task_board_draft.agent = agent.or_else(|| Some(MAIN_AGENT_KEY.to_string()));
            iced::Task::none()
        }
        TaskBoardMessage::UpdateEditingTaskExecutor(executor) => {
            set_draft_executor_selection(&mut app.task_board_draft, executor);
            app.task_board_executor_popover = false;
            iced::Task::none()
        }
        TaskBoardMessage::UpdateEditingTaskReasoningEffort(reasoning_effort) => {
            app.task_board_draft.reasoning_effort = reasoning_effort;
            iced::Task::none()
        }
        TaskBoardMessage::UpdateEditingTaskSplitMode(mode) => {
            app.task_board_draft.task_split_mode = mode;
            iced::Task::none()
        }
        TaskBoardMessage::UpdateEditingTaskPrompt(prompt) => {
            app.task_board_draft.prompt = prompt;
            iced::Task::none()
        }
        TaskBoardMessage::SaveEditingTask => {
            if let Some(task_id) = &app.task_board_editing_task_id.clone()
                && let Some(task) = app.task_board_tasks.iter_mut().find(|t| &t.id == task_id)
            {
                task.priority = app.task_board_draft.priority.parse().unwrap_or(task.priority);
                task.model = normalize_task_model(&app.task_board_draft.model);
                task.agent =
                    app.task_board_draft.agent.clone().or_else(|| Some(MAIN_AGENT_KEY.to_string()));
                task.acp_agent = app.task_board_draft.acp_agent.clone();
                task.reasoning_effort = app.task_board_draft.reasoning_effort;
                task.task_split_mode = app.task_board_draft.task_split_mode;
                task.prompt = app.task_board_draft.prompt.clone();
                task.updated_at_ms = crate::app::time::now_ms();
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
        TaskBoardMessage::ClearCreateSubmitSuccess => {
            app.task_board_create_submit_success = false;
            iced::Task::none()
        }
        TaskBoardMessage::ClearEditSubmitSuccess => {
            app.task_board_edit_submit_success = false;
            iced::Task::none()
        }
        TaskBoardMessage::CloseTaskPanel => {
            app.task_board_create_modal_open = false;
            set_viewing_logs(app, None);
            app.task_board_logs_auto_scroll = true;
            app.task_board_editing_task_id = None;
            app.task_board_create_submit_success = false;
            app.task_board_edit_submit_success = false;
            iced::Task::none()
        }
        TaskBoardMessage::SetAutoPromoteDelay(seconds) => {
            app.task_board_settings.auto_promote_delay_seconds = seconds;
            save_settings(app);
            iced::Task::none()
        }
        TaskBoardMessage::AddSubTask { task_id, content } => {
            if content.trim().is_empty() {
                return iced::Task::none();
            }
            let mut should_sync_viewing_logs = false;
            let mut task_clone = None;
            if let Some(task) = app.task_board_tasks.iter_mut().find(|t| t.id == task_id) {
                let subtask = SubTask::new(content);
                task.subtasks.push(subtask);
                should_sync_viewing_logs = true;
                task.updated_at_ms = crate::app::time::now_ms();
                task_clone = Some(task.clone());
            }
            if should_sync_viewing_logs {
                sync_viewing_logs(app, &task_id);
            }
            if let Some(task_clone) = task_clone
                && let Some(project_path) = &app.project_path
            {
                let _ = project_path;
                return persist_task_metadata_patch_task(app, task_clone, "保存子任务");
            }
            iced::Task::none()
        }
        TaskBoardMessage::RemoveSubTask { task_id, subtask_id } => {
            let mut should_sync_viewing_logs = false;
            let mut task_clone = None;
            if let Some(task) = app.task_board_tasks.iter_mut().find(|t| t.id == task_id) {
                task.subtasks.retain(|s| s.id != subtask_id);
                should_sync_viewing_logs = true;
                task.updated_at_ms = crate::app::time::now_ms();
                task_clone = Some(task.clone());
            }
            if should_sync_viewing_logs {
                sync_viewing_logs(app, &task_id);
            }
            if let Some(task_clone) = task_clone
                && let Some(project_path) = &app.project_path
            {
                let _ = project_path;
                return persist_task_metadata_patch_task(app, task_clone, "保存子任务");
            }
            iced::Task::none()
        }
        TaskBoardMessage::MoveSubTaskUp { task_id, subtask_id } => {
            let mut should_sync_viewing_logs = false;
            let mut task_clone = None;
            if let Some(task) = app.task_board_tasks.iter_mut().find(|t| t.id == task_id) {
                let idx = task.subtasks.iter().position(|s| s.id == subtask_id);
                if let Some(i) = idx
                    && i > 0
                {
                    task.subtasks.swap(i, i - 1);
                    should_sync_viewing_logs = true;
                    task.updated_at_ms = crate::app::time::now_ms();
                    task_clone = Some(task.clone());
                }
            }
            if should_sync_viewing_logs {
                sync_viewing_logs(app, &task_id);
            }
            if let Some(task_clone) = task_clone
                && let Some(project_path) = &app.project_path
            {
                let _ = project_path;
                return persist_task_metadata_patch_task(app, task_clone, "保存子任务");
            }
            iced::Task::none()
        }
        TaskBoardMessage::MoveSubTaskDown { task_id, subtask_id } => {
            let mut should_sync_viewing_logs = false;
            let mut task_clone = None;
            if let Some(task) = app.task_board_tasks.iter_mut().find(|t| t.id == task_id) {
                let idx = task.subtasks.iter().position(|s| s.id == subtask_id);
                if let Some(i) = idx
                    && i < task.subtasks.len() - 1
                {
                    task.subtasks.swap(i, i + 1);
                    should_sync_viewing_logs = true;
                    task.updated_at_ms = crate::app::time::now_ms();
                    task_clone = Some(task.clone());
                }
            }
            if should_sync_viewing_logs {
                sync_viewing_logs(app, &task_id);
            }
            if let Some(task_clone) = task_clone
                && let Some(project_path) = &app.project_path
            {
                let _ = project_path;
                return persist_task_metadata_patch_task(app, task_clone, "保存子任务");
            }
            iced::Task::none()
        }
        TaskBoardMessage::ToggleSubTaskCompleted { task_id, subtask_id } => {
            let mut should_sync_viewing_logs = false;
            let mut task_clone = None;
            if let Some(task) = app.task_board_tasks.iter_mut().find(|t| t.id == task_id)
                && let Some(subtask) = task.subtasks.iter_mut().find(|s| s.id == subtask_id)
            {
                subtask.completed = !subtask.completed;
                if subtask.completed {
                    subtask.status = SubTaskStatus::Completed;
                } else {
                    subtask.status = SubTaskStatus::Pending;
                    subtask.execution_started_at_ms = None;
                }
                should_sync_viewing_logs = true;
                task.updated_at_ms = crate::app::time::now_ms();
                task_clone = Some(task.clone());
            }
            if should_sync_viewing_logs {
                sync_viewing_logs(app, &task_id);
            }
            if let Some(task_clone) = task_clone
                && let Some(project_path) = &app.project_path
            {
                let _ = project_path;
                return persist_task_metadata_patch_task(app, task_clone, "保存子任务");
            }
            iced::Task::none()
        }
        TaskBoardMessage::UpdateSubTaskContent { task_id, subtask_id, content } => {
            let mut should_sync_viewing_logs = false;
            let mut task_clone = None;
            if let Some(task) = app.task_board_tasks.iter_mut().find(|t| t.id == task_id)
                && let Some(subtask) = task.subtasks.iter_mut().find(|s| s.id == subtask_id)
            {
                subtask.content = content;
                should_sync_viewing_logs = true;
                task.updated_at_ms = crate::app::time::now_ms();
                task_clone = Some(task.clone());
            }
            if should_sync_viewing_logs {
                sync_viewing_logs(app, &task_id);
            }
            if let Some(task_clone) = task_clone
                && let Some(project_path) = &app.project_path
            {
                let _ = project_path;
                return persist_task_metadata_patch_task(app, task_clone, "保存子任务");
            }
            iced::Task::none()
        }
        TaskBoardMessage::SubTaskUpdated { task_id, subtasks } => {
            let mut should_sync_viewing_logs = false;
            let mut task_clone = None;
            if let Some(task) = app.task_board_tasks.iter_mut().find(|t| t.id == task_id) {
                task.subtasks = subtasks;
                should_sync_viewing_logs = true;
                task.updated_at_ms = crate::app::time::now_ms();
                task_clone = Some(task.clone());
            }
            if should_sync_viewing_logs {
                sync_viewing_logs(app, &task_id);
            }
            if let Some(task_clone) = task_clone
                && let Some(project_path) = &app.project_path
            {
                let _ = project_path;
                return persist_task_metadata_patch_task(app, task_clone, "保存子任务");
            }
            iced::Task::none()
        }
        TaskBoardMessage::UpdateNewSubtaskContent(content) => {
            app.task_board_new_subtask_content = content;
            iced::Task::none()
        }
        TaskBoardMessage::DescEditorAction(action) => {
            app.task_board_desc_editor.perform(action);
            app.task_board_draft.description = app.task_board_desc_editor.text().to_string();
            iced::Task::none()
        }
        TaskBoardMessage::PromptEditorAction(action) => {
            app.task_board_prompt_editor.perform(action);
            app.task_board_draft.prompt = app.task_board_prompt_editor.text().to_string();
            iced::Task::none()
        }
        TaskBoardMessage::LogsViewerEditorAction(action) => {
            close_logs_context_menu(app);
            if let iced::widget::text_editor::Action::Scroll { lines } = &action {
                apply_task_logs_scroll_lines(app, *lines);
            }
            app.task_board_logs_editor.perform(action);
            iced::Task::none()
        }
        TaskBoardMessage::LogsViewerEditorWheelScrolled { delta, viewport_height } => {
            close_logs_context_menu(app);
            app.task_board_logs_viewport_height = viewport_height.max(0.0);

            let line_height = app.current_line_height.max(1.0);
            let delta_lines = match delta {
                iced::mouse::ScrollDelta::Lines { y, .. } => -y * 1.25,
                iced::mouse::ScrollDelta::Pixels { y, .. } => -y / line_height,
            };

            app.task_board_logs_scroll_remainder += delta_lines;

            let whole_lines = if app.task_board_logs_scroll_remainder >= 0.0 {
                app.task_board_logs_scroll_remainder.floor() as i32
            } else {
                app.task_board_logs_scroll_remainder.ceil() as i32
            };

            if whole_lines != 0 {
                app.task_board_logs_scroll_remainder -= whole_lines as f32;
                apply_task_logs_scroll_lines(app, whole_lines);
                app.task_board_logs_editor
                    .perform(iced::widget::text_editor::Action::Scroll { lines: whole_lines });
            }

            iced::Task::none()
        }
        TaskBoardMessage::LogsViewerScrollbarChanged { top_line, viewport_height } => {
            close_logs_context_menu(app);
            app.task_board_logs_viewport_height = viewport_height.max(0.0);

            let max_scroll = task_logs_max_scroll_top_line(app);
            let target_top_line = top_line.round().clamp(0.0, max_scroll);
            let current_top_line = app.task_board_logs_scroll_top_line.round();
            let delta = (target_top_line - current_top_line) as i32;

            if delta != 0 {
                apply_task_logs_scroll_lines(app, delta);
                app.task_board_logs_editor
                    .perform(iced::widget::text_editor::Action::Scroll { lines: delta });
            }

            iced::Task::none()
        }
        TaskBoardMessage::LogsViewerOpenContextMenu { x, y } => {
            app.task_board_logs_context_menu_open = true;
            app.task_board_logs_context_menu_pos = Some((x, y));
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
        TaskBoardMessage::ToggleModelPopover => {
            let new = !app.task_board_model_popover;
            app.task_board_model_popover = new;
            if new {
                app.task_board_bulk_model_popover = false;
                app.model_popover_hover = None;
                if !app.model_settings.loading && app.model_settings.providers.is_empty() {
                    return iced::Task::done(Message::Settings(
                        crate::app::message::SettingsMessage::ModelsRefresh,
                    ));
                }
            }
            iced::Task::none()
        }
        TaskBoardMessage::ModelSelected(model) => {
            app.task_board_draft.model = model;
            if app.task_board_draft.acp_agent.is_none() && !crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(app, &app.task_board_draft.model).contains(&app.task_board_draft.reasoning_effort) {
                app.task_board_draft.reasoning_effort = None;
            }
            app.task_board_last_model = app.task_board_draft.model.clone();
            app.task_board_model_popover = false;
            app.model_popover_hover = None;
            iced::Task::none()
        }
        TaskBoardMessage::CloseModelPopover => {
            app.task_board_model_popover = false;
            app.model_popover_hover = None;
            iced::Task::none()
        }
        TaskBoardMessage::ToggleExecutorPopover => {
            app.task_board_executor_popover = !app.task_board_executor_popover;
            if app.task_board_executor_popover {
                app.task_board_bulk_executor_popover = false;
            }
            iced::Task::none()
        }
        TaskBoardMessage::ToggleClearPromptAfterCreate(enabled) => {
            app.task_board_clear_prompt_after_create = enabled;
            crate::app::set_config_field(
                "task_board_clear_prompt_after_create",
                serde_json::Value::Bool(enabled),
            );
            iced::Task::none()
        }
        TaskBoardMessage::ToggleCloseAfterCreate(enabled) => {
            app.task_board_close_after_create = enabled;
            crate::app::set_config_field(
                "task_board_close_after_create",
                serde_json::Value::Bool(enabled),
            );
            iced::Task::none()
        }
        TaskBoardMessage::ToggleCloseAfterEdit(enabled) => {
            app.task_board_close_after_edit = enabled;
            crate::app::set_config_field(
                "task_board_close_after_edit",
                serde_json::Value::Bool(enabled),
            );
            iced::Task::none()
        }
        TaskBoardMessage::ToggleTaskSelection { task_id, selected } => {
            if selected {
                app.task_board_selected_tasks.insert(task_id);
            } else {
                app.task_board_selected_tasks.remove(&task_id);
            }
            iced::Task::none()
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
        TaskBoardMessage::SelectAllTasksInStatus(status) => {
            for task_id in visible_task_ids_for_status(app, status) {
                app.task_board_selected_tasks.insert(task_id);
            }
            iced::Task::none()
        }
        TaskBoardMessage::InvertTaskSelectionInStatus(status) => {
            for task_id in visible_task_ids_for_status(app, status) {
                if !app.task_board_selected_tasks.remove(&task_id) {
                    app.task_board_selected_tasks.insert(task_id);
                }
            }
            iced::Task::none()
        }
        TaskBoardMessage::UpdateBulkPriorityInput(value) => {
            app.task_board_bulk_priority_input = value;
            iced::Task::none()
        }
        TaskBoardMessage::UpdateBulkModelInput(value) => {
            app.task_board_bulk_model_input = value;
            if app.task_board_bulk_acp_agent.is_none() && !crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(app, &app.task_board_bulk_model_input).contains(&app.task_board_bulk_reasoning_effort) {
                app.task_board_bulk_reasoning_effort = None;
            }
            iced::Task::none()
        }
        TaskBoardMessage::UpdateBulkTaskSplitMode(mode) => {
            app.task_board_bulk_task_split_mode = mode;
            iced::Task::none()
        }
        TaskBoardMessage::ToggleBulkModelPopover => {
            let new = !app.task_board_bulk_model_popover;
            app.task_board_bulk_model_popover = new;
            if new {
                app.task_board_model_popover = false;
                app.model_popover_hover = None;
                if !app.model_settings.loading && app.model_settings.providers.is_empty() {
                    return iced::Task::done(Message::Settings(
                        crate::app::message::SettingsMessage::ModelsRefresh,
                    ));
                }
            }
            iced::Task::none()
        }
        TaskBoardMessage::BulkModelSelected(model) => {
            app.task_board_bulk_model_input = normalize_task_model(&model);
            if app.task_board_bulk_acp_agent.is_none() && !crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(app, &app.task_board_bulk_model_input).contains(&app.task_board_bulk_reasoning_effort) {
                app.task_board_bulk_reasoning_effort = None;
            }
            app.task_board_last_model = app.task_board_bulk_model_input.clone();
            app.task_board_bulk_model_popover = false;
            app.model_popover_hover = None;
            iced::Task::none()
        }
        TaskBoardMessage::BulkAgentSelected(agent) => {
            app.task_board_bulk_agent =
                if agent.trim().is_empty() { MAIN_AGENT_KEY.to_string() } else { agent };
            iced::Task::none()
        }
        TaskBoardMessage::UpdateBulkReasoningEffort(reasoning_effort) => {
            app.task_board_bulk_reasoning_effort = reasoning_effort;
            iced::Task::none()
        }
        TaskBoardMessage::CloseBulkModelPopover => {
            app.task_board_bulk_model_popover = false;
            app.model_popover_hover = None;
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

            for task in &mut tasks {
                let previous_priority = task.priority;
                task.priority = priority;
                task.add_log(format!("批量设置优先级: {} -> {}", previous_priority, priority));
            }

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
            if app.task_board_bulk_acp_agent.is_none() && !crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(app, &app.task_board_bulk_model_input).contains(&app.task_board_bulk_reasoning_effort) {
                app.task_board_bulk_reasoning_effort = None;
            }
            app.task_board_last_model = model.clone();

            for task in &mut tasks {
                let previous_model = task.model.clone();
                task.model = model.clone();
                task.add_log(format!("批量设置模型: {} -> {}", previous_model, task.model));
            }

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

            let agent = if app.task_board_bulk_agent.trim().is_empty() {
                MAIN_AGENT_KEY.to_string()
            } else {
                app.task_board_bulk_agent.trim().to_string()
            };
            app.task_board_bulk_agent = agent.clone();

            for task in &mut tasks {
                let previous_agent =
                    task.agent.clone().unwrap_or_else(|| MAIN_AGENT_KEY.to_string());
                task.agent = Some(agent.clone());
                task.add_log(format!("批量设置代理: {} -> {}", previous_agent, agent));
            }

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
        TaskBoardMessage::ToggleBulkExecutorPopover => {
            app.task_board_bulk_executor_popover = !app.task_board_bulk_executor_popover;
            if app.task_board_bulk_executor_popover {
                app.task_board_executor_popover = false;
            }
            iced::Task::none()
        }
        TaskBoardMessage::BulkExecutorSelected(executor) => {
            app.task_board_bulk_acp_agent = executor.clone();
            app.task_board_last_acp_agent = executor;
            app.task_board_bulk_executor_popover = false;
            iced::Task::none()
        }
        TaskBoardMessage::CloseBulkExecutorPopover => {
            app.task_board_bulk_executor_popover = false;
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

            for task in &mut tasks {
                let previous_executor = task_acp_agent_label(task.acp_agent.as_deref());
                set_task_executor_selection(task, executor.clone());
                task.add_log(format!(
                    "批量设置 ACP 智能体: {} -> {}",
                    previous_executor,
                    task_acp_agent_label(task.acp_agent.as_deref())
                ));
            }

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
            let mut tasks = Vec::new();
            for task_id in &task_ids {
                if let Some(task) = app.task_board_tasks.iter_mut().find(|task| &task.id == task_id)
                {
                    task.reasoning_effort = reasoning_effort;
                    task.add_log("批量设置推理强度".to_string());
                    tasks.push(task.clone());
                }
            }

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

            let mut tasks = Vec::new();
            for task_id in &task_ids {
                if let Some(task) = app.task_board_tasks.iter_mut().find(|task| &task.id == task_id)
                {
                    let previous_mode = task.task_split_mode;
                    task.task_split_mode = task_split_mode;
                    task.add_log(format!(
                        "批量设置任务拆分: {} -> {}",
                        previous_mode.label(),
                        task_split_mode.label()
                    ));
                    tasks.push(task.clone());
                }
            }

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
            tasks.sort_by_key(|task| (task.priority, task.order));
            let task_ids = tasks.iter().map(|task| task.id.clone()).collect::<Vec<_>>();
            for task in &mut tasks {
                task.add_log("手动批量合并已入队".to_string());
            }
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
        TaskBoardMessage::OpenSettingsModal => {
            app.task_board_settings_modal_open = true;
            app.task_board_settings_modal_tab = TaskBoardSettingsModalTab::default();
            iced::Task::none()
        }
        TaskBoardMessage::CloseSettingsModal => {
            app.task_board_settings_modal_open = false;
            iced::Task::none()
        }
        TaskBoardMessage::SelectSettingsModalTab(tab) => {
            app.task_board_settings_modal_tab = tab;
            iced::Task::none()
        }
        TaskBoardMessage::SetMaxConcurrent(count) => {
            app.task_board_settings.max_concurrent = count.clamp(1, 10);
            app.task_board_worktree_desired_capacity = None;
            save_settings(app);
            iced::Task::none()
        }
        TaskBoardMessage::ToggleAutoRefresh(enabled) => {
            app.task_board_settings.auto_refresh = enabled;
            save_settings(app);
            iced::Task::none()
        }
        TaskBoardMessage::SetRefreshIntervalSeconds(seconds) => {
            app.task_board_settings.refresh_interval_seconds = seconds.clamp(1, 3600);
            app.task_board_next_refresh_at_ms =
                next_deadline_ms(task_board_refresh_interval_secs(app));
            save_settings(app);
            iced::Task::none()
        }
        TaskBoardMessage::SetSchedulerTickIntervalSeconds(seconds) => {
            app.task_board_settings.scheduler_tick_interval_seconds = seconds.clamp(1, 60);
            app.task_board_next_scheduler_tick_at_ms =
                next_deadline_ms(scheduler_tick_interval_secs(app));
            save_settings(app);
            iced::Task::none()
        }
        TaskBoardMessage::SetAutoPromoteTickIntervalSeconds(seconds) => {
            app.task_board_settings.auto_promote_tick_interval_seconds = seconds.clamp(1, 3600);
            app.task_board_next_auto_promote_tick_at_ms =
                next_deadline_ms(auto_promote_tick_interval_secs(app));
            save_settings(app);
            iced::Task::none()
        }
        TaskBoardMessage::SetFailedRetryMinutes(minutes) => {
            app.task_board_settings.failed_retry_minutes = minutes.clamp(1, 1440);
            save_settings(app);
            iced::Task::none()
        }
        TaskBoardMessage::SetFailedRetryMaxAttempts(attempts) => {
            app.task_board_settings.failed_retry_max_attempts = attempts.clamp(0, 10);
            save_settings(app);
            iced::Task::none()
        }
        TaskBoardMessage::SetRunningTimeoutMinutes(minutes) => {
            app.task_board_settings.running_timeout_minutes = minutes.clamp(1, 1440);
            save_settings(app);
            iced::Task::none()
        }
        TaskBoardMessage::ToggleRecycleWorktreeOnTaskFinish(enabled) => {
            app.task_board_settings.recycle_worktree_on_task_finish = enabled;
            save_settings(app);
            iced::Task::none()
        }
        TaskBoardMessage::SetPrSubmittedStallTimeoutSeconds(seconds) => {
            app.task_board_settings.pr_submitted_stall_timeout_seconds = seconds.clamp(5, 3600);
            save_settings(app);
            iced::Task::none()
        }
        TaskBoardMessage::SetAgentSchedulerDispatchMode(dispatch_mode) => {
            app.task_board_settings.agent_scheduler_dispatch_mode = dispatch_mode;
            save_settings(app);
            iced::Task::none()
        }
        TaskBoardMessage::SettingsUpdated(mut settings) => {
            settings = settings.sanitized();
            app.task_board_settings = settings;
            app.task_board_worktree_desired_capacity = None;
            app.task_board_next_refresh_at_ms =
                next_deadline_ms(task_board_refresh_interval_secs(app));
            app.task_board_next_scheduler_tick_at_ms =
                next_deadline_ms(scheduler_tick_interval_secs(app));
            app.task_board_next_auto_promote_tick_at_ms =
                next_deadline_ms(auto_promote_tick_interval_secs(app));
            save_settings(app);
            iced::Task::none()
        }
        TaskBoardMessage::ToggleWorktreePixelOffice(enabled) => {
            app.task_board_worktree_pixel_office = enabled;
            crate::app::set_config_field(
                "task_board_worktree_pixel_office",
                serde_json::Value::Bool(enabled),
            );
            iced::Task::none()
        }
        TaskBoardMessage::CleanAllWorktreesPressed => {
            let Some(project_path) = app.project_path.clone() else {
                return iced::Task::none();
            };
            if app.task_board_worktree_manual_action_kind.is_some() {
                return app.show_warning_toast("已有 worktree 操作进行中");
            }
            if app.task_board_worktree_manual_confirm_kind != Some("cleanup") {
                app.task_board_worktree_manual_confirm_kind = Some("cleanup");
                app.push_notification(
                    "一键清理风险较高，请再次点击“确认一键清理”继续，或点击“取消”返回".to_string(),
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
                    crate::app::task::reset_all_managed_worktrees_async_with_logs(
                        project_path,
                        true,
                        Some(log_tx),
                    ),
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
                Ok(count) => app.push_notification(format!(
                    "已完成 worktree 一键清理，共处理 {} 个槽位",
                    count
                )),
                Err(error) => {
                    app.push_notification(format!(
                        "worktree 一键清理有失败，请查看右下角通知：{}",
                        truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS)
                    ));
                    app.notifications_expanded = true;
                }
            }
            finish_worktree_action_logs(app);
            maybe_schedule_worktree_snapshot_refresh(app, true)
        }
        TaskBoardMessage::DeleteAllWorktreesPressed => {
            let Some(project_path) = app.project_path.clone() else {
                return iced::Task::none();
            };
            if app.task_board_worktree_manual_action_kind.is_some() {
                return app.show_warning_toast("已有 worktree 操作进行中");
            }
            if app.task_board_worktree_manual_confirm_kind != Some("delete") {
                app.task_board_worktree_manual_confirm_kind = Some("delete");
                app.push_notification(
                    "删除所有 worktree 风险极高，请再次点击“确认删除所有 worktree”继续，或点击“取消”返回".to_string(),
                );
                app.notifications_expanded = true;
                return app.show_warning_toast("再次点击确认按钮才会删除 worktree");
            }
            let (log_tx, log_rx) = std::sync::mpsc::channel();
            app.task_board_worktree_manual_confirm_kind = None;
            app.task_board_worktree_manual_action_kind = Some("delete");
            app.task_board_worktree_action_logs.clear();
            app.task_board_worktree_action_logs_visible_until_ms = None;
            app.task_board_worktree_action_log_rx = Some(log_rx);
            push_worktree_action_log(app, "准备开始删除所有 worktree...".to_string());
            iced::Task::batch(vec![
                iced::Task::perform(
                    crate::app::task::delete_all_managed_worktrees_async_with_logs(
                        project_path,
                        Some(log_tx),
                    ),
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
                    app.push_notification("没有找到可删除的受管 worktree".to_string());
                    app.show_warning_toast("没有找到可删除的受管 worktree")
                }
                Ok(count) => {
                    app.push_notification(format!("已删除所有 worktree，共处理 {} 个槽位", count));
                    app.show_success_toast(format!("已删除 {} 个 worktree", count))
                }
                Err(error) => {
                    app.push_notification(format!(
                        "删除所有 worktree 有失败，请查看右下角通知：{}",
                        truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS)
                    ));
                    app.notifications_expanded = true;
                    app.show_error_toast("删除所有 worktree 失败")
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
                return app.show_warning_toast("已有 worktree 操作进行中");
            }
            if let Some(snapshot) = &app.task_board_worktree_snapshot
                && let Some(slot) = snapshot.slots.iter().find(|slot| slot.id == slot_id)
                && matches!(
                    slot.state,
                    crate::app::task::WorktreeState::Busy
                        | crate::app::task::WorktreeState::Recycling
                )
            {
                return app.show_warning_toast("该 worktree 正在使用中，暂不能回收");
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
                    app.push_notification(format!("worktree 槽位已回收: {}", slot_id));
                    app.show_success_toast(format!("已回收 worktree {}", slot_id))
                }
                Err(error) => {
                    app.push_notification(format!(
                        "worktree 槽位回收失败 slot={}：{}",
                        slot_id,
                        truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS)
                    ));
                    app.notifications_expanded = true;
                    app.show_error_toast("worktree 槽位回收失败")
                }
            };
            finish_worktree_action_logs(app);
            iced::Task::batch(vec![toast_task, maybe_schedule_worktree_snapshot_refresh(app, true)])
        }
        TaskBoardMessage::CommitMergeAllWorktreesPressed => {
            let Some(project_path) = app.project_path.clone() else {
                return iced::Task::none();
            };
            let tasks = app.task_board_tasks.clone();
            if app.task_board_worktree_manual_action_kind.is_some() {
                return app.show_warning_toast("已有 worktree 操作进行中");
            }
            if app.task_board_worktree_manual_confirm_kind != Some("merge") {
                app.task_board_worktree_manual_confirm_kind = Some("merge");
                app.push_notification(
                    "一键合并风险较高，请再次点击“确认一键合并”继续，或点击“取消”返回".to_string(),
                );
                app.notifications_expanded = true;
                return iced::Task::none();
            }
            let (log_tx, log_rx) = std::sync::mpsc::channel();
            app.task_board_worktree_manual_confirm_kind = None;
            app.task_board_worktree_manual_action_kind = Some("merge");
            app.task_board_worktree_action_logs.clear();
            app.task_board_worktree_action_logs_visible_until_ms = None;
            app.task_board_worktree_action_log_rx = Some(log_rx);
            push_worktree_action_log(app, "准备开始一键合并...".to_string());
            iced::Task::batch(vec![
                iced::Task::perform(
                    crate::app::task::commit_merge_all_worktrees_async_with_logs(
                        project_path,
                        tasks,
                        Some(log_tx),
                    ),
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
                Ok(count) => app.push_notification(format!(
                    "已完成 worktree 一键合并，共合并 {} 个分支",
                    count
                )),
                Err(error) => {
                    app.push_notification(format!(
                        "worktree 一键合并有失败，请查看右下角通知：{}",
                        truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS)
                    ));
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
            app.push_notification(format!("已取消 {} 确认，可重新发起操作", action_label));
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
        TaskBoardMessage::ColumnScrollChanged { status, has_vertical_scrollbar } => {
            app.task_board_column_has_vertical_scrollbar.insert(status, has_vertical_scrollbar);
            iced::Task::none()
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
                        let mut changed = false;
                        for task in &mut app.task_board_tasks {
                            if task.status == TaskStatus::Pending
                                || task.status == TaskStatus::Planning
                                || task.status == TaskStatus::Running
                                || task.status == TaskStatus::CodeReview
                                || task.status == TaskStatus::PrSubmitted
                            {
                                task.add_log(format!(
                                    "后台 worktree 池维护失败: {}",
                                    truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS)
                                ));
                                changed = true;
                            }
                        }
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
                    if vw_gateway_client::is_gateway_unavailable_error(&error) {
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
                app.push_notification(format!(
                    "任务 {} worktree 回收失败: {}",
                    task_id,
                    truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS)
                ));
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
                    return app.show_warning_toast(format!(
                        "同步 Agent Scheduler 状态失败: {}",
                        truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS)
                    ));
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
                    app.show_error_toast(format!(
                        "保存任务结果失败: {}",
                        truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS)
                    )),
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
                for log in final_logs {
                    let _ = append_task_log_stream(task, &log);
                }
                if task.status != TaskStatus::Planning {
                    task.add_log("任务状态已变化，忽略本次拆分结果".to_string());
                } else {
                    match result {
                        Ok(outcome) => {
                            task.subtasks.clear();
                            for (offset, plan_subtask) in outcome.subtasks.into_iter().enumerate() {
                                let mut subtask = SubTask::new(plan_subtask.title);
                                subtask.order = offset as u32;
                                subtask.boundary = plan_subtask.boundary;
                                subtask.acceptance_criteria = plan_subtask.acceptance_criteria;
                                subtask.target_files = plan_subtask.target_files;
                                subtask.batch = plan_subtask.batch;
                                subtask.depends_on_batches = plan_subtask.depends_on_batches;
                                task.subtasks.push(subtask);
                            }
                            task.add_log(format!("任务拆分完成: {} 个子任务", task.subtasks.len()));
                            task.add_log(format!(
                                "create-task-plans 计划目录: {}",
                                truncate_for_ui(&outcome.plan_dir, TASK_LOG_UI_MAX_ENTRY_CHARS)
                            ));
                            task.add_log(format!(
                                "prompt.md: {}",
                                truncate_for_ui(
                                    &outcome.prompt_md_path,
                                    TASK_LOG_UI_MAX_ENTRY_CHARS
                                )
                            ));
                            task.add_log(format!(
                                "批次摘要: {}",
                                truncate_for_ui(
                                    &summarize_subtask_batches(&task.subtasks),
                                    TASK_LOG_UI_MAX_DETAIL_CHARS
                                )
                            ));
                            if !outcome.raw_output.trim().is_empty() {
                                task.add_log(format!(
                                    "拆分结果: {}",
                                    truncate_for_ui(
                                        &outcome.raw_output,
                                        TASK_LOG_UI_MAX_DETAIL_CHARS
                                    )
                                ));
                            }
                            let _ = crate::app::task::write_task_plan_files(&project_path, task);

                            match crate::app::task::assign_task_execution_worktree(
                                &project_path,
                                task,
                                None,
                            ) {
                                Ok(assigned_worktree_path) => {
                                    let _ = task.start_execution("开始执行任务".to_string());
                                    task.selected_worktree_path = assigned_worktree_path;
                                    if let Some(selected_path) = &task.selected_worktree_path {
                                        task.add_log(format!(
                                            "[WORKTREE] 执行前已分配工作区: {}",
                                            selected_path
                                        ));
                                    }
                                    task.add_log("子任务将按顺序串行执行".to_string());
                                    let _ = crate::app::task::write_task_plan_files(
                                        &project_path,
                                        task,
                                    );
                                    execute_task_to_start = Some(task.clone());
                                    task_to_persist = Some(task.clone());
                                }
                                Err(error) => {
                                    mark_task_paused_after_worktree_preallocation_failure(
                                        task, error,
                                    );
                                    plan_failed = true;
                                    task_to_persist = Some(task.clone());
                                }
                            }
                        }
                        Err(error) => {
                            if is_model_response_timeout_error(&error) {
                                task.mark_execution_failed(format!(
                                    "任务拆分模型响应超时: {}",
                                    error
                                ));
                                record_task_worktree_recycle_after_failure(
                                    task,
                                    "任务拆分模型响应超时",
                                );
                            } else {
                                task.mark_execution_failed(format!(
                                    "任务拆分失败（create-task-plans 计划拆分产物缺失或无效，如 prompt.md 缺失、解析失败或批次校验失败）: {}",
                                    error
                                ));
                            }
                            plan_failed = true;
                            task_to_persist = Some(task.clone());
                        }
                    }
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
        TaskBoardMessage::ToggleImportMode(enabled) => {
            app.task_board_is_import_mode = enabled;
            iced::Task::none()
        }
        TaskBoardMessage::SetImportPromptFormat(format) => {
            app.task_board_import_prompt_format = format;
            iced::Task::none()
        }
        TaskBoardMessage::ToggleImportPromptCollapsed => {
            app.task_board_import_prompt_collapsed = !app.task_board_import_prompt_collapsed;
            iced::Task::none()
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
            app.push_notification("已复制导入提示词模板".to_string());
            iced::clipboard::write(template).map(|_: ()| Message::None)
        }
        TaskBoardMessage::ImportEditorAction(action) => {
            app.task_board_import_editor.perform(action);
            iced::Task::none()
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
                app.push_notification("当前平台暂不支持文件选择，请粘贴内容导入".to_string());
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
                    app.push_notification("已将文件内容填入导入表单".to_string());
                }
                Err(err) => {
                    app.push_notification(err);
                }
            }
            iced::Task::none()
        }
        TaskBoardMessage::InsertDemoData(template) => {
            if let Some(content) = import_demo_content(&template) {
                app.task_board_import_editor =
                    iced::widget::text_editor::Content::with_text(content);
            }
            iced::Task::none()
        }
        TaskBoardMessage::ClearImportEditor => {
            app.task_board_import_editor = iced::widget::text_editor::Content::new();
            iced::Task::none()
        }
        TaskBoardMessage::ImportTasksSubmitted => {
            let content = app.task_board_import_editor.text().to_string();
            if content.trim().is_empty() {
                return iced::Task::none();
            }

            let mut tasks_to_create = Vec::new();
            let default_priority = app.task_board_settings.default_priority;
            let imported = match parse_task_import_content(&content) {
                Ok(imported) => imported,
                Err(error) => {
                    app.push_notification(error);
                    return iced::Task::none();
                }
            };
            for record in imported {
                let mut task = Task::new(record.priority.unwrap_or(default_priority));
                task.status = initial_task_status_for_app(app);
                task.prompt = record.prompt;
                task.model = record.model.unwrap_or_else(|| app.task_board_draft.model.clone());
                task.agent = record
                    .agent
                    .or_else(|| app.task_board_draft.agent.clone())
                    .or_else(|| Some(MAIN_AGENT_KEY.to_string()));
                set_task_executor_selection(
                    &mut task,
                    record
                        .acp_agent
                        .as_deref()
                        .and_then(normalize_task_acp_agent_input)
                        .or_else(|| app.task_board_draft.acp_agent.clone()),
                );
                task.reasoning_effort =
                    record.reasoning_effort.or(app.task_board_draft.reasoning_effort);
                task.task_split_mode =
                    record.task_split_mode.unwrap_or(app.task_board_draft.task_split_mode);
                tasks_to_create.push(task);
            }

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

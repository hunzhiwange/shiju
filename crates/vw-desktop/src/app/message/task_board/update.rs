//! 处理任务看板状态更新分支，将 UI 消息转换为应用状态变更和异步任务。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use crate::app::Message;
use crate::app::components::system_settings::SystemTab;
use crate::app::state::{MAIN_AGENT_KEY, TaskBoardViewMode};
use crate::app::task::SubTaskStatus;
use crate::app::task::models::parse_task_import_content;
use crate::app::task::staff::models::{
    TaskBoardStageAction, default_stage_actions_for_workflow,
    default_stage_assignments_for_workflow,
};
use crate::app::task::staff::stage_capabilities::{
    SAVED_WORKFLOW_CAPABILITY_ID, stage_capability_registry,
};
use crate::app::task::staff::{
    TaskBoardStaffCapability, TaskBoardStaffCapabilityDraftField, TaskBoardStaffCapabilitySummary,
    TaskBoardStaffCapabilitySummaryKind, TaskBoardStaffConfig, TaskBoardStaffLevel,
    TaskBoardStaffMember, TaskBoardStaffOrchestrationContext, TaskBoardStageAssignment,
    TaskBoardStageWorkflowBinding, task_board_staff_capabilities_from_gateway_response,
    task_board_staff_capability_catalog_request,
};
use crate::app::task::workflow::{
    DEFAULT_TASK_WORKFLOW_ID, StatusOutcome, StatusRole, TaskStateMachine, TaskTransitionIntent,
    TaskWorkflow, task_workflow_by_id, task_workflow_snapshot_for_gateway,
};

use super::helpers::*;
use super::*;
use serde_json::Value;
use std::collections::BTreeMap;

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
#[path = "update/staff_messages.rs"]
mod staff_messages_update;

const TASK_BOARD_MATRIX_PROCESSED_EVENT_LIMIT: usize = 2048;
const TASK_BOARD_MATRIX_CHANNEL_TYPE: &str = "matrix";
const TASK_BOARD_MATRIX_PLACEHOLDER_ROOM_ID: &str = "!room-id:localhost";

#[cfg(test)]
static TEST_GATEWAY_ENV_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

fn task_board_project_matches(app: &crate::app::App, project_path: &str) -> bool {
    app.project_path.as_deref() == Some(project_path)
}

fn initial_task_status_for_app(app: &crate::app::App) -> TaskStatus {
    initial_task_status_for_workflow_id(&app.task_board_settings.workflow_id)
}

fn task_pool_status_for_gateway(
    status: TaskStatus,
) -> vw_gateway_client::vw_api_types::task::TaskPoolStatus {
    use vw_gateway_client::vw_api_types::task::TaskPoolStatus;

    match status {
        TaskStatus::Pool => TaskPoolStatus::Pool,
        TaskStatus::Pending => TaskPoolStatus::Pending,
        TaskStatus::Planning => TaskPoolStatus::Planning,
        TaskStatus::Running => TaskPoolStatus::Running,
        TaskStatus::Failed => TaskPoolStatus::Failed,
        TaskStatus::Paused => TaskPoolStatus::Paused,
        TaskStatus::CodeComplete => TaskPoolStatus::CodeComplete,
        TaskStatus::CodeReview => TaskPoolStatus::CodeReview,
        TaskStatus::PrSubmitted => TaskPoolStatus::PrSubmitted,
        TaskStatus::Completed => TaskPoolStatus::Completed,
        TaskStatus::Archived => TaskPoolStatus::Archived,
        // 自定义状态无对应网关协议变体（api-types 协议形态本任务不改），
        // 暂回退到 Pool 占位，待工作流迁移任务接管自定义状态的协议映射。
        TaskStatus::Custom(_) => TaskPoolStatus::Pool,
    }
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

fn staff_matrix_binding_instance_id(member: &TaskBoardStaffMember) -> Option<&str> {
    member
        .channel_bindings
        .iter()
        .find(|binding| binding.channel_type.trim() == TASK_BOARD_MATRIX_CHANNEL_TYPE)
        .and_then(|binding| {
            let instance_id = binding.instance_id.trim();
            (!instance_id.is_empty()).then_some(instance_id)
        })
}

fn task_pool_matrix_actor_id(
    task_id: &str,
    matrix_instance_id: Option<&str>,
    matrix_config: &vw_config_types::channels::MatrixConfig,
) -> Option<String> {
    let skip_reason = if matrix_config.homeserver.trim().is_empty() {
        Some("homeserver is empty")
    } else if matrix_config.access_token.trim().is_empty() {
        Some("access_token is empty")
    } else if matrix_config.room_id.trim().is_empty() {
        Some("room_id is empty")
    } else if matrix_config.room_id.trim() == TASK_BOARD_MATRIX_PLACEHOLDER_ROOM_ID {
        Some("room_id is a placeholder")
    } else {
        None
    };

    if let Some(reason) = skip_reason {
        tracing::warn!(
            target: "vw_desktop",
            task_id = %task_id,
            matrix_instance_id = matrix_instance_id.unwrap_or("default"),
            reason = reason,
            "task board Matrix event skipped: Matrix publish target is incomplete"
        );
        return None;
    }

    let Some(actor_id) =
        matrix_config.user_id.as_deref().map(str::trim).filter(|value| !value.is_empty())
    else {
        tracing::warn!(
            target: "vw_desktop",
            task_id = %task_id,
            matrix_instance_id = matrix_instance_id.unwrap_or("default"),
            "task board Matrix event skipped: Matrix publish target has no user_id"
        );
        return None;
    };

    Some(actor_id.to_string())
}

fn task_pool_matrix_publish_target(
    app: &crate::app::App,
    task: &Task,
) -> Option<(Option<String>, String)> {
    let staff_instance_id = find_task_execution_staff_member(&app.task_board_staff_config, task)
        .and_then(staff_matrix_binding_instance_id);

    if let Some(instance_id) = staff_instance_id {
        let matrix_config =
            if instance_id == vw_config_types::channels::chat::DEFAULT_CHANNEL_INSTANCE_ID {
                app.channels_settings.matrix.as_ref()
            } else {
                app.channels_settings
                    .matrix_instances
                    .iter()
                    .find(|instance| instance.id == instance_id)
                    .map(|instance| &instance.config)
            };

        let Some(matrix_config) = matrix_config else {
            tracing::warn!(
                target: "vw_desktop",
                task_id = %task.id,
                matrix_instance_id = %instance_id,
                "task board Matrix event skipped: staff Matrix instance is not configured"
            );
            return None;
        };

        let actor_id = task_pool_matrix_actor_id(&task.id, Some(instance_id), matrix_config)?;

        return Some((Some(instance_id.to_string()), actor_id));
    }

    let matrix_config = app.channels_settings.matrix.as_ref()?;
    let actor_id = task_pool_matrix_actor_id(&task.id, None, matrix_config)?;

    Some((None, actor_id))
}

fn build_task_pool_matrix_publish_request(
    app: &crate::app::App,
    task: &Task,
    from_status: Option<TaskStatus>,
) -> Option<vw_gateway_client::vw_api_types::task::TaskPoolMatrixPublishRequest> {
    use vw_gateway_client::vw_api_types::task::{
        TaskPoolMatrixActorDto, TaskPoolMatrixEventDto, TaskPoolMatrixEventKindDto,
        TaskPoolMatrixPublishRequest,
    };

    if from_status == Some(task.status) {
        return None;
    }

    let (matrix_instance_id, actor_id) = task_pool_matrix_publish_target(app, task)?;

    let event = TaskPoolMatrixEventDto {
        version: 1,
        event_id: format!(
            "desktop:{}:{}:{}",
            task.id,
            task.status.to_string_key(),
            task.updated_at_ms
        ),
        task_id: task.id.clone(),
        actor: TaskPoolMatrixActorDto { matrix_user_id: actor_id, display_name: None },
        occurred_at_ms: task.updated_at_ms,
        kind: TaskPoolMatrixEventKindDto::TaskStatusChanged {
            from_status: from_status.map(task_pool_status_for_gateway),
            to_status: task_pool_status_for_gateway(task.status),
        },
    };

    if let Err(error) = event.validate_for_matrix_room_event() {
        tracing::warn!(
            target: "vw_desktop",
            task_id = %task.id,
            error = %error,
            "invalid task board Matrix event skipped"
        );
        return None;
    }

    Some(TaskPoolMatrixPublishRequest { event, matrix_instance_id })
}

fn publish_task_pool_matrix_status_event(
    app: &crate::app::App,
    task: &Task,
    from_status: Option<TaskStatus>,
) -> iced::Task<crate::app::Message> {
    let Some(request) = build_task_pool_matrix_publish_request(app, task, from_status) else {
        return iced::Task::none();
    };

    let task_id = task.id.clone();
    iced::Task::perform(
        async move {
            let client = crate::app::config::gateway_client()?;
            let response = client.task_pool_matrix_publish(&request).await?;
            if response.accepted {
                Ok(())
            } else {
                Err("gateway rejected task pool Matrix event".to_string())
            }
        },
        move |result| {
            Message::TaskBoard(TaskBoardMessage::TaskPoolMatrixEventPublished {
                task_id: task_id.clone(),
                result,
            })
        },
    )
}

fn poll_task_pool_matrix_events(app: &crate::app::App) -> iced::Task<crate::app::Message> {
    if app.channels_settings.matrix.is_none() || app.project_path.is_none() {
        return iced::Task::none();
    }

    let after_cursor = app.task_board_matrix_event_cursor;
    iced::Task::perform(
        async move {
            let client = crate::app::config::gateway_client()?;
            client.task_pool_matrix_events(Some(after_cursor)).await
        },
        |result| Message::TaskBoard(TaskBoardMessage::MatrixTaskPoolEventsLoaded(result)),
    )
}

fn open_matrix_task_pool_config(app: &mut crate::app::App) -> iced::Task<crate::app::Message> {
    app.channels_settings.expanded_panels.insert("matrix".to_string());
    app.channels_settings.save_error = None;
    app.show_system_settings = true;
    app.system_settings_tab = SystemTab::Channels;
    app.system_settings_help_tab = None;
    iced::Task::none()
}

fn task_board_matrix_event_is_processed(app: &crate::app::App, event_id: &str) -> bool {
    app.task_board_processed_matrix_event_ids.iter().any(|processed_id| processed_id == event_id)
}

fn remember_task_board_matrix_event(app: &mut crate::app::App, event_id: &str) {
    if event_id.is_empty() || task_board_matrix_event_is_processed(app, event_id) {
        return;
    }

    app.task_board_processed_matrix_event_ids.push_back(event_id.to_string());
    while app.task_board_processed_matrix_event_ids.len() > TASK_BOARD_MATRIX_PROCESSED_EVENT_LIMIT
    {
        app.task_board_processed_matrix_event_ids.pop_front();
    }
}

fn accept_matrix_task_pool_event(
    app: &mut crate::app::App,
    event: &vw_gateway_client::vw_api_types::task::TaskPoolMatrixEventDto,
) -> bool {
    if task_board_matrix_event_is_processed(app, &event.event_id) {
        return false;
    }

    if let Err(error) = event.validate_for_matrix_room_event() {
        tracing::warn!(
            target: "vw_desktop",
            event_id = %event.event_id,
            task_id = %event.task_id,
            error = %error,
            "invalid Matrix task pool event skipped"
        );
        remember_task_board_matrix_event(app, &event.event_id);
        return false;
    }

    remember_task_board_matrix_event(app, &event.event_id);
    true
}

fn apply_matrix_task_pool_events(
    app: &mut crate::app::App,
    response: vw_gateway_client::vw_api_types::task::TaskPoolMatrixEventsResponse,
) -> bool {
    app.task_board_matrix_event_cursor = response.cursor;
    let mut accepted = false;
    for event in &response.events {
        accepted |= accept_matrix_task_pool_event(app, event);
    }
    accepted
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

    app.task_board_tasks = next_tasks;
    app.task_board_loading = false;

    if !updated_tasks.is_empty() {
        refresh_staff_projection(app);
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

fn refresh_staff_projection(app: &mut crate::app::App) {
    if let Err(error) = sync_staff_config_for_current_workflow(app) {
        app.task_board_staff_config_load_error = Some(error);
        app.task_board_staff_projection = Default::default();
        return;
    }
    let workflow_id = current_staff_workflow_id(app);
    match crate::app::task::staff::build_staff_mode_projection(
        &app.task_board_tasks,
        &app.task_board_staff_config,
        &app.task_board_executor.running_tasks,
        &workflow_id,
    ) {
        Ok(projection) => {
            app.task_board_staff_projection = projection;
            app.task_board_staff_config_load_error = None;
        }
        Err(error) => {
            app.task_board_staff_config_load_error = Some(error);
            app.task_board_staff_projection = Default::default();
        }
    }
}

fn current_staff_workflow_id(app: &crate::app::App) -> String {
    let workflow_id = app.task_board_settings.workflow_id.trim();
    if workflow_id.is_empty() {
        DEFAULT_TASK_WORKFLOW_ID.to_string()
    } else {
        workflow_id.to_string()
    }
}

fn apply_staff_config_loaded(
    app: &mut crate::app::App,
    workflow_id: &str,
    result: Result<TaskBoardStaffConfig, String>,
) {
    match result {
        Ok(config) => match config.retarget_for_workflow_id(workflow_id) {
            Ok(config) => {
                app.task_board_staff_config = config;
                app.task_board_staff_config_workflow_id = Some(workflow_id.to_string());
                app.task_board_staff_config_load_error = None;
                if workflow_id == current_staff_workflow_id(app) {
                    refresh_staff_projection(app);
                }
            }
            Err(error) => {
                app.task_board_staff_config_workflow_id = Some(workflow_id.to_string());
                app.task_board_staff_config_load_error = Some(error);
            }
        },
        Err(error) => {
            app.task_board_staff_config_workflow_id = Some(workflow_id.to_string());
            app.task_board_staff_config_load_error = Some(error);
        }
    }
}

fn sync_staff_config_for_current_workflow(app: &mut crate::app::App) -> Result<(), String> {
    let workflow_id = current_staff_workflow_id(app);
    match app.task_board_staff_config.clone().retarget_for_workflow_id(&workflow_id) {
        Ok(config) => {
            app.task_board_staff_config = config;
            app.task_board_staff_config_workflow_id = Some(workflow_id);
        }
        Err(error) => {
            app.task_board_staff_config_workflow_id = Some(workflow_id);
            return Err(error);
        }
    }
    app.task_board_staff_config_load_error = None;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StaffExecutionDefaults {
    member_id: String,
    member_name: String,
    delegate_agent: Option<String>,
    acp_agent: Option<String>,
    cleared_default_delegate_agent: bool,
}

fn normalize_staff_binding(value: Option<&str>) -> Option<String> {
    value.map(str::trim).filter(|value| !value.is_empty()).map(ToString::to_string)
}

fn task_agent_uses_default(agent: Option<&str>) -> bool {
    agent.map(str::trim).filter(|value| !value.is_empty()).is_none_or(|value| {
        value.eq_ignore_ascii_case(MAIN_AGENT_KEY)
            || value.eq_ignore_ascii_case(crate::app::task::TASK_AGENT_MAIN)
    })
}

fn task_acp_agent_uses_default(acp_agent: Option<&str>) -> bool {
    acp_agent.and_then(crate::app::task::normalize_task_acp_agent_input).is_none()
}

fn member_staff_acp_agent(member: &TaskBoardStaffMember) -> Option<String> {
    member.acp_agent.as_deref().and_then(crate::app::task::normalize_task_acp_agent_input).or_else(
        || member.executor_backend.and_then(crate::app::task::legacy_executor_to_task_acp_agent),
    )
}

fn find_task_execution_staff_member<'a>(
    config: &'a TaskBoardStaffConfig,
    task: &Task,
) -> Option<&'a TaskBoardStaffMember> {
    config
        .members
        .iter()
        .find(|member| {
            member.enabled && member.current_task_ids.iter().any(|task_id| task_id == &task.id)
        })
        .or_else(|| {
            let assignee = task.assignee.trim();
            if assignee.is_empty() {
                return None;
            }
            config
                .members
                .iter()
                .find(|member| member.enabled && (member.id == assignee || member.name == assignee))
        })
}

fn apply_staff_execution_defaults(
    config: &TaskBoardStaffConfig,
    task: &mut Task,
) -> Option<StaffExecutionDefaults> {
    let member = find_task_execution_staff_member(config, task)?;
    let mut applied_delegate_agent = None;
    let mut applied_acp_agent = None;
    let mut cleared_default_delegate_agent = false;

    if task_agent_uses_default(task.agent.as_deref())
        && let Some(delegate_agent) = normalize_staff_binding(member.delegate_agent.as_deref())
        && task.agent.as_deref().map(str::trim) != Some(delegate_agent.as_str())
    {
        task.agent = Some(delegate_agent.clone());
        applied_delegate_agent = Some(delegate_agent);
    }

    if task_acp_agent_uses_default(task.acp_agent.as_deref())
        && let Some(acp_agent) = member_staff_acp_agent(member)
    {
        task.acp_agent = Some(acp_agent.clone());
        applied_acp_agent = Some(acp_agent);
    }

    if applied_delegate_agent.is_none()
        && !task_acp_agent_uses_default(task.acp_agent.as_deref())
        && task_agent_uses_default(task.agent.as_deref())
        && task.agent.as_deref().map(str::trim).is_some_and(|value| !value.is_empty())
    {
        task.agent = None;
        cleared_default_delegate_agent = true;
    }

    if applied_delegate_agent.is_none()
        && applied_acp_agent.is_none()
        && !cleared_default_delegate_agent
    {
        return None;
    }

    Some(StaffExecutionDefaults {
        member_id: member.id.clone(),
        member_name: member.name.clone(),
        delegate_agent: applied_delegate_agent,
        acp_agent: applied_acp_agent,
        cleared_default_delegate_agent,
    })
}

fn staff_execution_defaults_log(defaults: &StaffExecutionDefaults) -> String {
    let delegate =
        defaults.delegate_agent.as_deref().unwrap_or(if defaults.cleared_default_delegate_agent {
            "禁用默认Main"
        } else {
            "保持任务配置"
        });
    let acp = defaults.acp_agent.as_deref().unwrap_or("保持任务配置");
    format!(
        "员工执行参数兜底: member={}({}) agent={} acp_agent={}",
        defaults.member_name, defaults.member_id, delegate, acp
    )
}

fn load_gateway_staff_projection_task(
    app: &crate::app::App,
    workflow_id: String,
) -> iced::Task<crate::app::Message> {
    let Some(project_path) = app.project_path.clone() else {
        return iced::Task::done(Message::TaskBoard(
            TaskBoardMessage::StaffConfigLoadedForWorkflow {
                workflow_id,
                result: Err("项目路径缺失，无法加载员工配置".to_string()),
            },
        ));
    };
    let requested_workflow_id = workflow_id.clone();
    iced::Task::perform(load_gateway_staff_projection(project_path, workflow_id), move |result| {
        Message::TaskBoard(TaskBoardMessage::StaffConfigLoadedForWorkflow {
            workflow_id: requested_workflow_id.clone(),
            result,
        })
    })
}

fn load_staff_capability_catalog_task(
    app: &crate::app::App,
    workflow_id: String,
) -> iced::Task<crate::app::Message> {
    let Some(project_path) = app.project_path.clone() else {
        return iced::Task::done(Message::TaskBoard(
            TaskBoardMessage::StaffCapabilityCatalogLoadedForWorkflow {
                workflow_id,
                result: Err("项目路径缺失，无法加载员工能力目录".to_string()),
            },
        ));
    };
    let requested_workflow_id = workflow_id.clone();
    iced::Task::perform(load_staff_capability_catalog(project_path, workflow_id), move |result| {
        Message::TaskBoard(TaskBoardMessage::StaffCapabilityCatalogLoadedForWorkflow {
            workflow_id: requested_workflow_id.clone(),
            result,
        })
    })
}

async fn load_staff_capability_catalog(
    project_path: String,
    workflow_id: String,
) -> Result<Vec<TaskBoardStaffCapability>, String> {
    let client = crate::app::config::gateway_client()?;
    let scope = resolve_task_board_scheduler_scope(&project_path).await?;
    client
        .staff_workflow_capabilities(&scope, &workflow_id)
        .await
        .map(task_board_staff_capabilities_from_gateway_response)
}

fn load_staff_stage_workflow_options_task() -> iced::Task<crate::app::Message> {
    iced::Task::perform(load_staff_stage_workflow_options(), |result| {
        Message::TaskBoard(TaskBoardMessage::StaffStageWorkflowOptionsLoaded(result))
    })
}

#[cfg(not(target_arch = "wasm32"))]
async fn load_staff_stage_workflow_options()
-> Result<Vec<vw_gateway_client::WorkflowRecordSummary>, String> {
    let client = crate::app::config::gateway_client()?;
    client.workflow_applications_list().await
}

#[cfg(target_arch = "wasm32")]
async fn load_staff_stage_workflow_options()
-> Result<Vec<vw_gateway_client::WorkflowRecordSummary>, String> {
    Err("Web 平台暂不支持读取本地 Workflow 应用列表".to_string())
}

#[derive(Debug, Clone)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
struct GeneratedStageWorkflowSpec {
    name: String,
    description: String,
    workflow_label: String,
    workflow_id: String,
    status_label: String,
    status_key: String,
    capability_label: String,
    capability_id: String,
    input_description: String,
    result_description: String,
    input_fields: Vec<String>,
    result_fields: Vec<String>,
}

#[derive(Debug, Clone)]
struct StageCapabilityWorkflowProfile {
    label: String,
    requires_workflow_uuid: bool,
    input_summary: TaskBoardStaffCapabilitySummary,
    result_summary: TaskBoardStaffCapabilitySummary,
    input_contract: String,
    result_semantics: String,
}

fn create_staff_stage_action_workflow_task(
    status: TaskStatus,
    spec: GeneratedStageWorkflowSpec,
) -> iced::Task<crate::app::Message> {
    iced::Task::perform(create_staff_stage_action_workflow(spec), move |result| {
        Message::TaskBoard(TaskBoardMessage::StaffStageActionWorkflowCreated { status, result })
    })
}

fn upsert_staff_stage_workflow_option(
    options: &mut Vec<vw_gateway_client::WorkflowRecordSummary>,
    summary: vw_gateway_client::WorkflowRecordSummary,
) {
    options.retain(|option| option.uuid != summary.uuid);
    options.push(summary);
    options
        .sort_by(|left, right| left.name.cmp(&right.name).then_with(|| left.uuid.cmp(&right.uuid)));
}

#[cfg(not(target_arch = "wasm32"))]
async fn create_staff_stage_action_workflow(
    spec: GeneratedStageWorkflowSpec,
) -> Result<vw_gateway_client::WorkflowRecordSummary, String> {
    let workflow_yaml = generated_stage_workflow_yaml(&spec)?;
    let client = crate::app::config::gateway_client()?;
    let record = client
        .workflow_application_create(&vw_gateway_client::WorkflowRecordUpsertBody {
            uuid: None,
            name: spec.name,
            description: spec.description,
            workflow_yaml,
        })
        .await?;
    Ok(vw_gateway_client::WorkflowRecordSummary {
        uuid: record.uuid,
        name: record.name,
        description: record.description,
        created_at_ms: record.created_at_ms,
        updated_at_ms: record.updated_at_ms,
    })
}

#[cfg(target_arch = "wasm32")]
async fn create_staff_stage_action_workflow(
    spec: GeneratedStageWorkflowSpec,
) -> Result<vw_gateway_client::WorkflowRecordSummary, String> {
    let _ = spec;
    Err("Web 平台暂不支持创建本地 Workflow 应用".to_string())
}

fn build_stage_action_workflow_spec(
    app: &crate::app::App,
    status: TaskStatus,
    workflow_id: &str,
) -> Result<GeneratedStageWorkflowSpec, String> {
    let workflow_id = workflow_id.trim();
    let workflow = task_workflow_by_id(workflow_id)
        .ok_or_else(|| format!("任务工作流未加载: {workflow_id}"))?;
    let status_definition = workflow
        .status_definition(status)
        .ok_or_else(|| format!("阶段不属于工作流 {}: {}", workflow.id(), status.to_string_key()))?;
    let stage_actions = app
        .task_board_staff_stage_action_draft
        .as_deref()
        .unwrap_or(&app.task_board_staff_config.stage_actions);
    let action = stage_actions
        .iter()
        .find(|action| action.status == status && stage_action_has_value(action))
        .cloned()
        .ok_or_else(|| "请先选择需要 Workflow UUID 的阶段能力".to_string())?;
    let profile = stage_capability_workflow_profile(app, &action.capability_id)
        .ok_or_else(|| format!("阶段能力不存在: {}", action.capability_id))?;
    if !profile.requires_workflow_uuid {
        return Err(format!("阶段能力 {} 不需要 Workflow UUID", action.capability_id));
    }

    let input_fields = generated_stage_workflow_input_fields(&profile, &action);
    let result_fields = generated_stage_workflow_result_fields(&profile);
    let status_label = status_definition.label().to_string();
    let name = format!("{status_label} · {}", profile.label);
    let description = format!(
        "由 VibeWindow 阶段能力生成。工作流: {} ({})；阶段: {} ({})；能力: {} ({})。",
        workflow.label(),
        workflow.id(),
        status_label,
        status_definition.key(),
        profile.label,
        action.capability_id
    );
    let input_description =
        summary_description_or_contract(&profile.input_summary, &profile.input_contract);
    let result_description =
        summary_description_or_contract(&profile.result_summary, &profile.result_semantics);

    Ok(GeneratedStageWorkflowSpec {
        name,
        description,
        workflow_label: workflow.label().to_string(),
        workflow_id: workflow.id().to_string(),
        status_label,
        status_key: status_definition.key().to_string(),
        capability_label: profile.label,
        capability_id: action.capability_id,
        input_description,
        result_description,
        input_fields,
        result_fields,
    })
}

fn stage_capability_workflow_profile(
    app: &crate::app::App,
    capability_id: &str,
) -> Option<StageCapabilityWorkflowProfile> {
    app.task_board_staff_capability_catalog
        .iter()
        .find(|capability| capability.id == capability_id)
        .map(|capability| StageCapabilityWorkflowProfile {
            label: capability.label.clone(),
            requires_workflow_uuid: capability.requires_workflow_uuid,
            input_summary: capability.input_summary.clone(),
            result_summary: capability.result_summary.clone(),
            input_contract: capability.input_contract.clone(),
            result_semantics: capability.result_semantics.clone(),
        })
        .or_else(|| {
            stage_capability_registry()
                .all()
                .iter()
                .find(|capability| capability.stable_id == capability_id)
                .map(|capability| StageCapabilityWorkflowProfile {
                    label: capability.label.to_string(),
                    requires_workflow_uuid: capability.requires_workflow_uuid,
                    input_summary: TaskBoardStaffCapabilitySummary::default(),
                    result_summary: TaskBoardStaffCapabilitySummary::default(),
                    input_contract: capability.input_contract.to_string(),
                    result_semantics: capability.result_semantics.to_string(),
                })
        })
}

fn summary_description_or_contract(
    summary: &TaskBoardStaffCapabilitySummary,
    fallback: &str,
) -> String {
    let description = summary.description.trim();
    if description.is_empty() { fallback.to_string() } else { description.to_string() }
}

fn generated_stage_workflow_input_fields(
    profile: &StageCapabilityWorkflowProfile,
    action: &TaskBoardStageAction,
) -> Vec<String> {
    let mut fields = Vec::new();
    for field in standard_stage_workflow_input_fields() {
        push_unique_workflow_field(&mut fields, field);
    }
    for field in &profile.input_summary.fields {
        push_unique_workflow_field(&mut fields, field);
    }
    for field in action.input_parameters.keys() {
        push_unique_workflow_field(&mut fields, field);
    }
    fields
}

fn generated_stage_workflow_result_fields(profile: &StageCapabilityWorkflowProfile) -> Vec<String> {
    let mut fields = Vec::new();
    for field in &profile.result_summary.fields {
        push_unique_workflow_field(&mut fields, field);
    }
    if fields.is_empty() {
        fields.push("outcome".to_string());
        fields.push("summary".to_string());
    }
    fields
}

fn standard_stage_workflow_input_fields() -> [&'static str; 9] {
    [
        "task_id",
        "task_summary",
        "status_key",
        "status_label",
        "workflow_id",
        "staff_member_id",
        "staff_member_name",
        "staff_role",
        "priority",
    ]
}

fn push_unique_workflow_field(fields: &mut Vec<String>, field: &str) {
    let field = sanitize_workflow_variable_name(field);
    if field.is_empty() || is_reserved_workflow_runtime_input(&field) {
        return;
    }
    if !fields.iter().any(|existing| existing == &field) {
        fields.push(field);
    }
}

fn sanitize_workflow_variable_name(field: &str) -> String {
    let mut value = field
        .trim()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '_' { ch } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if value.is_empty() {
        return value;
    }
    if value.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        value.insert_str(0, "field_");
    }
    value
}

fn is_reserved_workflow_runtime_input(field: &str) -> bool {
    matches!(field.trim(), "workflow_uuid" | "application_uuid")
}

#[cfg(not(target_arch = "wasm32"))]
fn generated_stage_workflow_yaml(spec: &GeneratedStageWorkflowSpec) -> Result<String, String> {
    use serde_json::json;

    let variables = spec
        .input_fields
        .iter()
        .map(|field| {
            json!({
                "label": generated_workflow_field_label(field),
                "required": false,
                "type": "paragraph",
                "variable": field,
            })
        })
        .collect::<Vec<_>>();
    let answer = generated_stage_workflow_answer(spec);
    let workflow = json!({
        "app": {
            "description": &spec.description,
            "icon": "🤖",
            "icon_background": "#FFEAD5",
            "mode": "advanced-chat",
            "name": &spec.name,
            "use_icon_as_answer_icon": false
        },
        "kind": "app",
        "version": "0.5.0",
        "workflow": {
            "conversation_variables": [],
            "environment_variables": [],
            "features": {},
            "graph": {
                "nodes": [
                    {
                        "data": {
                            "desc": &spec.input_description,
                            "selected": false,
                            "title": "开始",
                            "type": "start",
                            "variables": variables
                        },
                        "height": 120,
                        "id": "start-node",
                        "position": {"x": -180, "y": 120},
                        "positionAbsolute": {"x": -180, "y": 120},
                        "selected": false,
                        "sourcePosition": "right",
                        "targetPosition": "left",
                        "type": "custom",
                        "width": 240
                    },
                    {
                        "data": {
                            "answer": answer,
                            "desc": &spec.result_description,
                            "selected": false,
                            "title": "回复",
                            "type": "answer",
                            "variables": []
                        },
                        "height": 116,
                        "id": "answer-node",
                        "position": {"x": 140, "y": 120},
                        "positionAbsolute": {"x": 140, "y": 120},
                        "selected": false,
                        "sourcePosition": "right",
                        "targetPosition": "left",
                        "type": "custom",
                        "width": 240
                    }
                ],
                "edges": [
                    {
                        "data": {
                            "isInLoop": false,
                            "sourceType": "start",
                            "targetType": "answer"
                        },
                        "id": "start-node-source-answer-node-target",
                        "selected": false,
                        "source": "start-node",
                        "sourceHandle": "source",
                        "target": "answer-node",
                        "targetHandle": "target",
                        "type": "custom",
                        "zIndex": 0
                    }
                ],
                "viewport": {"x": 160, "y": 120, "zoom": 1}
            }
        }
    });

    serde_yaml::to_string(&workflow).map_err(|error| format!("生成阶段 Workflow 失败: {error}"))
}

#[cfg(not(target_arch = "wasm32"))]
fn generated_workflow_field_label(field: &str) -> String {
    field.replace('_', " ")
}

#[cfg(not(target_arch = "wasm32"))]
fn generated_stage_workflow_answer(spec: &GeneratedStageWorkflowSpec) -> String {
    format!(
        "阶段 Workflow 模板已执行。\n工作流：{} ({})\n阶段：{} ({})\n能力：{} ({})\n任务摘要：{{{{#start-node.task_summary#}}}}\n\n入参字段：{}\n返回字段：{}\n结果去向：按任务流管理中的成功/失败边自动推进",
        spec.workflow_label,
        spec.workflow_id,
        spec.status_label,
        spec.status_key,
        spec.capability_label,
        spec.capability_id,
        spec.input_fields.join(", "),
        spec.result_fields.join(", "),
    )
}

async fn load_gateway_staff_projection(
    project_path: String,
    workflow_id: String,
) -> Result<TaskBoardStaffConfig, String> {
    let staff_config = load_gateway_staff_config_for_workflow(&project_path, &workflow_id).await?;
    crate::app::task::task_board_staff_config_from_staff_config_for_workflow_id(
        staff_config,
        &workflow_id,
    )
}

async fn load_gateway_staff_config_for_workflow(
    project_path: &str,
    workflow_id: &str,
) -> Result<vw_config_types::staff::StaffConfig, String> {
    let mut staff_config = load_gateway_staff_config().await?;
    let client = crate::app::config::gateway_client()?;
    let scope = resolve_task_board_scheduler_scope(project_path).await?;
    let orchestration = client.staff_workflow_orchestration(&scope, workflow_id).await?;
    staff_config.project_state =
        staff_project_state_from_workflow_orchestration(orchestration.orchestration)?;
    Ok(staff_config)
}

#[cfg(not(target_arch = "wasm32"))]
async fn load_gateway_staff_config() -> Result<vw_config_types::staff::StaffConfig, String> {
    crate::app::config::load_staff_config_result()
        .map_err(crate::app::config::server_config_unreachable_error)
}

#[cfg(target_arch = "wasm32")]
async fn load_gateway_staff_config() -> Result<vw_config_types::staff::StaffConfig, String> {
    crate::app::config::load_full_agent_config_async()
        .await
        .map(|config| config.staff)
        .map_err(crate::app::config::server_config_unreachable_error)
}

fn persist_gateway_staff_projection_task(
    config: TaskBoardStaffConfig,
    project_path: String,
    workflow_id: String,
    mapper: fn(Result<(), String>) -> TaskBoardMessage,
) -> iced::Task<crate::app::Message> {
    iced::Task::perform(
        persist_gateway_staff_projection(config, project_path, workflow_id),
        move |result| Message::TaskBoard(mapper(result)),
    )
}

fn persist_gateway_staff_orchestration_task(
    app: &crate::app::App,
    config: TaskBoardStaffConfig,
    workflow_id: String,
    mapper: fn(Result<(), String>) -> TaskBoardMessage,
) -> iced::Task<crate::app::Message> {
    let project_path = app.project_path.clone();
    iced::Task::perform(
        persist_gateway_staff_orchestration(project_path, config, workflow_id),
        move |result| Message::TaskBoard(mapper(result)),
    )
}

fn save_staff_capability_catalog_task(
    app: &crate::app::App,
    workflow_id: String,
    draft: Vec<TaskBoardStaffCapability>,
) -> iced::Task<crate::app::Message> {
    let project_path = app.project_path.clone();
    let requested_workflow_id = workflow_id.clone();
    iced::Task::perform(
        save_staff_capability_catalog(project_path, workflow_id, draft),
        move |result| {
            Message::TaskBoard(TaskBoardMessage::StaffCapabilityCatalogSavedForWorkflow {
                workflow_id: requested_workflow_id.clone(),
                result,
            })
        },
    )
}

async fn save_staff_capability_catalog(
    project_path: Option<String>,
    workflow_id: String,
    draft: Vec<TaskBoardStaffCapability>,
) -> Result<Vec<TaskBoardStaffCapability>, String> {
    let project_path =
        project_path.ok_or_else(|| "项目路径缺失，无法保存员工能力目录".to_string())?;
    let request = task_board_staff_capability_catalog_request(&draft);
    let client = crate::app::config::gateway_client()?;
    let scope = resolve_task_board_scheduler_scope(&project_path).await?;
    client
        .save_staff_workflow_capabilities(&scope, &workflow_id, &request)
        .await
        .map(task_board_staff_capabilities_from_gateway_response)
}

async fn persist_gateway_staff_orchestration(
    project_path: Option<String>,
    config: TaskBoardStaffConfig,
    workflow_id: String,
) -> Result<(), String> {
    let project_path = project_path.ok_or_else(|| "项目路径缺失，无法保存员工编排".to_string())?;
    let mut staff_config =
        load_gateway_staff_config_for_workflow(&project_path, &workflow_id).await?;
    crate::app::task::apply_task_board_staff_config_to_staff_config_for_workflow_id(
        &mut staff_config,
        &config,
        &workflow_id,
    )?;
    let request = vw_gateway_client::vw_api_types::staff::StaffWorkflowOrchestrationRequest {
        workflow_id: workflow_id.clone(),
        orchestration: staff_workflow_orchestration_to_gateway(staff_config.project_state)?,
    };
    let client = crate::app::config::gateway_client()?;
    let scope = resolve_task_board_scheduler_scope(&project_path).await?;
    client
        .save_staff_workflow_orchestration(&scope, &workflow_id, &request)
        .await
        .map_err(crate::app::config::server_config_unreachable_error)
        .map(|_| ())
}

#[cfg(not(target_arch = "wasm32"))]
async fn persist_gateway_staff_projection(
    config: TaskBoardStaffConfig,
    project_path: String,
    workflow_id: String,
) -> Result<(), String> {
    let scope = resolve_task_board_scheduler_scope(&project_path).await?;
    let workflow = task_workflow_by_id(&workflow_id)
        .ok_or_else(|| format!("任务工作流未加载: {workflow_id}"))?;
    let mut staff_config =
        load_gateway_staff_config_for_workflow(&project_path, &workflow_id).await?;
    crate::app::task::apply_task_board_staff_config_to_staff_config_for_workflow_id(
        &mut staff_config,
        &config,
        &workflow_id,
    )?;
    let request = vw_gateway_client::vw_api_types::staff::StaffProjectOrchestrationRequest {
        workflow: task_workflow_snapshot_for_gateway(&workflow),
        project_state: staff_project_state_to_gateway(staff_config.project_state)?,
    };
    let client = crate::app::config::gateway_client()?;
    client
        .save_staff_project_orchestration(&scope, &request)
        .await
        .map_err(crate::app::config::server_config_unreachable_error)
        .map(|_| ())
}

#[cfg(target_arch = "wasm32")]
async fn persist_gateway_staff_projection(
    config: TaskBoardStaffConfig,
    project_path: String,
    workflow_id: String,
) -> Result<(), String> {
    let scope = resolve_task_board_scheduler_scope(&project_path).await?;
    let workflow = task_workflow_by_id(&workflow_id)
        .ok_or_else(|| format!("任务工作流未加载: {workflow_id}"))?;
    let mut staff_config =
        load_gateway_staff_config_for_workflow(&project_path, &workflow_id).await?;
    crate::app::task::apply_task_board_staff_config_to_staff_config_for_workflow_id(
        &mut staff_config,
        &config,
        &workflow_id,
    )?;
    let request = vw_gateway_client::vw_api_types::staff::StaffProjectOrchestrationRequest {
        workflow: task_workflow_snapshot_for_gateway(&workflow),
        project_state: staff_project_state_to_gateway(staff_config.project_state)?,
    };
    let client = crate::app::config::gateway_client()?;
    client
        .save_staff_project_orchestration(&scope, &request)
        .await
        .map_err(crate::app::config::server_config_unreachable_error)
        .map(|_| ())
}

fn staff_project_state_to_gateway(
    project_state: vw_config_types::staff::StaffProjectStateConfig,
) -> Result<vw_gateway_client::vw_api_types::staff::StaffProjectStateDto, String> {
    let value = serde_json::to_value(project_state).map_err(|err| err.to_string())?;
    serde_json::from_value(value).map_err(|err| err.to_string())
}

fn staff_project_state_from_gateway(
    project_state: vw_gateway_client::vw_api_types::staff::StaffProjectStateDto,
) -> Result<vw_config_types::staff::StaffProjectStateConfig, String> {
    let value = serde_json::to_value(project_state).map_err(|err| err.to_string())?;
    serde_json::from_value(value).map_err(|err| err.to_string())
}

fn staff_project_state_from_workflow_orchestration(
    orchestration: vw_gateway_client::vw_api_types::staff::StaffWorkflowOrchestrationDto,
) -> Result<vw_config_types::staff::StaffProjectStateConfig, String> {
    staff_project_state_from_gateway(vw_gateway_client::vw_api_types::staff::StaffProjectStateDto {
        capabilities: orchestration.capabilities,
        stage_assignments: orchestration.stage_assignments,
        stage_workflow_bindings: Vec::new(),
        stage_actions: orchestration.stage_actions,
        member_task_assignments: Vec::new(),
    })
}

fn staff_workflow_orchestration_to_gateway(
    project_state: vw_config_types::staff::StaffProjectStateConfig,
) -> Result<vw_gateway_client::vw_api_types::staff::StaffWorkflowOrchestrationDto, String> {
    let project_state = staff_project_state_to_gateway(project_state)?;
    Ok(vw_gateway_client::vw_api_types::staff::StaffWorkflowOrchestrationDto {
        capabilities: project_state.capabilities,
        stage_assignments: project_state.stage_assignments,
        stage_workflow_bindings: project_state.stage_workflow_bindings,
        stage_actions: project_state.stage_actions,
    })
}

fn staff_command_from_gateway(
    command: vw_gateway_client::vw_api_types::staff::StaffCommandDto,
) -> TaskBoardStaffCommand {
    use vw_gateway_client::vw_api_types::staff::StaffCommandDto;

    match command {
        StaffCommandDto::AssignTask { command_id, task_id, member_id } => {
            TaskBoardStaffCommand::AssignTask { command_id, task_id, member_id }
        }
        StaffCommandDto::TakeOverTask { command_id, task_id, member_id } => {
            TaskBoardStaffCommand::TakeOverTask { command_id, task_id, member_id }
        }
        StaffCommandDto::PauseTask { command_id, task_id, member_id } => {
            TaskBoardStaffCommand::PauseTask { command_id, task_id, member_id }
        }
        StaffCommandDto::ResumeTask { command_id, task_id, member_id } => {
            TaskBoardStaffCommand::ResumeTask { command_id, task_id, member_id }
        }
        StaffCommandDto::StartTask { command_id, task_id, member_id } => {
            TaskBoardStaffCommand::StartTask { command_id, task_id, member_id }
        }
        StaffCommandDto::OpenTaskLogs { command_id, task_id } => {
            TaskBoardStaffCommand::OpenTaskLogs { command_id, task_id }
        }
        StaffCommandDto::FilterByMember { command_id, member_id } => {
            TaskBoardStaffCommand::FilterByMember { command_id, member_id }
        }
    }
}

fn staff_command_to_gateway(
    command: &TaskBoardStaffCommand,
) -> vw_gateway_client::vw_api_types::staff::StaffCommandDto {
    use vw_gateway_client::vw_api_types::staff::StaffCommandDto;

    match command {
        TaskBoardStaffCommand::AssignTask { command_id, task_id, member_id } => {
            StaffCommandDto::AssignTask {
                command_id: command_id.clone(),
                task_id: task_id.clone(),
                member_id: member_id.clone(),
            }
        }
        TaskBoardStaffCommand::TakeOverTask { command_id, task_id, member_id } => {
            StaffCommandDto::TakeOverTask {
                command_id: command_id.clone(),
                task_id: task_id.clone(),
                member_id: member_id.clone(),
            }
        }
        TaskBoardStaffCommand::PauseTask { command_id, task_id, member_id } => {
            StaffCommandDto::PauseTask {
                command_id: command_id.clone(),
                task_id: task_id.clone(),
                member_id: member_id.clone(),
            }
        }
        TaskBoardStaffCommand::ResumeTask { command_id, task_id, member_id } => {
            StaffCommandDto::ResumeTask {
                command_id: command_id.clone(),
                task_id: task_id.clone(),
                member_id: member_id.clone(),
            }
        }
        TaskBoardStaffCommand::StartTask { command_id, task_id, member_id } => {
            StaffCommandDto::StartTask {
                command_id: command_id.clone(),
                task_id: task_id.clone(),
                member_id: member_id.clone(),
            }
        }
        TaskBoardStaffCommand::OpenTaskLogs { command_id, task_id } => {
            StaffCommandDto::OpenTaskLogs {
                command_id: command_id.clone(),
                task_id: task_id.clone(),
            }
        }
        TaskBoardStaffCommand::FilterByMember { command_id, member_id } => {
            StaffCommandDto::FilterByMember {
                command_id: command_id.clone(),
                member_id: member_id.clone(),
            }
        }
    }
}

fn staff_level_from_gateway(value: &str) -> TaskBoardStaffLevel {
    match value.trim() {
        "principal" => TaskBoardStaffLevel::Principal,
        "manager" => TaskBoardStaffLevel::Manager,
        "executor" => TaskBoardStaffLevel::Executor,
        _ => TaskBoardStaffLevel::Member,
    }
}

fn staff_orchestration_context_from_gateway(
    context: vw_gateway_client::vw_api_types::staff::StaffOrchestrationContextDto,
) -> TaskBoardStaffOrchestrationContext {
    TaskBoardStaffOrchestrationContext {
        stage_status: TaskStatus::parse_key(&context.status_key).unwrap_or(TaskStatus::Pool),
        stage_label: context.stage_label,
        member_level: staff_level_from_gateway(&context.member_level),
        member_level_label: context.member_level_label,
        member_name: context.member_name,
        action_source: context.action_source,
        target: context.target,
        manager_member_names: context.manager_member_names,
    }
}

fn empty_stage_assignment(status: TaskStatus) -> TaskBoardStageAssignment {
    TaskBoardStageAssignment {
        status,
        primary_member_id: None,
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: false,
    }
}

fn next_custom_staff_capability_id(draft: &[TaskBoardStaffCapability]) -> String {
    for index in 1.. {
        let id = format!("custom_capability_{index}");
        if !draft.iter().any(|capability| capability.id == id) {
            return id;
        }
    }
    "custom_capability".to_string()
}

fn normalize_capability_summary_fields(fields: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for field in fields {
        let field = field.trim();
        if !field.is_empty() && !normalized.iter().any(|existing: &String| existing == field) {
            normalized.push(field.to_string());
        }
    }
    normalized
}

fn update_staff_capability_draft_field(
    draft: &mut [TaskBoardStaffCapability],
    capability_id: &str,
    field: TaskBoardStaffCapabilityDraftField,
    value: String,
) {
    let Some(index) = draft.iter().position(|capability| capability.id == capability_id) else {
        return;
    };
    if !draft[index].editable() {
        return;
    }

    match field {
        TaskBoardStaffCapabilityDraftField::Id => {
            let value = value.trim();
            if value.is_empty()
                || draft
                    .iter()
                    .enumerate()
                    .any(|(other_index, capability)| other_index != index && capability.id == value)
            {
                return;
            }
            draft[index].id = value.to_string();
        }
        TaskBoardStaffCapabilityDraftField::Label => {
            draft[index].label = value;
        }
    }
}

fn update_staff_capability_summary_draft(
    draft: &mut [TaskBoardStaffCapability],
    capability_id: &str,
    summary: TaskBoardStaffCapabilitySummaryKind,
    description: String,
    fields: Vec<String>,
) {
    let Some(capability) =
        draft.iter_mut().find(|capability| capability.id == capability_id && capability.editable())
    else {
        return;
    };

    let target = match summary {
        TaskBoardStaffCapabilitySummaryKind::Input => &mut capability.input_summary,
        TaskBoardStaffCapabilitySummaryKind::Result => &mut capability.result_summary,
    };
    target.description = description;
    target.fields = normalize_capability_summary_fields(fields);
}

fn start_staff_capability_catalog_load(
    app: &mut crate::app::App,
) -> iced::Task<crate::app::Message> {
    start_staff_capability_catalog_load_for_workflow(app, current_staff_workflow_id(app))
}

fn start_staff_capability_catalog_load_for_workflow(
    app: &mut crate::app::App,
    workflow_id: String,
) -> iced::Task<crate::app::Message> {
    app.task_board_staff_capability_catalog_loading = true;
    app.task_board_staff_capability_catalog_error = None;
    load_staff_capability_catalog_task(app, workflow_id)
}

fn staff_stage_assignment_draft_mut(
    app: &mut crate::app::App,
) -> &mut Vec<TaskBoardStageAssignment> {
    if app.task_board_staff_assignment_draft.is_none() {
        app.task_board_staff_assignment_draft =
            Some(app.task_board_staff_config.stage_assignments.clone());
    }
    app.task_board_staff_assignment_draft
        .as_mut()
        .expect("staff stage assignment draft initialized")
}

fn stage_action_draft_from_config(config: &TaskBoardStaffConfig) -> Vec<TaskBoardStageAction> {
    let mut actions = config.stage_actions.clone();
    for binding in &config.stage_workflow_bindings {
        let workflow_uuid = binding.workflow_uuid.trim();
        if workflow_uuid.is_empty() || actions.iter().any(|action| action.status == binding.status)
        {
            continue;
        }
        actions.push(TaskBoardStageAction {
            status: binding.status,
            capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
            workflow_uuid: Some(workflow_uuid.to_string()),
            input_parameters: Default::default(),
        });
    }
    actions
}

fn stage_workflow_bindings_for_stage_actions(
    actions: &[TaskBoardStageAction],
) -> Vec<TaskBoardStageWorkflowBinding> {
    actions
        .iter()
        .filter(|action| action.capability_id.trim() == SAVED_WORKFLOW_CAPABILITY_ID)
        .filter_map(|action| {
            action
                .workflow_uuid
                .as_deref()
                .map(str::trim)
                .filter(|workflow_uuid| !workflow_uuid.is_empty())
                .map(|workflow_uuid| TaskBoardStageWorkflowBinding {
                    status: action.status,
                    workflow_uuid: workflow_uuid.to_string(),
                })
        })
        .collect()
}

fn staff_stage_action_draft_mut(app: &mut crate::app::App) -> &mut Vec<TaskBoardStageAction> {
    if app.task_board_staff_stage_action_draft.is_none() {
        app.task_board_staff_stage_action_draft =
            Some(stage_action_draft_from_config(&app.task_board_staff_config));
    }
    app.task_board_staff_stage_action_draft.as_mut().expect("staff stage action draft initialized")
}

fn refresh_stage_workflow_binding_draft_from_action_draft(app: &mut crate::app::App) {
    let bindings = app
        .task_board_staff_stage_action_draft
        .as_deref()
        .map(stage_workflow_bindings_for_stage_actions)
        .unwrap_or_else(|| {
            stage_workflow_bindings_for_stage_actions(&stage_action_draft_from_config(
                &app.task_board_staff_config,
            ))
        });
    app.task_board_staff_workflow_binding_draft = Some(bindings);
}

fn ensure_stage_assignment(
    assignments: &mut Vec<TaskBoardStageAssignment>,
    status: TaskStatus,
) -> &mut TaskBoardStageAssignment {
    if let Some(index) = assignments.iter().position(|assignment| assignment.status == status) {
        return &mut assignments[index];
    }
    assignments.push(empty_stage_assignment(status));
    let index = assignments.len() - 1;
    &mut assignments[index]
}

fn normalize_optional_member_id(member_id: Option<String>) -> Option<String> {
    member_id.as_deref().map(str::trim).filter(|value| !value.is_empty()).map(ToString::to_string)
}

fn normalize_optional_workflow_uuid(workflow_uuid: Option<String>) -> Option<String> {
    workflow_uuid
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn normalize_optional_capability_id(capability_id: Option<String>) -> Option<String> {
    capability_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn stage_capability_requires_workflow_uuid(app: &crate::app::App, capability_id: &str) -> bool {
    app.task_board_staff_capability_catalog
        .iter()
        .find(|capability| capability.id == capability_id)
        .map(|capability| capability.requires_workflow_uuid)
        .or_else(|| {
            stage_capability_registry()
                .all()
                .iter()
                .find(|capability| capability.stable_id == capability_id)
                .map(|capability| capability.requires_workflow_uuid)
        })
        .unwrap_or(false)
}

fn set_stage_action_capability(
    actions: &mut Vec<TaskBoardStageAction>,
    status: TaskStatus,
    capability_id: Option<String>,
    requires_workflow_uuid: bool,
) {
    let Some(capability_id) = normalize_optional_capability_id(capability_id) else {
        actions.retain(|action| action.status != status);
        return;
    };

    if let Some(action) = actions.iter_mut().find(|action| action.status == status) {
        let changed = action.capability_id != capability_id;
        action.capability_id = capability_id;
        if changed {
            action.input_parameters.clear();
        }
        if !requires_workflow_uuid {
            action.workflow_uuid = None;
        }
    } else {
        actions.push(TaskBoardStageAction {
            status,
            capability_id,
            workflow_uuid: None,
            input_parameters: Default::default(),
        });
    }
}

fn set_stage_action_workflow(
    actions: &mut Vec<TaskBoardStageAction>,
    status: TaskStatus,
    workflow_uuid: Option<String>,
) {
    let workflow_uuid = normalize_optional_workflow_uuid(workflow_uuid);
    if let Some(action) = actions.iter_mut().find(|action| action.status == status) {
        if action.capability_id.trim().is_empty() {
            action.capability_id = SAVED_WORKFLOW_CAPABILITY_ID.to_string();
        }
        action.workflow_uuid = workflow_uuid;
    } else {
        actions.push(TaskBoardStageAction {
            status,
            capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
            workflow_uuid,
            input_parameters: Default::default(),
        });
    }
}

fn set_stage_action_parameter(
    actions: &mut [TaskBoardStageAction],
    status: TaskStatus,
    key: String,
    value: String,
) {
    let key = key.trim();
    if key.is_empty() {
        return;
    }
    let Some(action) = actions.iter_mut().find(|action| action.status == status) else {
        return;
    };
    let value = value.trim();
    if value.is_empty() {
        action.input_parameters.remove(key);
    } else {
        action.input_parameters.insert(key.to_string(), Value::String(value.to_string()));
    }
}

fn remove_member_id(member_ids: &mut Vec<String>, member_id: &str) {
    member_ids.retain(|candidate| candidate != member_id);
}

fn toggle_member_id(member_ids: &mut Vec<String>, member_id: &str) -> bool {
    let member_id = member_id.trim();
    if member_id.is_empty() {
        return false;
    }
    if let Some(index) = member_ids.iter().position(|candidate| candidate == member_id) {
        member_ids.remove(index);
        false
    } else {
        member_ids.push(member_id.to_string());
        true
    }
}

fn stage_assignment_has_value(assignment: &TaskBoardStageAssignment) -> bool {
    assignment.primary_member_id.is_some()
        || !assignment.backup_member_ids.is_empty()
        || !assignment.manager_member_ids.is_empty()
        || assignment.autonomous
}

fn stage_action_has_value(action: &TaskBoardStageAction) -> bool {
    !action.capability_id.trim().is_empty()
}

fn task_status_order(status: TaskStatus, workflow: &TaskWorkflow) -> usize {
    workflow
        .statuses()
        .iter()
        .position(|candidate| candidate.status == status)
        .unwrap_or(usize::MAX)
}

fn prune_stage_assignment_draft(
    assignments: &mut Vec<TaskBoardStageAssignment>,
    workflow: &TaskWorkflow,
) {
    let workflow_statuses =
        workflow.statuses().iter().map(|definition| definition.status).collect::<Vec<_>>();
    assignments.retain(|assignment| {
        stage_assignment_has_value(assignment) && workflow_statuses.contains(&assignment.status)
    });
    assignments.sort_by_key(|assignment| task_status_order(assignment.status, workflow));
}

fn prune_stage_action_draft(actions: &mut Vec<TaskBoardStageAction>, workflow: &TaskWorkflow) {
    let workflow_statuses =
        workflow.statuses().iter().map(|definition| definition.status).collect::<Vec<_>>();
    actions.retain(|action| {
        stage_action_has_value(action) && workflow_statuses.contains(&action.status)
    });
    actions.sort_by_key(|action| task_status_order(action.status, workflow));
}

fn save_staff_stage_assignment(
    app: &mut crate::app::App,
    workflow_id: Option<String>,
) -> iced::Task<crate::app::Message> {
    let workflow_id = workflow_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| current_staff_workflow_id(app));
    let workflow = match task_workflow_by_id(&workflow_id) {
        Some(workflow) => workflow,
        None => {
            return app.show_error_toast(format!(
                "阶段分配保存失败：unknown task board workflow id: {workflow_id}"
            ));
        }
    };
    let mut config = app.task_board_staff_config.clone();
    config.stage_assignments = app
        .task_board_staff_assignment_draft
        .clone()
        .unwrap_or_else(|| config.stage_assignments.clone());
    prune_stage_assignment_draft(&mut config.stage_assignments, &workflow);
    config.stage_actions = app
        .task_board_staff_stage_action_draft
        .clone()
        .unwrap_or_else(|| stage_action_draft_from_config(&config));
    prune_stage_action_draft(&mut config.stage_actions, &workflow);
    config.stage_workflow_bindings =
        stage_workflow_bindings_for_stage_actions(&config.stage_actions);

    let normalized = match config.retarget_for_workflow_id(&workflow_id) {
        Ok(config) => config,
        Err(error) => {
            return app.show_error_toast(format!("阶段分配保存失败：{error}"));
        }
    };

    app.task_board_staff_config = normalized.clone();
    sync_staff_settings_orchestration_from_task_board(app, &normalized, &workflow_id);
    app.task_board_staff_assignment_open = false;
    app.task_board_staff_assignment_draft = None;
    app.task_board_staff_stage_action_draft = None;
    app.task_board_staff_workflow_binding_draft = None;
    refresh_staff_projection(app);
    record_staff_event(
        app,
        crate::app::task::staff::TaskBoardStaffEvent::command_event(
            format!("staff-stage-assignment-{}", now_ms()),
            None,
            None,
            TaskBoardStaffCommandStatus::Applied,
            now_ms(),
            "阶段分配已保存并刷新员工投影",
            None,
        ),
    );

    let tasks = vec![
        app.show_success_toast("阶段分配已保存"),
        persist_gateway_staff_orchestration_task(
            app,
            normalized,
            workflow_id,
            TaskBoardMessage::StaffStageAssignmentPersisted,
        ),
    ];

    iced::Task::batch(tasks)
}

fn sync_staff_settings_orchestration_from_task_board(
    app: &mut crate::app::App,
    task_board_config: &TaskBoardStaffConfig,
    workflow_id: &str,
) {
    let mut staff_config = app.staff_settings.config.clone();
    match crate::app::task::apply_task_board_staff_config_to_staff_config_for_workflow_id(
        &mut staff_config,
        task_board_config,
        workflow_id,
    ) {
        Ok(()) => {
            let project_state = staff_config.project_state;
            app.staff_settings.config.project_state.member_task_assignments =
                project_state.member_task_assignments.clone();
            app.staff_settings.config.project_state.capabilities.clear();
            app.staff_settings.config.project_state.stage_assignments.clear();
            app.staff_settings.config.project_state.stage_workflow_bindings.clear();
            app.staff_settings.config.project_state.stage_actions.clear();
            app.staff_settings.config.workflow_orchestrations.insert(
                workflow_id.trim().to_string(),
                vw_config_types::staff::StaffWorkflowOrchestrationConfig {
                    capabilities: project_state.capabilities,
                    stage_assignments: project_state.stage_assignments,
                    stage_workflow_bindings: Vec::new(),
                    stage_actions: project_state.stage_actions,
                },
            );
        }
        Err(_error) => {}
    }
}

const STAFF_EVENT_LIMIT: usize = 80;

fn record_staff_event(
    app: &mut crate::app::App,
    event: crate::app::task::staff::TaskBoardStaffEvent,
) {
    app.task_board_staff_events.push(event);
    if app.task_board_staff_events.len() > STAFF_EVENT_LIMIT {
        let overflow = app.task_board_staff_events.len() - STAFF_EVENT_LIMIT;
        app.task_board_staff_events.drain(0..overflow);
    }
}

fn run_explicit_staff_auto_schedule(app: &mut crate::app::App) -> iced::Task<crate::app::Message> {
    refresh_staff_projection(app);
    if let Some(error) = app.task_board_staff_config_load_error.clone() {
        return app.show_error_toast(format!("自主调度已阻塞：{error}"));
    }
    if let Some(project_path) = app.project_path.clone() {
        let running_task_ids = app.task_board_executor.running_tasks.clone();
        let now = now_ms();
        return iced::Task::perform(
            async move {
                let scope = resolve_task_board_scheduler_scope(&project_path).await?;
                let client = crate::app::config::gateway_client()?;
                client
                    .staff_auto_schedule_plan(
                        &scope,
                        &vw_gateway_client::vw_api_types::staff::StaffAutoSchedulePlanRequest {
                            running_task_ids,
                            now_ms: Some(vw_gateway_client::vw_api_types::common::TimestampMs(
                                i64::try_from(now).unwrap_or(i64::MAX),
                            )),
                        },
                    )
                    .await
            },
            move |result| {
                Message::TaskBoard(TaskBoardMessage::StaffAutoSchedulePlanLoaded { result })
            },
        );
    }
    let workflow_id = current_staff_workflow_id(app);
    let timestamp_ms = now_ms();
    let plan = match crate::app::task::staff::build_staff_auto_schedule_plan(
        &app.task_board_tasks,
        &app.task_board_staff_config,
        &app.task_board_staff_projection,
        &app.task_board_executor.running_tasks,
        &workflow_id,
        timestamp_ms,
    ) {
        Ok(plan) => plan,
        Err(error) => {
            app.task_board_staff_config_load_error = Some(error.clone());
            return app.show_error_toast(format!("自主调度已阻塞：{error}"));
        }
    };

    for command_context in plan.command_contexts {
        app.task_board_staff_command_queue.remember_orchestration_context(
            &command_context.command_id,
            command_context.orchestration,
        );
    }

    for skip in plan.skipped {
        record_staff_event(
            app,
            crate::app::task::staff::TaskBoardStaffEvent::command_event(
                format!("staff-auto-skip-{}-{timestamp_ms}", skip.task_id),
                Some(skip.task_id.clone()),
                None,
                TaskBoardStaffCommandStatus::Skipped,
                timestamp_ms,
                format!(
                    "自主调度跳过任务 {}（{}）：{}",
                    skip.task_id,
                    skip.status.label(),
                    skip.reason
                ),
                skip.orchestration,
            ),
        );
    }

    iced::Task::batch(plan.commands.into_iter().map(|command| {
        iced::Task::done(Message::TaskBoard(TaskBoardMessage::SubmitStaffCommandSilently(command)))
    }))
}

fn staff_command_action(
    app: &crate::app::App,
    command: &TaskBoardStaffCommand,
) -> Result<TaskBoardStaffCommandAction, String> {
    crate::app::task::staff::validate_staff_command(
        command,
        &app.task_board_tasks,
        &app.task_board_staff_config,
        &app.task_board_executor.running_tasks,
        &current_staff_workflow_id(app),
    )
    .map_err(|error| error.message)
}

fn apply_staff_command_action(
    app: &mut crate::app::App,
    action: TaskBoardStaffCommandAction,
) -> Result<(String, Vec<iced::Task<crate::app::Message>>), String> {
    match action {
        TaskBoardStaffCommandAction::AssignTask { task_id, member_id } => {
            let tasks = assign_staff_task(app, &task_id, &member_id)?;
            Ok((format!("任务 {task_id} 已分配给员工 {member_id}"), tasks))
        }
        TaskBoardStaffCommandAction::ChangeTaskStatus { task_id, new_status } => Ok((
            format!("任务 {task_id} 已交给状态更新路径：{}", new_status.label()),
            vec![iced::Task::done(Message::TaskBoard(TaskBoardMessage::TaskStatusChanged {
                task_id,
                new_status,
            }))],
        )),
        TaskBoardStaffCommandAction::ExecuteTask { task_id } => Ok((
            format!("任务 {task_id} 已交给执行路径"),
            vec![iced::Task::done(Message::TaskBoard(TaskBoardMessage::ExecuteTask { task_id }))],
        )),
        TaskBoardStaffCommandAction::RunCodeReview { task_id } => Ok((
            format!("任务 {task_id} 已交给代码审核路径"),
            vec![iced::Task::done(Message::TaskBoard(TaskBoardMessage::AutoCodeReviewTick))],
        )),
        TaskBoardStaffCommandAction::RunMerge { task_id } => {
            if !app.task_board_manual_merge_queue.contains(&task_id) {
                app.task_board_manual_merge_queue.push(task_id.clone());
            }
            Ok((
                format!("任务 {task_id} 已交给合并路径"),
                vec![iced::Task::done(Message::TaskBoard(TaskBoardMessage::ManualMergeQueueTick))],
            ))
        }
        TaskBoardStaffCommandAction::RunStageWorkflow {
            task_id,
            status,
            workflow_uuid,
            member_id,
        } => apply_stage_workflow_action(app, task_id, status, workflow_uuid, member_id),
        TaskBoardStaffCommandAction::ViewTaskLogs { task_id } => Ok((
            format!("任务 {task_id} 已打开日志"),
            vec![iced::Task::done(Message::TaskBoard(TaskBoardMessage::ViewTaskLogs(task_id)))],
        )),
        TaskBoardStaffCommandAction::FilterByMember { member_id } => {
            app.task_board_staff_filter_member_id =
                if app.task_board_staff_filter_member_id.as_deref() == Some(member_id.as_str()) {
                    None
                } else {
                    Some(member_id.clone())
                };
            Ok((format!("员工模式已按成员 {member_id} 筛选"), Vec::new()))
        }
    }
}

fn assign_staff_task(
    app: &mut crate::app::App,
    task_id: &str,
    member_id: &str,
) -> Result<Vec<iced::Task<crate::app::Message>>, String> {
    let mut found_member = false;
    for member in &mut app.task_board_staff_config.members {
        member.current_task_ids.retain(|current_task_id| current_task_id != task_id);
        if member.id == member_id {
            found_member = true;
            member.current_task_ids.push(task_id.to_string());
        }
    }
    if !found_member {
        return Err(format!("员工命令失败：员工 {member_id} 不存在"));
    }

    refresh_staff_projection(app);

    let Some(project_path) = app.project_path.clone() else {
        return Ok(Vec::new());
    };
    let config = app.task_board_staff_config.clone();
    Ok(vec![persist_gateway_staff_projection_task(
        config,
        project_path,
        current_staff_workflow_id(app),
        TaskBoardMessage::StaffTaskAssignmentPersisted,
    )])
}

fn apply_stage_workflow_action(
    app: &crate::app::App,
    task_id: String,
    status: TaskStatus,
    workflow_uuid: String,
    member_id: String,
) -> Result<(String, Vec<iced::Task<crate::app::Message>>), String> {
    let project_path = app.project_path.clone().unwrap_or_default();
    let workflow = workflow_for_task_board_settings(app).ok_or_else(|| {
        format!("员工命令失败：任务工作流未加载: {}", current_staff_workflow_id(app))
    })?;
    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == task_id)
        .ok_or_else(|| format!("员工命令失败：任务 {task_id} 不存在"))?;
    if task.status != status {
        return Err(format!(
            "员工命令失败：任务 {task_id} 当前是 {}，阶段 workflow 绑定已过期",
            task.status.label_for_workflow_id(workflow.id())
        ));
    }
    let member = app
        .task_board_staff_config
        .members
        .iter()
        .find(|member| member.id == member_id)
        .ok_or_else(|| format!("员工命令失败：员工 {member_id} 不存在"))?;
    let stage_action = app
        .task_board_staff_config
        .stage_actions
        .iter()
        .find(|action| action.status == status && stage_action_has_value(action));
    let request = build_stage_workflow_request(task, &workflow, status, member, stage_action);
    let result_task_id = task_id.clone();
    let result_workflow_uuid = workflow_uuid.clone();
    let run_workflow_uuid = workflow_uuid.clone();
    Ok((
        format!("任务 {task_id} 已交给阶段 workflow {workflow_uuid}"),
        vec![iced::Task::perform(
            async move {
                let client = crate::app::config::gateway_client()?;
                client.run_workflow_application(&run_workflow_uuid, &request).await
            },
            move |result| {
                Message::TaskBoard(TaskBoardMessage::TaskStageWorkflowCompleted {
                    project_path: project_path.clone(),
                    task_id: result_task_id.clone(),
                    status,
                    workflow_uuid: result_workflow_uuid.clone(),
                    result,
                })
            },
        )],
    ))
}

fn build_stage_workflow_request(
    task: &Task,
    workflow: &TaskWorkflow,
    status: TaskStatus,
    member: &TaskBoardStaffMember,
    stage_action: Option<&TaskBoardStageAction>,
) -> vw_gateway_client::GatewayWorkflowStreamRequest {
    let status_key = workflow
        .status_definition(status)
        .map(|definition| definition.key().to_string())
        .unwrap_or_else(|| status.to_string_key().to_string());
    let status_label = status.label_for_workflow_id(workflow.id());
    let task_summary = safe_stage_workflow_task_summary(task);
    let mut inputs: BTreeMap<String, Value> = [
        ("task_id".to_string(), serde_json::json!(task.id)),
        ("task_summary".to_string(), serde_json::json!(task_summary)),
        ("status_key".to_string(), serde_json::json!(status_key)),
        ("status_label".to_string(), serde_json::json!(status_label)),
        ("workflow_id".to_string(), serde_json::json!(workflow.id())),
        ("staff_member_id".to_string(), serde_json::json!(member.id)),
        ("staff_member_name".to_string(), serde_json::json!(member.name)),
        ("staff_role".to_string(), serde_json::json!(member.role.key())),
        ("priority".to_string(), serde_json::json!(task.priority)),
    ]
    .into();
    if let Some(stage_action) = stage_action {
        merge_stage_action_parameters_into_workflow_inputs(
            &mut inputs,
            &stage_action.input_parameters,
        );
    }

    vw_gateway_client::GatewayWorkflowStreamRequest {
        query: format!(
            "请执行任务 {} 的 {} 阶段 workflow。任务摘要：{}",
            task.id, status_label, task_summary
        ),
        inputs,
        response_mode: vw_gateway_client::GatewayWorkflowResponseMode::Streaming,
        user: member.id.clone(),
        conversation_id: None,
    }
}

fn merge_stage_action_parameters_into_workflow_inputs(
    inputs: &mut BTreeMap<String, Value>,
    parameters: &BTreeMap<String, Value>,
) {
    for (key, value) in parameters {
        let key = key.trim();
        if key.is_empty() || is_reserved_workflow_runtime_input(key) || inputs.contains_key(key) {
            continue;
        }
        inputs.insert(key.to_string(), value.clone());
    }
}

fn safe_stage_workflow_task_summary(task: &Task) -> String {
    let description = task.description.trim();
    let summary = if description.is_empty() {
        format!("任务 {}", task.id)
    } else {
        description.to_string()
    };
    truncate_for_ui(&summary, TASK_LOG_UI_MAX_DETAIL_CHARS)
}

fn submit_staff_command(
    app: &mut crate::app::App,
    command: TaskBoardStaffCommand,
    notify: bool,
) -> iced::Task<crate::app::Message> {
    let should_notify = notify && should_toast_staff_command(&command);
    let event = app.task_board_staff_command_queue.enqueue(command, now_ms());
    let status = event.status;
    let message = event.message.clone();
    record_staff_event(app, event);

    if status == TaskBoardStaffCommandStatus::Skipped {
        return app.show_warning_toast(message);
    }

    let mut tasks = Vec::new();
    if should_notify {
        tasks.push(app.show_info_toast(message));
    }
    tasks.push(iced::Task::done(Message::TaskBoard(TaskBoardMessage::StaffCommandQueueTick)));
    iced::Task::batch(tasks)
}

fn should_toast_staff_command(command: &TaskBoardStaffCommand) -> bool {
    matches!(
        command,
        TaskBoardStaffCommand::AssignTask { .. }
            | TaskBoardStaffCommand::TakeOverTask { .. }
            | TaskBoardStaffCommand::PauseTask { .. }
            | TaskBoardStaffCommand::ResumeTask { .. }
            | TaskBoardStaffCommand::StartTask { .. }
    )
}

fn process_staff_command_queue(app: &mut crate::app::App) -> iced::Task<crate::app::Message> {
    let Some((command, inflight_event)) = app.task_board_staff_command_queue.claim_next(now_ms())
    else {
        return iced::Task::none();
    };
    record_staff_event(app, inflight_event);

    if let Some(project_path) = app.project_path.clone() {
        let command_for_request = command.clone();
        let command_for_result = command.clone();
        return iced::Task::perform(
            async move {
                let scope = resolve_task_board_scheduler_scope(&project_path).await?;
                let client = crate::app::config::gateway_client()?;
                client
                    .staff_command(
                        &scope,
                        &vw_gateway_client::vw_api_types::staff::StaffCommandRequest {
                            command: staff_command_to_gateway(&command_for_request),
                            dry_run: false,
                        },
                    )
                    .await
            },
            move |result| {
                Message::TaskBoard(TaskBoardMessage::StaffCommandGatewayCompleted {
                    command: command_for_result.clone(),
                    result,
                })
            },
        );
    }

    let result = staff_command_action(app, &command)
        .and_then(|action| apply_staff_command_action(app, action));
    let (completion_event, mut tasks) = match result {
        Ok((message, action_tasks)) => {
            let event = app.task_board_staff_command_queue.mark_applied(
                command.command_id(),
                now_ms(),
                message,
            );
            (event, action_tasks)
        }
        Err(error) => {
            let event = app.task_board_staff_command_queue.mark_failed(
                command.command_id(),
                now_ms(),
                error.clone(),
            );
            (event, Vec::new())
        }
    };

    if let Some(event) = completion_event {
        match event.command_status() {
            Some(TaskBoardStaffCommandStatus::Applied) if should_toast_staff_command(&command) => {
                tasks.push(app.show_success_toast(event.message.clone()));
            }
            Some(TaskBoardStaffCommandStatus::Failed) => {
                tasks.push(app.show_error_toast(event.message.clone()));
            }
            _ => {}
        }
        record_staff_event(app, event);
    }

    if app.task_board_staff_command_queue.has_claimable_pending() {
        tasks.push(iced::Task::done(Message::TaskBoard(TaskBoardMessage::StaffCommandQueueTick)));
    }

    iced::Task::batch(tasks)
}

fn apply_staff_command_gateway_action(
    app: &mut crate::app::App,
    action: Option<vw_gateway_client::vw_api_types::staff::StaffCommandActionDto>,
) -> Vec<iced::Task<crate::app::Message>> {
    use vw_gateway_client::vw_api_types::staff::StaffCommandActionDto;

    match action {
        Some(StaffCommandActionDto::ViewTaskLogs { task_id }) => {
            vec![iced::Task::done(Message::TaskBoard(TaskBoardMessage::ViewTaskLogs(task_id)))]
        }
        Some(StaffCommandActionDto::FilterByMember { member_id }) => {
            app.task_board_staff_filter_member_id =
                if app.task_board_staff_filter_member_id.as_deref() == Some(member_id.as_str()) {
                    None
                } else {
                    Some(member_id)
                };
            Vec::new()
        }
        Some(_) => vec![iced::Task::done(Message::TaskBoard(TaskBoardMessage::LoadTasks))],
        None => Vec::new(),
    }
}

fn apply_staff_command_gateway_completed(
    app: &mut crate::app::App,
    command: TaskBoardStaffCommand,
    result: Result<vw_gateway_client::vw_api_types::staff::StaffCommandResponse, String>,
) -> iced::Task<crate::app::Message> {
    let (completion_event, mut tasks) = match result {
        Ok(response)
            if response.accepted
                && response
                    .scheduler_result
                    .as_ref()
                    .is_none_or(|scheduler_result| scheduler_result.accepted) =>
        {
            let action_tasks = apply_staff_command_gateway_action(app, response.action);
            let event = app.task_board_staff_command_queue.mark_applied(
                command.command_id(),
                now_ms(),
                response.message,
            );
            (event, action_tasks)
        }
        Ok(response) => {
            let event = app.task_board_staff_command_queue.mark_failed(
                command.command_id(),
                now_ms(),
                response.message,
            );
            (event, Vec::new())
        }
        Err(error) => {
            let event = app.task_board_staff_command_queue.mark_failed(
                command.command_id(),
                now_ms(),
                error,
            );
            (event, Vec::new())
        }
    };

    if let Some(event) = completion_event {
        match event.command_status() {
            Some(TaskBoardStaffCommandStatus::Applied) if should_toast_staff_command(&command) => {
                tasks.push(app.show_success_toast(event.message.clone()));
            }
            Some(TaskBoardStaffCommandStatus::Failed) => {
                tasks.push(app.show_error_toast(event.message.clone()));
            }
            _ => {}
        }
        record_staff_event(app, event);
    }

    if app.task_board_staff_command_queue.has_claimable_pending() {
        tasks.push(iced::Task::done(Message::TaskBoard(TaskBoardMessage::StaffCommandQueueTick)));
    }

    iced::Task::batch(tasks)
}

fn toggle_staff_task_status_filter(filters: &mut Vec<TaskStatus>, status: TaskStatus) {
    if status == TaskStatus::Archived {
        if filters.as_slice() == [TaskStatus::Archived] {
            filters.clear();
        } else {
            filters.clear();
            filters.push(TaskStatus::Archived);
        }
        return;
    }

    filters.retain(|candidate| *candidate != TaskStatus::Archived);
    if let Some(index) = filters.iter().position(|candidate| *candidate == status) {
        filters.remove(index);
    } else {
        filters.push(status);
    }
    order_staff_task_status_filters(filters);
}

fn order_staff_task_status_filters(filters: &mut Vec<TaskStatus>) {
    let mut ordered = Vec::new();
    for status in TaskStatus::all() {
        if filters.iter().any(|candidate| *candidate == status) {
            ordered.push(status);
        }
    }
    *filters = ordered;
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StaffInputMember {
    id: String,
    name: String,
}

fn select_staff_command_member(
    app: &mut crate::app::App,
    member_id: String,
) -> iced::Task<crate::app::Message> {
    let Some(member) =
        app.task_board_staff_config.members.iter().find(|member| member.id == member_id)
    else {
        return app.show_warning_toast("员工候选已失效，请刷新员工配置");
    };

    app.task_board_staff_command_input =
        staff_command_input_with_member(&app.task_board_staff_command_input, member.name.trim());
    iced::Task::none()
}

fn staff_command_input_with_member(input: &str, member_name: &str) -> String {
    let mention = format!("@{}", member_name.trim());
    let Some(at_pos) = input.rfind('@') else {
        return format!("{mention} 执行 ");
    };

    let token_end = input[at_pos + '@'.len_utf8()..]
        .find(char::is_whitespace)
        .map_or(input.len(), |offset| at_pos + '@'.len_utf8() + offset);
    let prefix = input[..at_pos].trim_end();
    let suffix = input[token_end..].trim_start();
    let command_tail = if suffix.is_empty() {
        "执行 ".to_string()
    } else if staff_command_tail_starts_with_action(suffix) {
        suffix.to_string()
    } else {
        format!("执行 {suffix}")
    };

    if prefix.is_empty() {
        format!("{mention} {command_tail}")
    } else {
        format!("{prefix} {mention} {command_tail}")
    }
}

fn staff_command_tail_starts_with_action(input: &str) -> bool {
    let normalized = input.trim_start().to_ascii_lowercase();
    normalized.starts_with("执行")
        || normalized.starts_with("开始")
        || normalized.starts_with("start")
        || normalized.starts_with("run")
}

fn submit_staff_command_input(app: &mut crate::app::App) -> iced::Task<crate::app::Message> {
    let input = app.task_board_staff_command_input.trim().to_string();
    if input.is_empty() {
        return app.show_warning_toast("请输入员工命令");
    }

    refresh_staff_projection(app);
    let timestamp_ms = now_ms();
    let commands = match staff_commands_from_input(app, &input, timestamp_ms) {
        Ok(commands) => commands,
        Err(error) => return app.show_warning_toast(error),
    };

    app.task_board_staff_command_input.clear();
    let mut tasks = Vec::new();
    let mut has_enqueued_command = false;
    let mut enqueued_messages = Vec::new();
    for command in commands {
        let event = app.task_board_staff_command_queue.enqueue(command, timestamp_ms);
        let status = event.status;
        let message = event.message.clone();
        record_staff_event(app, event);
        if status == TaskBoardStaffCommandStatus::Skipped {
            tasks.push(app.show_warning_toast(message));
        } else {
            has_enqueued_command = true;
            enqueued_messages.push(message);
        }
    }

    if has_enqueued_command {
        let toast_message = match enqueued_messages.as_slice() {
            [message] => message.clone(),
            _ => format!("{} 条员工命令已进入员工命令队列", enqueued_messages.len()),
        };
        tasks.push(app.show_info_toast(toast_message));
        tasks.push(iced::Task::done(Message::TaskBoard(TaskBoardMessage::StaffCommandQueueTick)));
    }

    iced::Task::batch(tasks)
}

fn staff_commands_from_input(
    app: &crate::app::App,
    input: &str,
    timestamp_ms: u64,
) -> Result<Vec<TaskBoardStaffCommand>, String> {
    let member = staff_input_member(&app.task_board_staff_config, input)?;
    if !staff_input_requests_start(input) {
        return Err("暂只支持“@员工 执行 任务ID”".to_string());
    }

    let task_id = staff_input_task_id(app, input, &member.id)?;
    let assignment = find_staff_assignment(&app.task_board_staff_projection, &task_id)
        .ok_or_else(|| format!("任务 {task_id} 不在员工工作流中"))?;
    let mut commands = Vec::new();
    if assignment.member_id.as_deref() != Some(member.id.as_str()) {
        commands.push(TaskBoardStaffCommand::AssignTask {
            command_id: format!("staff-input-assign-{task_id}-{}-{timestamp_ms}", member.id),
            task_id: task_id.clone(),
            member_id: member.id.clone(),
        });
    }
    commands.push(TaskBoardStaffCommand::StartTask {
        command_id: format!("staff-input-start-{task_id}-{}-{timestamp_ms}", member.id),
        task_id,
        member_id: member.id,
    });
    Ok(commands)
}

fn staff_input_member(
    config: &TaskBoardStaffConfig,
    input: &str,
) -> Result<StaffInputMember, String> {
    let input = input.to_ascii_lowercase();
    let mut matches = Vec::new();
    for member in &config.members {
        let id_mention = format!("@{}", member.id).to_ascii_lowercase();
        let name_mention = format!("@{}", member.name).to_ascii_lowercase();
        if input.contains(&id_mention) || input.contains(&name_mention) {
            matches.push(StaffInputMember { id: member.id.clone(), name: member.name.clone() });
        }
    }
    matches.dedup_by(|a, b| a.id == b.id);

    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err("没有找到 @员工".to_string()),
        _ => Err("一次只能 @ 一个员工".to_string()),
    }
}

fn staff_input_requests_start(input: &str) -> bool {
    let normalized = input.to_ascii_lowercase();
    normalized.contains("执行")
        || normalized.contains("开始")
        || normalized.contains("start")
        || normalized.contains("run")
}

fn staff_input_task_id(
    app: &crate::app::App,
    input: &str,
    member_id: &str,
) -> Result<String, String> {
    let mut task_ids = app
        .task_board_tasks
        .iter()
        .filter(|task| !task.id.is_empty() && input.contains(&task.id))
        .map(|task| task.id.clone())
        .collect::<Vec<_>>();
    task_ids.sort();
    task_ids.dedup();
    if task_ids.len() == 1 {
        return Ok(task_ids.remove(0));
    }
    if task_ids.len() > 1 {
        return Err("员工命令里匹配到多个任务，请保留一个任务 ID".to_string());
    }

    let mut candidates = app
        .task_board_staff_projection
        .lanes
        .iter()
        .flat_map(|lane| lane.assignments.iter())
        .filter(|assignment| {
            assignment.member_id.as_deref() == Some(member_id) && assignment.actions.can_start
        })
        .map(|assignment| assignment.task_id.clone())
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.dedup();
    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Err("这个员工当前没有可执行任务".to_string()),
        _ => Err("这个员工有多个可执行任务，请补任务 ID".to_string()),
    }
}

fn find_staff_assignment<'a>(
    projection: &'a crate::app::task::staff::TaskBoardStaffProjection,
    task_id: &str,
) -> Option<&'a crate::app::task::staff::TaskBoardStaffAssignment> {
    projection
        .lanes
        .iter()
        .flat_map(|lane| lane.assignments.iter())
        .chain(projection.unassigned.iter())
        .find(|assignment| assignment.task_id == task_id)
}

fn schedule_task_pipeline_for_status(
    app: &mut crate::app::App,
    status: TaskStatus,
) -> iced::Task<crate::app::Message> {
    let workflow = workflow_for_task_board_settings(app);
    match workflow.as_ref().and_then(|workflow| workflow.status_role(status)) {
        Some(StatusRole::ReviewReady | StatusRole::Review) => {
            schedule_auto_review_tick_with_deadline(app)
        }
        Some(StatusRole::Merging) => schedule_auto_merge_tick_with_deadline(app),
        _ => iced::Task::none(),
    }
}

fn workflow_for_task_board_settings(app: &crate::app::App) -> Option<TaskWorkflow> {
    task_workflow_by_id(&current_staff_workflow_id(app))
}

fn apply_stage_workflow_completed(
    app: &mut crate::app::App,
    project_path: String,
    task_id: String,
    status: TaskStatus,
    workflow_uuid: String,
    result: Result<vw_gateway_client::GatewayWorkflowRunOutcome, String>,
) -> iced::Task<crate::app::Message> {
    if !stage_workflow_project_matches(app, &project_path) {
        return iced::Task::none();
    }

    let Some(workflow) = workflow_for_task_board_settings(app) else {
        return iced::Task::none();
    };
    let mut matrix_status_change = None;

    if let Some(task) = app.task_board_tasks.iter_mut().find(|task| task.id == task_id) {
        if task.status != status {
            task.add_log(format!(
                "任务状态已变化，忽略阶段 workflow {} 结果",
                truncate_for_ui(&workflow_uuid, TASK_LOG_UI_MAX_ENTRY_CHARS)
            ));
        } else {
            let from_status = task.status;
            match result {
                Ok(outcome) => {
                    let summary = truncate_for_ui(&outcome.summary, TASK_LOG_UI_MAX_DETAIL_CHARS);
                    task.add_log(format!("阶段 workflow 完成: {summary}"));
                    if let Some(run_id) = outcome.workflow_run_id.as_deref()
                        && !run_id.trim().is_empty()
                    {
                        task.add_log(format!(
                            "阶段 workflow 运行 ID: {}",
                            truncate_for_ui(run_id, TASK_LOG_UI_MAX_ENTRY_CHARS)
                        ));
                    }
                    advance_stage_workflow_success(task, &workflow, status);
                }
                Err(error) => {
                    let detail = truncate_for_ui(&error, TASK_LOG_UI_MAX_DETAIL_CHARS);
                    task.add_log(format!("阶段 workflow 执行失败: {detail}"));
                    advance_stage_workflow_failure(task, &workflow, status, detail);
                }
            }
            if task.status != from_status {
                matrix_status_change = Some((task.clone(), from_status));
            }
        }
    }

    sync_viewing_logs(app, &task_id);
    let publish_task = matrix_status_change
        .as_ref()
        .map(|(task, from_status)| {
            publish_task_pool_matrix_status_event(app, task, Some(*from_status))
        })
        .unwrap_or_else(iced::Task::none);
    publish_task
}

fn stage_workflow_project_matches(app: &crate::app::App, project_path: &str) -> bool {
    if project_path.is_empty() {
        return app.project_path.is_none();
    }
    task_board_project_matches(app, project_path)
}

fn advance_stage_workflow_success(task: &mut Task, workflow: &TaskWorkflow, status: TaskStatus) {
    let Some(target) = workflow.next_status_for_outcome(status, StatusOutcome::succeed()) else {
        mark_task_paused_for_workflow(
            task,
            workflow,
            "阶段 workflow 成功但当前工作流没有成功后续状态".to_string(),
        );
        return;
    };
    let intent = stage_workflow_success_intent(workflow.status_role(status));
    apply_stage_workflow_transition(
        task,
        workflow,
        target,
        intent,
        "阶段 workflow 成功但状态机拒绝推进",
    );
}

fn advance_stage_workflow_failure(
    task: &mut Task,
    workflow: &TaskWorkflow,
    status: TaskStatus,
    detail: String,
) {
    let Some(target) = workflow.next_status_for_outcome(status, StatusOutcome::failed()) else {
        mark_task_paused_for_workflow(task, workflow, format!("阶段 workflow 执行失败: {detail}"));
        return;
    };
    let intent = stage_workflow_failure_intent(workflow.status_role(status));
    apply_stage_workflow_transition(
        task,
        workflow,
        target,
        intent,
        "阶段 workflow 失败但状态机拒绝推进",
    );
}

fn apply_stage_workflow_transition(
    task: &mut Task,
    workflow: &TaskWorkflow,
    target: TaskStatus,
    intent: TaskTransitionIntent,
    rejection_context: &str,
) {
    let machine = TaskStateMachine::new(workflow.clone());
    if let Err(error) = machine.validate_transition(task.status, target, intent) {
        mark_task_paused_for_workflow(task, workflow, format!("{rejection_context}: {error}"));
        return;
    }
    task.set_status(target);
}

fn stage_workflow_success_intent(role: Option<StatusRole>) -> TaskTransitionIntent {
    match role {
        Some(StatusRole::ReviewReady) => TaskTransitionIntent::EnterReview,
        Some(StatusRole::Review) => TaskTransitionIntent::ReviewApproved,
        Some(StatusRole::Merging) => TaskTransitionIntent::MergeSucceeded,
        Some(StatusRole::Backlog) => TaskTransitionIntent::AutoPromote,
        Some(StatusRole::Queued | StatusRole::Planning | StatusRole::Failed) => {
            TaskTransitionIntent::StartExecution
        }
        _ => TaskTransitionIntent::ExecutionSucceeded,
    }
}

fn stage_workflow_failure_intent(role: Option<StatusRole>) -> TaskTransitionIntent {
    match role {
        Some(StatusRole::Review) => TaskTransitionIntent::ReviewBlocked,
        Some(StatusRole::Merging) => TaskTransitionIntent::MergeFailed,
        _ => TaskTransitionIntent::ExecutionFailed,
    }
}

fn mark_task_paused_for_workflow(task: &mut Task, workflow: &TaskWorkflow, reason: String) {
    if let Some(started_at) = task.execution_started_at_ms {
        task.last_execution_duration_ms = Some(now_ms().saturating_sub(started_at));
    }
    task.execution_started_at_ms = None;
    let paused_status = workflow
        .statuses()
        .iter()
        .find(|definition| definition.role() == StatusRole::Paused)
        .map(|definition| definition.status)
        .unwrap_or(TaskStatus::Paused);
    task.set_status(paused_status);
    task.pause_reason = Some(reason.clone());
    task.add_log(format!("暂停原因: {reason}"));
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
) -> Vec<Task> {
    let mut merged = loaded_tasks
        .into_iter()
        .map(|loaded_task| {
            if running_task_ids.iter().any(|id| id == &loaded_task.id)
                && let Some(current_task) =
                    current_tasks.iter().find(|task| task.id == loaded_task.id)
            {
                return current_task.clone();
            }
            loaded_task
        })
        .collect::<Vec<_>>();

    for current_task in current_tasks {
        if running_task_ids.iter().any(|id| id == &current_task.id)
            && !merged.iter().any(|task| task.id == current_task.id)
        {
            merged.push(current_task.clone());
        }
    }

    merged
}

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
            let matrix_poll = poll_task_pool_matrix_events(app);

            if let Some(project_path) = &app.project_path {
                let path: String = project_path.clone();
                return iced::Task::batch(vec![
                    maybe_schedule_worktree_snapshot_refresh(app, true),
                    matrix_poll,
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
                let staff_refresh = if app.task_board_view_mode == TaskBoardViewMode::Staff {
                    app.task_board_staff_config_load_error = None;
                    load_gateway_staff_projection_task(app, current_staff_workflow_id(app))
                } else {
                    iced::Task::none()
                };
                let capability_refresh = if app.task_board_view_mode == TaskBoardViewMode::Staff {
                    start_staff_capability_catalog_load(app)
                } else {
                    iced::Task::none()
                };
                return iced::Task::batch(vec![
                    refresh_snapshot,
                    staff_refresh,
                    capability_refresh,
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
            );
            refresh_staff_projection(app);
            sync_task_log_cache_for_loaded_tasks(app);
            app.task_board_loading = false;
            prune_bulk_selection(app);
            app.task_board_settings =
                sanitized_task_board_settings(app.task_board_settings.clone());
            iced::Task::none()
        }
        TaskBoardMessage::ToggleStaffMode(enabled) => {
            app.task_board_view_mode =
                if enabled { TaskBoardViewMode::Staff } else { TaskBoardViewMode::Kanban };

            if !enabled {
                return iced::Task::none();
            }

            app.task_board_staff_config_load_error = None;
            refresh_staff_projection(app);

            if app.project_path.is_some() {
                return iced::Task::batch(vec![
                    load_gateway_staff_projection_task(app, current_staff_workflow_id(app)),
                    start_staff_capability_catalog_load(app),
                ]);
            }

            iced::Task::none()
        }
        TaskBoardMessage::StaffConfigLoaded(result) => {
            let workflow_id = current_staff_workflow_id(app);
            apply_staff_config_loaded(app, &workflow_id, result);
            iced::Task::none()
        }
        TaskBoardMessage::StaffConfigLoadedForWorkflow { workflow_id, result } => {
            apply_staff_config_loaded(app, &workflow_id, result);
            iced::Task::none()
        }
        TaskBoardMessage::LoadStaffCapabilityCatalog => {
            let workflow_id = current_staff_workflow_id(app);
            start_staff_capability_catalog_load_for_workflow(app, workflow_id)
        }
        TaskBoardMessage::LoadStaffCapabilityCatalogForWorkflow { workflow_id } => {
            start_staff_capability_catalog_load_for_workflow(app, workflow_id)
        }
        TaskBoardMessage::StaffCapabilityCatalogLoaded(result) => match result {
            Ok(capabilities) => {
                app.task_board_staff_capability_catalog_loading = false;
                app.task_board_staff_capability_catalog_error = None;
                app.task_board_staff_capability_catalog_workflow_id =
                    Some(current_staff_workflow_id(app));
                app.task_board_staff_capability_catalog = capabilities.clone();
                app.task_board_staff_capability_draft = capabilities;
                iced::Task::none()
            }
            Err(error) => {
                app.task_board_staff_capability_catalog_loading = false;
                app.task_board_staff_capability_catalog_workflow_id =
                    Some(current_staff_workflow_id(app));
                app.task_board_staff_capability_catalog_error = Some(error.clone());
                app.show_error_toast(format!("能力目录加载失败：{error}"))
            }
        },
        TaskBoardMessage::StaffCapabilityCatalogLoadedForWorkflow { workflow_id, result } => {
            match result {
                Ok(capabilities) => {
                    app.task_board_staff_capability_catalog_loading = false;
                    app.task_board_staff_capability_catalog_error = None;
                    app.task_board_staff_capability_catalog_workflow_id = Some(workflow_id);
                    app.task_board_staff_capability_catalog = capabilities.clone();
                    app.task_board_staff_capability_draft = capabilities;
                    iced::Task::none()
                }
                Err(error) => {
                    app.task_board_staff_capability_catalog_loading = false;
                    app.task_board_staff_capability_catalog_workflow_id = Some(workflow_id);
                    app.task_board_staff_capability_catalog_error = Some(error.clone());
                    app.show_error_toast(format!("能力目录加载失败：{error}"))
                }
            }
        }
        TaskBoardMessage::UpdateStaffCapabilityDraft { capability_id, field, value } => {
            update_staff_capability_draft_field(
                &mut app.task_board_staff_capability_draft,
                &capability_id,
                field,
                value,
            );
            iced::Task::none()
        }
        TaskBoardMessage::UpdateStaffCapabilitySummaryDraft {
            capability_id,
            summary,
            description,
            fields,
        } => {
            update_staff_capability_summary_draft(
                &mut app.task_board_staff_capability_draft,
                &capability_id,
                summary,
                description,
                fields,
            );
            iced::Task::none()
        }
        TaskBoardMessage::AddStaffCapabilityDraft => {
            let id = next_custom_staff_capability_id(&app.task_board_staff_capability_draft);
            let label = format!("自定义能力 {}", id.trim_start_matches("custom_capability_"));
            app.task_board_staff_capability_draft
                .push(TaskBoardStaffCapability::custom_saved_workflow(id, label));
            iced::Task::none()
        }
        TaskBoardMessage::DeleteStaffCapabilityDraft { capability_id } => {
            app.task_board_staff_capability_draft
                .retain(|capability| capability.built_in || capability.id != capability_id);
            iced::Task::none()
        }
        TaskBoardMessage::SaveStaffCapabilityCatalog => {
            app.task_board_staff_capability_saving = true;
            app.task_board_staff_capability_catalog_error = None;
            let workflow_id = current_staff_workflow_id(app);
            save_staff_capability_catalog_task(
                app,
                workflow_id,
                app.task_board_staff_capability_draft.clone(),
            )
        }
        TaskBoardMessage::SaveStaffCapabilityCatalogForWorkflow { workflow_id } => {
            app.task_board_staff_capability_saving = true;
            app.task_board_staff_capability_catalog_error = None;
            save_staff_capability_catalog_task(
                app,
                workflow_id,
                app.task_board_staff_capability_draft.clone(),
            )
        }
        TaskBoardMessage::StaffCapabilityCatalogSaved(result) => {
            app.task_board_staff_capability_saving = false;
            match result {
                Ok(capabilities) => {
                    app.task_board_staff_capability_catalog_error = None;
                    app.task_board_staff_capability_catalog_workflow_id =
                        Some(current_staff_workflow_id(app));
                    app.task_board_staff_capability_catalog = capabilities.clone();
                    app.task_board_staff_capability_draft = capabilities;
                    let mut tasks = vec![app.show_success_toast("能力目录已保存")];
                    tasks.push(load_gateway_staff_projection_task(
                        app,
                        current_staff_workflow_id(app),
                    ));
                    iced::Task::batch(tasks)
                }
                Err(error) => {
                    app.task_board_staff_capability_catalog_workflow_id =
                        Some(current_staff_workflow_id(app));
                    app.task_board_staff_capability_catalog_error = Some(error.clone());
                    app.show_error_toast(format!("能力目录保存失败：{error}"))
                }
            }
        }
        TaskBoardMessage::StaffCapabilityCatalogSavedForWorkflow { workflow_id, result } => {
            app.task_board_staff_capability_saving = false;
            match result {
                Ok(capabilities) => {
                    app.task_board_staff_capability_catalog_error = None;
                    app.task_board_staff_capability_catalog_workflow_id = Some(workflow_id.clone());
                    app.task_board_staff_capability_catalog = capabilities.clone();
                    app.task_board_staff_capability_draft = capabilities;
                    let mut tasks = vec![app.show_success_toast("能力目录已保存")];
                    tasks.push(load_gateway_staff_projection_task(app, workflow_id));
                    iced::Task::batch(tasks)
                }
                Err(error) => {
                    app.task_board_staff_capability_catalog_workflow_id = Some(workflow_id);
                    app.task_board_staff_capability_catalog_error = Some(error.clone());
                    app.show_error_toast(format!("能力目录保存失败：{error}"))
                }
            }
        }
        TaskBoardMessage::RefreshStaffProjection => {
            refresh_staff_projection(app);
            iced::Task::none()
        }
        TaskBoardMessage::ReloadStaffConfig => {
            app.task_board_staff_config_load_error = None;
            let mut tasks = vec![start_staff_capability_catalog_load(app)];
            if app.project_path.is_some() {
                tasks.push(load_gateway_staff_projection_task(app, current_staff_workflow_id(app)));
            }
            iced::Task::batch(tasks)
        }
        TaskBoardMessage::ReloadStaffConfigForWorkflow { workflow_id } => {
            app.task_board_staff_config_load_error = None;
            let mut tasks =
                vec![start_staff_capability_catalog_load_for_workflow(app, workflow_id.clone())];
            tasks.push(load_gateway_staff_projection_task(app, workflow_id));
            iced::Task::batch(tasks)
        }
        TaskBoardMessage::StaffAutoSchedulePlanLoaded { result } => {
            let response = match result {
                Ok(response) => response,
                Err(error) => {
                    app.task_board_staff_config_load_error = Some(error.clone());
                    return app.show_error_toast(format!("自主调度已阻塞：{error}"));
                }
            };
            let timestamp_ms = now_ms();
            for command_context in response.plan.command_contexts {
                app.task_board_staff_command_queue.remember_orchestration_context(
                    &command_context.command_id,
                    staff_orchestration_context_from_gateway(command_context.orchestration),
                );
            }
            for skip in response.plan.skipped {
                let status = TaskStatus::parse_key(&skip.status_key).unwrap_or(TaskStatus::Pool);
                record_staff_event(
                    app,
                    crate::app::task::staff::TaskBoardStaffEvent::command_event(
                        format!("staff-auto-skip-{}-{timestamp_ms}", skip.task_id),
                        Some(skip.task_id.clone()),
                        None,
                        TaskBoardStaffCommandStatus::Skipped,
                        timestamp_ms,
                        format!(
                            "自主调度跳过任务 {}（{}）：{}",
                            skip.task_id,
                            status.label(),
                            skip.reason
                        ),
                        skip.orchestration.map(staff_orchestration_context_from_gateway),
                    ),
                );
            }
            iced::Task::batch(response.plan.commands.into_iter().map(|command| {
                iced::Task::done(Message::TaskBoard(TaskBoardMessage::SubmitStaffCommandSilently(
                    staff_command_from_gateway(command),
                )))
            }))
        }
        TaskBoardMessage::SubmitStaffCommand(command) => submit_staff_command(app, command, true),
        TaskBoardMessage::SubmitStaffCommandSilently(command) => {
            submit_staff_command(app, command, false)
        }
        TaskBoardMessage::StaffCommandQueueTick => process_staff_command_queue(app),
        TaskBoardMessage::StaffCommandGatewayCompleted { command, result } => {
            apply_staff_command_gateway_completed(app, command, result)
        }
        TaskBoardMessage::SelectStaffFeedTab(tab) => {
            app.task_board_staff_feed_tab = tab;
            iced::Task::none()
        }
        TaskBoardMessage::SelectAllStaffTaskStatuses => {
            app.task_board_staff_task_status_filter.clear();
            iced::Task::none()
        }
        TaskBoardMessage::ToggleStaffTaskStatusFilter(status) => {
            toggle_staff_task_status_filter(&mut app.task_board_staff_task_status_filter, status);
            iced::Task::none()
        }
        TaskBoardMessage::UpdateStaffCommandInput(value) => {
            app.task_board_staff_command_input = value;
            iced::Task::none()
        }
        TaskBoardMessage::SelectStaffCommandMember { member_id } => {
            select_staff_command_member(app, member_id)
        }
        TaskBoardMessage::SubmitStaffCommandInput => submit_staff_command_input(app),
        TaskBoardMessage::OpenMatrixTaskPoolConfig => open_matrix_task_pool_config(app),
        TaskBoardMessage::OpenStaffStageAssignment => {
            app.task_board_staff_assignment_open = true;
            app.task_board_staff_assignment_draft =
                Some(app.task_board_staff_config.stage_assignments.clone());
            let stage_actions = stage_action_draft_from_config(&app.task_board_staff_config);
            app.task_board_staff_workflow_binding_draft =
                Some(stage_workflow_bindings_for_stage_actions(&stage_actions));
            app.task_board_staff_stage_action_draft = Some(stage_actions);
            app.task_board_staff_stage_action_reset_confirm = false;
            app.task_board_staff_workflow_options_error = None;
            iced::Task::batch(vec![
                load_staff_stage_workflow_options_task(),
                start_staff_capability_catalog_load(app),
            ])
        }
        TaskBoardMessage::CloseStaffStageAssignment => {
            app.task_board_staff_assignment_open = false;
            app.task_board_staff_assignment_draft = None;
            app.task_board_staff_stage_action_draft = None;
            app.task_board_staff_workflow_binding_draft = None;
            app.task_board_staff_stage_action_reset_confirm = false;
            iced::Task::none()
        }
        TaskBoardMessage::SelectStaffStageAssignmentTab { status, tab } => {
            app.task_board_staff_assignment_tabs.insert(status, tab);
            iced::Task::none()
        }
        TaskBoardMessage::SelectStaffOrchestrationTab(tab) => {
            app.task_board_staff_orchestration_tab = tab;
            iced::Task::none()
        }
        TaskBoardMessage::SelectStaffStagePrimaryMember { status, member_id } => {
            let assignment = ensure_stage_assignment(staff_stage_assignment_draft_mut(app), status);
            assignment.primary_member_id = normalize_optional_member_id(member_id);
            if let Some(member_id) = assignment.primary_member_id.as_deref() {
                remove_member_id(&mut assignment.backup_member_ids, member_id);
                remove_member_id(&mut assignment.manager_member_ids, member_id);
            }
            iced::Task::none()
        }
        TaskBoardMessage::SelectStaffStageActionCapability { status, capability_id } => {
            let requires_workflow_uuid = capability_id
                .as_deref()
                .map(str::trim)
                .filter(|capability_id| !capability_id.is_empty())
                .is_some_and(|capability_id| {
                    stage_capability_requires_workflow_uuid(app, capability_id)
                });
            set_stage_action_capability(
                staff_stage_action_draft_mut(app),
                status,
                capability_id,
                requires_workflow_uuid,
            );
            app.task_board_staff_stage_action_reset_confirm = false;
            refresh_stage_workflow_binding_draft_from_action_draft(app);
            iced::Task::none()
        }
        TaskBoardMessage::SelectStaffStageActionWorkflow { status, workflow_uuid } => {
            set_stage_action_workflow(staff_stage_action_draft_mut(app), status, workflow_uuid);
            app.task_board_staff_stage_action_reset_confirm = false;
            refresh_stage_workflow_binding_draft_from_action_draft(app);
            iced::Task::none()
        }
        TaskBoardMessage::CreateStaffStageActionWorkflow { status, workflow_id } => {
            match build_stage_action_workflow_spec(app, status, &workflow_id) {
                Ok(spec) => create_staff_stage_action_workflow_task(status, spec),
                Err(error) => app.show_error_toast(format!("Workflow 创建失败：{error}")),
            }
        }
        TaskBoardMessage::StaffStageActionWorkflowCreated { status, result } => match result {
            Ok(summary) => {
                upsert_staff_stage_workflow_option(
                    &mut app.task_board_staff_workflow_options,
                    summary.clone(),
                );
                app.task_board_staff_workflow_options_error = None;
                set_stage_action_workflow(
                    staff_stage_action_draft_mut(app),
                    status,
                    Some(summary.uuid),
                );
                app.task_board_staff_stage_action_reset_confirm = false;
                refresh_stage_workflow_binding_draft_from_action_draft(app);
                app.show_success_toast("Workflow 已创建并绑定")
            }
            Err(error) => app.show_error_toast(format!("Workflow 创建失败：{error}")),
        },
        TaskBoardMessage::UpdateStaffStageActionParameter { status, key, value } => {
            set_stage_action_parameter(staff_stage_action_draft_mut(app), status, key, value);
            app.task_board_staff_stage_action_reset_confirm = false;
            iced::Task::none()
        }
        TaskBoardMessage::RequestResetStaffStageActions => {
            app.task_board_staff_stage_action_reset_confirm = true;
            app.show_warning_toast("将覆盖当前阶段能力和自动接续草稿，请再次点击确认重置")
        }
        TaskBoardMessage::ConfirmResetStaffStageActions => {
            app.task_board_staff_stage_action_reset_confirm = false;
            let Some(workflow) = workflow_for_task_board_settings(app) else {
                return app.show_error_toast("阶段能力重置失败：任务工作流未加载");
            };
            app.task_board_staff_assignment_draft = Some(default_stage_assignments_for_workflow(
                &workflow,
                &app.task_board_staff_config.members,
            ));
            app.task_board_staff_stage_action_draft =
                Some(default_stage_actions_for_workflow(&workflow));
            refresh_stage_workflow_binding_draft_from_action_draft(app);
            app.show_success_toast("阶段能力与自动接续已重置为当前生命周期默认配置")
        }
        TaskBoardMessage::CancelResetStaffStageActions => {
            app.task_board_staff_stage_action_reset_confirm = false;
            iced::Task::none()
        }
        TaskBoardMessage::SelectStaffStageWorkflowBinding { status, workflow_uuid } => {
            if workflow_uuid.as_deref().map(str::trim).filter(|value| !value.is_empty()).is_some() {
                set_stage_action_workflow(staff_stage_action_draft_mut(app), status, workflow_uuid);
            } else {
                set_stage_action_capability(staff_stage_action_draft_mut(app), status, None, false);
            }
            app.task_board_staff_stage_action_reset_confirm = false;
            refresh_stage_workflow_binding_draft_from_action_draft(app);
            iced::Task::none()
        }
        TaskBoardMessage::LoadStaffStageWorkflowOptions => {
            app.task_board_staff_workflow_options_error = None;
            load_staff_stage_workflow_options_task()
        }
        TaskBoardMessage::StaffStageWorkflowOptionsLoaded(result) => match result {
            Ok(mut options) => {
                options.sort_by(|left, right| {
                    left.name.cmp(&right.name).then_with(|| left.uuid.cmp(&right.uuid))
                });
                app.task_board_staff_workflow_options = options;
                app.task_board_staff_workflow_options_error = None;
                iced::Task::none()
            }
            Err(error) => {
                app.task_board_staff_workflow_options_error = Some(error.clone());
                app.show_error_toast(format!("Workflow 列表加载失败：{error}"))
            }
        },
        TaskBoardMessage::ToggleStaffStageBackupMember { status, member_id } => {
            let assignment = ensure_stage_assignment(staff_stage_assignment_draft_mut(app), status);
            if toggle_member_id(&mut assignment.backup_member_ids, &member_id) {
                if assignment.primary_member_id.as_deref() == Some(member_id.trim()) {
                    assignment.primary_member_id = None;
                }
                remove_member_id(&mut assignment.manager_member_ids, member_id.trim());
            }
            iced::Task::none()
        }
        TaskBoardMessage::ToggleStaffStageManagerMember { status, member_id } => {
            let assignment = ensure_stage_assignment(staff_stage_assignment_draft_mut(app), status);
            if toggle_member_id(&mut assignment.manager_member_ids, &member_id) {
                if assignment.primary_member_id.as_deref() == Some(member_id.trim()) {
                    assignment.primary_member_id = None;
                }
                remove_member_id(&mut assignment.backup_member_ids, member_id.trim());
            }
            iced::Task::none()
        }
        TaskBoardMessage::ToggleStaffStageAutonomous { status, enabled } => {
            let assignment = ensure_stage_assignment(staff_stage_assignment_draft_mut(app), status);
            assignment.autonomous = enabled;
            app.task_board_staff_stage_action_reset_confirm = false;
            iced::Task::none()
        }
        TaskBoardMessage::SaveStaffStageAssignment => {
            app.task_board_staff_stage_action_reset_confirm = false;
            save_staff_stage_assignment(app, None)
        }
        TaskBoardMessage::SaveStaffStageAssignmentForWorkflow { workflow_id } => {
            app.task_board_staff_stage_action_reset_confirm = false;
            save_staff_stage_assignment(app, Some(workflow_id))
        }
        TaskBoardMessage::StaffStageAssignmentPersisted(result) => match result {
            Ok(()) => run_explicit_staff_auto_schedule(app),
            Err(error) => app.show_error_toast(format!("阶段分配持久化失败：{error}")),
        },
        TaskBoardMessage::StaffTaskAssignmentPersisted(result) => match result {
            Ok(()) => iced::Task::none(),
            Err(error) => app.show_error_toast(format!("员工任务分配持久化失败：{error}")),
        },
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
            let from_status = app
                .task_board_tasks
                .iter()
                .find(|existing| existing.id == task.id)
                .map(|existing| existing.status)
                .filter(|status| *status != task.status);
            let publish_task = publish_task_pool_matrix_status_event(app, &task, from_status);
            if let Some(existing) = app.task_board_tasks.iter_mut().find(|t| t.id == task.id) {
                *existing = task.clone();
            }
            refresh_staff_projection(app);
            prune_bulk_selection(app);
            iced::Task::batch(vec![
                publish_task,
                schedule_task_pipeline_for_status(app, task.status),
            ])
        }
        TaskBoardMessage::TaskPoolMatrixEventPublished { task_id, result } => {
            if let Err(error) = result {
                tracing::warn!(
                    target: "vw_desktop",
                    task_id = %task_id,
                    error = %error,
                    "failed to publish task board Matrix event"
                );
                return app.show_warning_toast(format!("Matrix 任务事件发布失败: {error}"));
            }
            iced::Task::none()
        }
        TaskBoardMessage::StaffHandoffIpcDeliveryCompleted {
            task_id,
            from_member_id,
            to_member_id,
            to_agent,
            timestamp_ms,
            result,
        } => staff_messages_update::apply_staff_handoff_ipc_delivery_result(
            app,
            task_id,
            from_member_id,
            to_member_id,
            to_agent,
            timestamp_ms,
            result,
        ),
        TaskBoardMessage::PollMatrixTaskPoolEvents => poll_task_pool_matrix_events(app),
        TaskBoardMessage::MatrixTaskPoolEventsLoaded(result) => {
            match result {
                Ok(response) => {
                    if apply_matrix_task_pool_events(app, response) {
                        return sync_agent_scheduler_status_for_current_project(app);
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        target: "vw_desktop",
                        error = %error,
                        "failed to poll task board Matrix events"
                    );
                }
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
            let publish_task = publish_task_pool_matrix_status_event(app, &task, None);
            app.task_board_tasks.push(task);
            prune_bulk_selection(app);
            app.task_board_create_submit_success = true;
            let mut post_create_tasks = vec![publish_task];
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
            app.task_board_last_model = normalize_task_model(&app.task_board_draft.model);
            app.task_board_model_popover = false;
            iced::Task::none()
        }
        TaskBoardMessage::UpdateEditingTaskModelInput(model) => {
            app.task_board_draft.model = model;
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
            refresh_staff_projection(app);
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
        TaskBoardMessage::TaskStageWorkflowCompleted {
            project_path,
            task_id,
            status,
            workflow_uuid,
            result,
        } => apply_stage_workflow_completed(
            app,
            project_path,
            task_id,
            status,
            workflow_uuid,
            result,
        ),
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
            let mut matrix_status_change: Option<(Task, TaskStatus)> = None;
            let mut execute_task_to_start: Option<Task> = None;
            let mut plan_failed = false;
            let staff_config = app.task_board_staff_config.clone();

            if let Some(task) = app.task_board_tasks.iter_mut().find(|t| t.id == task_id) {
                for log in final_logs {
                    let _ = append_task_log_stream(task, &log);
                }
                if task.status != TaskStatus::Planning {
                    task.add_log("任务状态已变化，忽略本次拆分结果".to_string());
                } else {
                    let from_status = task.status;
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
                                    if let Some(defaults) =
                                        apply_staff_execution_defaults(&staff_config, task)
                                    {
                                        task.add_log(staff_execution_defaults_log(&defaults));
                                    }
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
                    if task.status != from_status {
                        matrix_status_change = Some((task.clone(), from_status));
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
            let publish_task = matrix_status_change
                .as_ref()
                .map(|(task, from_status)| {
                    publish_task_pool_matrix_status_event(app, task, Some(*from_status))
                })
                .unwrap_or_else(iced::Task::none);
            if plan_failed {
                let pipeline_task = task_to_persist
                    .as_ref()
                    .map(|task| schedule_task_pipeline_for_status(app, task.status))
                    .unwrap_or_else(iced::Task::none);
                return iced::Task::batch(vec![
                    persist_task,
                    publish_task,
                    schedule_scheduler_tick_with_deadline(app),
                    pipeline_task,
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
#[cfg(test)]
#[path = "update_tests.rs"]
mod update_tests;

#[cfg(test)]
#[path = "staff_tests.rs"]
mod staff_tests;

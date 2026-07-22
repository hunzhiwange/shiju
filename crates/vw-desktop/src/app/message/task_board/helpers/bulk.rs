//! 提供任务看板消息处理过程中复用的局部辅助逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;

/// 执行 visible_task_ids_for_status 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn visible_task_ids_for_status(
    app: &crate::app::App,
    status: TaskStatus,
) -> Vec<String> {
    app.task_board_tasks
        .iter()
        .filter(|task| task.status == status && !task.deleted && !task.archived)
        .map(|task| task.id.clone())
        .collect()
}

/// 执行 selected_task_ids_for_status 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn selected_task_ids_for_status(
    app: &crate::app::App,
    status: TaskStatus,
) -> Vec<String> {
    visible_task_ids_for_status(app, status)
        .into_iter()
        .filter(|task_id| app.task_board_selected_tasks.contains(task_id))
        .collect()
}

/// 执行 prune_bulk_selection 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn prune_bulk_selection(app: &mut crate::app::App) {
    let valid_ids = app
        .task_board_tasks
        .iter()
        .filter(|task| !task.deleted && !task.archived)
        .map(|task| task.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    app.task_board_selected_tasks.retain(|task_id| valid_ids.contains(task_id.as_str()));
}

/// 执行 clear_selected_task_ids 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn clear_selected_task_ids(app: &mut crate::app::App, task_ids: &[String]) {
    for task_id in task_ids {
        app.task_board_selected_tasks.remove(task_id);
    }
}

/// 执行 clear_selection_for_status 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn clear_selection_for_status(app: &mut crate::app::App, status: TaskStatus) {
    let task_ids = visible_task_ids_for_status(app, status);
    clear_selected_task_ids(app, &task_ids);
}

/// 执行 selected_tasks_for_status 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn selected_tasks_for_status(app: &crate::app::App, status: TaskStatus) -> Vec<Task> {
    app.task_board_tasks
        .iter()
        .filter(|task| {
            task.status == status
                && !task.deleted
                && !task.archived
                && app.task_board_selected_tasks.contains(&task.id)
        })
        .cloned()
        .collect()
}

fn agent_scheduler_command_request(
    task_id: &str,
    command: vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandKindDto,
    reason: impl Into<Option<String>>,
    payload: vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandPayloadDto,
) -> vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandRequest {
    vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandRequest {
        task_id: task_id.to_string().into(),
        command,
        dispatch_mode:
            vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerDispatchModeDto::Local,
        max_concurrent: None,
        reason: reason.into(),
        payload,
    }
}

async fn submit_agent_scheduler_bulk_command(
    project_path: &str,
    commands: Vec<vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandRequest>,
    action_label: &str,
) -> Result<(), String> {
    if commands.is_empty() {
        return Ok(());
    }
    let scope = resolve_task_board_scheduler_scope(project_path).await?;
    let client = crate::app::config::gateway_client()?;
    let response = client
        .agent_scheduler_bulk_command(
            &scope,
            &vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerBulkCommandRequest {
                commands,
                stop_on_error: true,
            },
        )
        .await?;
    if response.rejected == 0 {
        return Ok(());
    }
    Err(format!(
        "{action_label}失败：gateway 拒绝 {} 个命令，已接受 {} 个命令",
        response.rejected, response.accepted
    ))
}

fn task_metadata_patch_for_gateway(
    task: &Task,
    workflow_id: &str,
) -> vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerTaskMetadataPatchDto {
    let status_key = agent_scheduler_status_key_for_task_status(task.status, workflow_id);
    let status = agent_scheduler_status_for_task_status(task.status, workflow_id);
    let status_role = agent_scheduler_status_role_for_task_status(task.status, workflow_id);
    vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerTaskMetadataPatchDto {
        title: Some(task.prompt.lines().next().unwrap_or(&task.id).chars().take(120).collect()),
        status_key: Some(status_key),
        status: Some(status),
        priority: Some(task.priority),
        order_no: Some(i64::from(task.order)),
        model: Some(task.model.clone()),
        agent: task.agent.clone(),
        acp_agent: task.acp_agent.clone(),
        reasoning_effort: task.reasoning_effort,
        prompt: Some(task.prompt.clone()),
        last_error: task.last_error.clone(),
        pause_reason: task.pause_reason.clone(),
        merge_source_branch: task.merge_source_branch.clone(),
        merge_target_branch: task.merge_target_branch.clone(),
        selected_worktree_path: task.selected_worktree_path.clone(),
        selected_worktree_slot_id: task.selected_worktree_slot_id.clone(),
        selected_worktree_branch: task.selected_worktree_branch.clone(),
        workflow_id: Some(if workflow_id.trim().is_empty() {
            crate::app::task::DEFAULT_TASK_WORKFLOW_ID.to_string()
        } else {
            workflow_id.trim().to_string()
        }),
        status_role,
        metadata_merge: Some(serde_json::json!({
            "assignee": &task.assignee,
            "description": &task.description,
            "subtasks": &task.subtasks,
            "auto_promote_delay_ms": task.auto_promote_delay_ms,
            "task_split_mode": task.task_split_mode.to_string_key(),
        })),
        metadata_remove_keys: Vec::new(),
    }
}

pub(crate) async fn persist_updated_tasks_via_gateway(
    project_path: &str,
    tasks: &[Task],
    action_label: &str,
    workflow_id: &str,
) -> Result<(), String> {
    let commands = tasks
        .iter()
        .map(|task| {
            agent_scheduler_command_request(
                &task.id,
                vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandKindDto::UpdateMetadata,
                Some(action_label.to_string()),
                vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandPayloadDto {
                    metadata_update: Some(task_metadata_patch_for_gateway(task, workflow_id)),
                    ..Default::default()
                },
            )
        })
        .collect::<Vec<_>>();
    submit_agent_scheduler_bulk_command(project_path, commands, action_label).await
}

pub(crate) async fn delete_tasks_via_gateway(
    project_path: &str,
    task_ids: &[String],
) -> Result<(), String> {
    let commands = task_ids
        .iter()
        .map(|task_id| {
            agent_scheduler_command_request(
                task_id,
                vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandKindDto::SoftDelete,
                Some("删除任务".to_string()),
                Default::default(),
            )
        })
        .collect::<Vec<_>>();
    submit_agent_scheduler_bulk_command(project_path, commands, "删除任务").await
}

pub(crate) async fn send_archive_commands_via_gateway(
    project_path: &str,
    task_ids: &[String],
) -> Result<(), String> {
    let commands = task_ids
        .iter()
        .map(|task_id| {
            agent_scheduler_command_request(
                task_id,
                vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandKindDto::Archive,
                Some("归档任务".to_string()),
                Default::default(),
            )
        })
        .collect::<Vec<_>>();
    submit_agent_scheduler_bulk_command(project_path, commands, "归档任务").await
}

pub(crate) async fn move_task_status_via_gateway(
    project_path: &str,
    task_id: &str,
    target_status: TaskStatus,
    workflow_id: &str,
    order_no: Option<i64>,
) -> Result<(), String> {
    let status_key = agent_scheduler_status_key_for_task_status(target_status, workflow_id);
    let status = agent_scheduler_status_for_task_status(target_status, workflow_id);
    let command = agent_scheduler_command_request(
        task_id,
        vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandKindDto::Move,
        Some("手动移动任务".to_string()),
        vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandPayloadDto {
            move_task: Some(
                vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerMoveTaskPayloadDto {
                    from_status_key: None,
                    to_status_key: status_key,
                    to_status: Some(status),
                    order_no,
                },
            ),
            ..Default::default()
        },
    );
    submit_agent_scheduler_bulk_command(project_path, vec![command], "手动移动任务").await
}

pub(crate) async fn move_tasks_to_status_via_gateway(
    project_path: &str,
    task_ids: &[String],
    target_status: TaskStatus,
    workflow_id: &str,
) -> Result<(), String> {
    let status_key = agent_scheduler_status_key_for_task_status(target_status, workflow_id);
    let status = agent_scheduler_status_for_task_status(target_status, workflow_id);
    let commands = task_ids
        .iter()
        .map(|task_id| {
            agent_scheduler_command_request(
                task_id,
                vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandKindDto::Move,
                Some("批量移动任务".to_string()),
                vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandPayloadDto {
                    move_task: Some(
                        vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerMoveTaskPayloadDto {
                            from_status_key: None,
                            to_status_key: status_key.clone(),
                            to_status: Some(status.clone()),
                            order_no: None,
                        },
                    ),
                    ..Default::default()
                },
            )
        })
        .collect::<Vec<_>>();
    submit_agent_scheduler_bulk_command(project_path, commands, "批量移动任务").await
}

/// 执行 normalized_bulk_model_input 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn normalized_bulk_model_input(value: &str) -> String {
    normalize_task_model_input(value)
}

/// 执行 reset_bulk_operation_inputs 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn reset_bulk_operation_inputs(app: &mut crate::app::App) {
    app.task_board_bulk_priority_input =
        parse_priority_or_default(&app.task_board_draft.priority, 999).to_string();
    app.task_board_bulk_model_input = normalized_bulk_model_input(&app.task_board_last_model);
    app.task_board_bulk_agent = app
        .task_board_draft
        .agent
        .clone()
        .unwrap_or_else(|| crate::app::task::TASK_AGENT_MAIN.to_string());
    app.task_board_bulk_acp_agent = app.task_board_last_acp_agent.clone();
    app.task_board_bulk_reasoning_effort = app.task_board_draft.reasoning_effort;
    app.task_board_bulk_task_split_mode = app.task_board_draft.task_split_mode;
}

/// 执行 deactivate_bulk_selection_mode 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn deactivate_bulk_selection_mode(app: &mut crate::app::App) {
    if let Some(active_status) = app.task_board_bulk_active_status.take() {
        clear_selection_for_status(app, active_status);
    }
}

fn manual_merge_running(app: &crate::app::App) -> bool {
    app.task_board_executor.running_tasks.iter().any(|task_id| {
        app.task_board_tasks
            .iter()
            .find(|task| task.id == *task_id)
            .is_some_and(|task| task.status == TaskStatus::PrSubmitted)
    })
}

fn pop_next_manual_merge_task(app: &mut crate::app::App) -> Option<Task> {
    while !app.task_board_manual_merge_queue.is_empty() {
        let task_id = app.task_board_manual_merge_queue.remove(0);
        if let Some(task) = app
            .task_board_tasks
            .iter()
            .find(|task| {
                task.id == task_id
                    && !task.deleted
                    && !task.archived
                    && task.status == TaskStatus::PrSubmitted
                    && !app.task_board_executor.running_tasks.contains(&task.id)
            })
            .cloned()
        {
            return Some(task);
        }
    }
    None
}

pub(crate) fn start_manual_merge_queue_task(
    app: &mut crate::app::App,
) -> iced::Task<crate::app::Message> {
    if manual_merge_running(app) {
        return iced::Task::none();
    }
    let Some(project_path) = app.project_path.clone() else {
        app.task_board_manual_merge_queue.clear();
        app.push_notification("缺少项目路径，无法执行批量合并".to_string());
        return iced::Task::none();
    };
    let Some(task) = pop_next_manual_merge_task(app) else {
        return iced::Task::none();
    };

    let task_id = task.id.clone();
    if let Some(task_in_list) = app.task_board_tasks.iter_mut().find(|t| t.id == task_id) {
        task_in_list.add_log("手动批量合并已发送 Agent Scheduler 合并命令".to_string());
    }
    sync_viewing_logs(app, &task_id);
    iced::Task::batch(vec![
        send_agent_scheduler_command(
            project_path.clone(),
            task,
            app.task_board_settings.workflow_id.clone(),
            vw_gateway_client::vw_api_types::agent_scheduler::AgentSchedulerCommandKindDto::Merge,
            app.task_board_settings.agent_scheduler_dispatch_mode,
            None,
            None,
        ),
        schedule_agent_scheduler_command_in_flight_status_sync(project_path, task_id),
    ])
}
#[cfg(test)]
#[path = "bulk_tests.rs"]
mod bulk_tests;

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
    desktop_task_board::bulk::visible_task_ids_for_status(&app.task_board_tasks, status)
}

/// 执行 selected_task_ids_for_status 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn selected_task_ids_for_status(
    app: &crate::app::App,
    status: TaskStatus,
) -> Vec<String> {
    desktop_task_board::bulk::selected_task_ids_for_status(&app.task_board_tasks, &app.task_board_selected_tasks, status)
}

/// 执行 prune_bulk_selection 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn prune_bulk_selection(app: &mut crate::app::App) {
    desktop_task_board::bulk::prune_bulk_selection(&app.task_board_tasks, &mut app.task_board_selected_tasks)
}

/// 执行 clear_selected_task_ids 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn clear_selected_task_ids(app: &mut crate::app::App, task_ids: &[String]) {
    desktop_task_board::bulk::clear_selected_task_ids(&mut app.task_board_selected_tasks, task_ids)
}

/// 执行 clear_selection_for_status 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn clear_selection_for_status(app: &mut crate::app::App, status: TaskStatus) {
    desktop_task_board::bulk::clear_selection_for_status(&app.task_board_tasks, &mut app.task_board_selected_tasks, status)
}

/// 执行 selected_tasks_for_status 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn selected_tasks_for_status(app: &crate::app::App, status: TaskStatus) -> Vec<Task> {
    desktop_task_board::bulk::selected_tasks_for_status(&app.task_board_tasks, &app.task_board_selected_tasks, status)
}

use desktop_task_board::bulk::agent_scheduler_command_request;

async fn submit_agent_scheduler_bulk_command(
    project_path: &str,
    commands: Vec<vw_api_agent_scheduler::AgentSchedulerCommandRequest>,
    action_label: &str,
) -> Result<(), String> {
    if commands.is_empty() {
        return Ok(());
    }
    let scope = resolve_task_board_scheduler_scope(project_path).await?;
    let client = crate::app::config::gateway_client()?;
    let response = gateway_scheduler::GatewaySchedulerClient::new(client.transport().clone())
        .agent_scheduler_bulk_command(
            &scope,
            &vw_api_agent_scheduler::AgentSchedulerBulkCommandRequest {
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
) -> vw_api_agent_scheduler::AgentSchedulerTaskMetadataPatchDto {
    desktop_task_board::bulk::task_metadata_patch_for_gateway(
        task,
        workflow_id,
        agent_scheduler_status_key_for_task_status(task.status, workflow_id),
        agent_scheduler_status_for_task_status(task.status, workflow_id),
        agent_scheduler_status_role_for_task_status(task.status, workflow_id),
    )
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
                vw_api_agent_scheduler::AgentSchedulerCommandKindDto::UpdateMetadata,
                Some(action_label.to_string()),
                vw_api_agent_scheduler::AgentSchedulerCommandPayloadDto {
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
                vw_api_agent_scheduler::AgentSchedulerCommandKindDto::SoftDelete,
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
                vw_api_agent_scheduler::AgentSchedulerCommandKindDto::Archive,
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
        vw_api_agent_scheduler::AgentSchedulerCommandKindDto::Move,
        Some("手动移动任务".to_string()),
        vw_api_agent_scheduler::AgentSchedulerCommandPayloadDto {
            move_task: Some(
                vw_api_agent_scheduler::AgentSchedulerMoveTaskPayloadDto {
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
                vw_api_agent_scheduler::AgentSchedulerCommandKindDto::Move,
                Some("批量移动任务".to_string()),
                vw_api_agent_scheduler::AgentSchedulerCommandPayloadDto {
                    move_task: Some(
                        vw_api_agent_scheduler::AgentSchedulerMoveTaskPayloadDto {
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
pub(crate) use desktop_task_board::bulk::normalized_bulk_model_input;

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
    desktop_task_board::bulk::manual_merge_running(&app.task_board_tasks, &app.task_board_executor.running_tasks)
}

fn pop_next_manual_merge_task(app: &mut crate::app::App) -> Option<Task> {
    desktop_task_board::bulk::pop_next_manual_merge_task(&app.task_board_tasks, &app.task_board_executor.running_tasks, &mut app.task_board_manual_merge_queue)
}

pub(crate) fn start_manual_merge_queue_task(
    app: &mut crate::app::App,
) -> iced::Task<crate::app::Message> {
    if manual_merge_running(app) {
        return iced::Task::none();
    }
    let Some(project_path) = app.project_path.clone() else {
        app.task_board_manual_merge_queue.clear();
        app.push_localized_notification("缺少项目路径，无法执行批量合并", vec![]);
        return iced::Task::none();
    };
    let Some(task) = pop_next_manual_merge_task(app) else {
        return iced::Task::none();
    };

    let task_id = task.id.clone();
    if let Some(task_in_list) = app.task_board_tasks.iter_mut().find(|t| t.id == task_id) {
        task_in_list.add_localized_log("手动批量合并已发送 Agent Scheduler 合并命令", vec![]);
    }
    sync_viewing_logs(app, &task_id);
    iced::Task::batch(vec![
        send_agent_scheduler_command(
            project_path.clone(),
            task,
            app.task_board_settings.workflow_id.clone(),
            vw_api_agent_scheduler::AgentSchedulerCommandKindDto::Merge,
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

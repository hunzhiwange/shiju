//! 提供任务看板消息处理过程中复用的局部辅助逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;

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

pub(crate) async fn persist_updated_tasks_via_gateway(
    project_path: &str,
    tasks: &[Task],
    action_label: &str,
    workflow_id: &str,
) -> Result<(), String> {
    let commands =
        desktop_task_board::bulk::persist_updated_tasks_commands(tasks, action_label, workflow_id);
    submit_agent_scheduler_bulk_command(project_path, commands, action_label).await
}

pub(crate) async fn delete_tasks_via_gateway(
    project_path: &str,
    task_ids: &[String],
) -> Result<(), String> {
    let commands = desktop_task_board::bulk::delete_tasks_commands(task_ids);
    submit_agent_scheduler_bulk_command(project_path, commands, "删除任务").await
}

pub(crate) async fn send_archive_commands_via_gateway(
    project_path: &str,
    task_ids: &[String],
) -> Result<(), String> {
    let commands = desktop_task_board::bulk::archive_tasks_commands(task_ids);
    submit_agent_scheduler_bulk_command(project_path, commands, "归档任务").await
}

pub(crate) async fn move_task_status_via_gateway(
    project_path: &str,
    task_id: &str,
    target_status: TaskStatus,
    workflow_id: &str,
    order_no: Option<i64>,
) -> Result<(), String> {
    let commands = desktop_task_board::bulk::move_task_status_commands(
        task_id,
        target_status,
        workflow_id,
        order_no,
    );
    submit_agent_scheduler_bulk_command(project_path, commands, "手动移动任务").await
}

pub(crate) async fn move_tasks_to_status_via_gateway(
    project_path: &str,
    task_ids: &[String],
    target_status: TaskStatus,
    workflow_id: &str,
) -> Result<(), String> {
    let commands = desktop_task_board::bulk::move_tasks_to_status_commands(
        task_ids,
        target_status,
        workflow_id,
    );
    submit_agent_scheduler_bulk_command(project_path, commands, "批量移动任务").await
}

pub(crate) fn bulk_state(
    app: &mut crate::app::App,
) -> desktop_task_board::update::bulk::BulkState<'_> {
    desktop_task_board::update::bulk::BulkState {
        task_board_bulk_active_status: &mut app.task_board_bulk_active_status,
        task_board_bulk_acp_agent: &mut app.task_board_bulk_acp_agent,
        task_board_bulk_agent: &mut app.task_board_bulk_agent,
        task_board_bulk_model_input: &mut app.task_board_bulk_model_input,
        task_board_bulk_priority_input: &mut app.task_board_bulk_priority_input,
        task_board_bulk_reasoning_effort: &mut app.task_board_bulk_reasoning_effort,
        task_board_bulk_task_split_mode: &mut app.task_board_bulk_task_split_mode,
        task_board_context_menu: &mut app.task_board_context_menu,
        task_board_draft: &mut app.task_board_draft,
        task_board_editing_task_id: &mut app.task_board_editing_task_id,
        task_board_last_acp_agent: &mut app.task_board_last_acp_agent,
        task_board_last_model: &mut app.task_board_last_model,
        task_board_manual_merge_queue: &mut app.task_board_manual_merge_queue,
        task_board_selected_tasks: &mut app.task_board_selected_tasks,
        task_board_tasks: &mut app.task_board_tasks,
    }
}

pub(crate) fn start_manual_merge_queue_task(
    app: &mut crate::app::App,
) -> iced::Task<crate::app::Message> {
    let effect = desktop_task_board::bulk::prepare_manual_merge(
        &mut app.task_board_tasks,
        &app.task_board_executor.running_tasks,
        &mut app.task_board_manual_merge_queue,
        app.project_path.is_some(),
    );
    let task = match effect {
        desktop_task_board::bulk::ManualMergeEffect::None => return iced::Task::none(),
        desktop_task_board::bulk::ManualMergeEffect::MissingProject => {
            app.push_localized_notification("缺少项目路径，无法执行批量合并", vec![]);
            return iced::Task::none();
        }
        desktop_task_board::bulk::ManualMergeEffect::Dispatch(task) => task,
    };
    let Some(project_path) = app.project_path.clone() else {
        return iced::Task::none();
    };
    let task_id = task.id.clone();
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

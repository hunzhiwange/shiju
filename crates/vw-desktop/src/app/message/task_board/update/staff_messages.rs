//! 员工任务流本地交接消息。

use super::*;

const STAFF_HANDOFF_BODY_MAX_CHARS: usize = 240;
const STAFF_HANDOFF_IPC_FAILURE_LOG_MAX_CHARS: usize = 240;

pub(super) fn maybe_record_task_handoff_message(
    app: &mut crate::app::App,
    task: &Task,
    from_status: TaskStatus,
    to_status: TaskStatus,
) -> iced::Task<crate::app::Message> {
    if from_status == to_status {
        return iced::Task::none();
    }

    if !crate::app::task::TaskStateMachine::default_lifecycle()
        .is_known_transition(from_status, to_status)
    {
        add_handoff_skip_log(
            app,
            &task.id,
            &format!(
                "状态机未认可的交接转移 {} -> {}，跳过本地交接消息",
                from_status.to_string_key(),
                to_status.to_string_key()
            ),
        );
        return iced::Task::none();
    }

    if app.task_board_view_mode != TaskBoardViewMode::Staff {
        add_handoff_skip_log(app, &task.id, "员工模式未加载，跳过本地交接消息");
        return iced::Task::none();
    }

    let config = &app.task_board_staff_config;
    let Some(from_member) = task_handoff_from_member(config, task, from_status) else {
        add_handoff_skip_log(app, &task.id, "未找到当前阶段员工，跳过本地交接消息");
        return iced::Task::none();
    };
    let Some(to_member) = stage_assignment_member(config, to_status) else {
        add_handoff_skip_log(app, &task.id, "未找到下一阶段收件员工，跳过本地交接消息");
        return iced::Task::none();
    };
    let from_member = from_member.clone();
    let to_member = to_member.clone();

    let timestamp_ms = now_ms();
    let body = truncate_handoff_body(format!(
        "任务 {} 交接: {} -> {}; from={}({}); to={}({}); 安全摘要=阶段已完成，可接手下一步。",
        task.id,
        from_status.label(),
        to_status.label(),
        from_member.name,
        from_member.id,
        to_member.name,
        to_member.id
    ));
    let ipc_delivery = build_staff_handoff_ipc_delivery(
        app,
        task,
        from_status,
        to_status,
        &from_member,
        &to_member,
        &body,
        timestamp_ms,
    );
    let event = crate::app::task::staff::TaskBoardStaffEvent::staff_message(
        task.id.clone(),
        from_member.id.clone(),
        to_member.id.clone(),
        body,
        timestamp_ms,
        ipc_delivery
            .as_ref()
            .map(|_| crate::app::task::staff::TaskBoardStaffMessageDeliveryStatus::Pending),
    );
    record_staff_event(app, event);
    ipc_delivery.unwrap_or_else(iced::Task::none)
}

pub(super) fn apply_staff_handoff_ipc_delivery_result(
    app: &mut crate::app::App,
    task_id: String,
    from_member_id: String,
    to_member_id: String,
    to_agent: String,
    timestamp_ms: u64,
    result: Result<(), String>,
) -> iced::Task<crate::app::Message> {
    let delivery_status = if result.is_ok() {
        crate::app::task::staff::TaskBoardStaffMessageDeliveryStatus::Delivered
    } else {
        crate::app::task::staff::TaskBoardStaffMessageDeliveryStatus::Failed
    };

    if let Some(message) =
        staff_handoff_message_mut(app, &task_id, &from_member_id, &to_member_id, timestamp_ms)
    {
        message.delivery_status = Some(delivery_status);
    }

    if let Some(task) = app.task_board_tasks.iter_mut().find(|task| task.id == task_id) {
        match result {
            Ok(()) => {
                task.add_log(format!("IPC 交接消息已投递: to_agent={to_agent}"));
            }
            Err(error) => {
                let detail = truncate_for_ui(&error, STAFF_HANDOFF_IPC_FAILURE_LOG_MAX_CHARS);
                task.add_log(format!("IPC 交接消息投递失败: to_agent={to_agent}; {detail}"));
                tracing::warn!(
                    target: "vw_desktop",
                    task_id = %task_id,
                    to_agent = %to_agent,
                    error = %error,
                    "staff handoff IPC delivery failed"
                );
            }
        }
    }

    iced::Task::none()
}

fn task_handoff_from_member<'a>(
    config: &'a TaskBoardStaffConfig,
    task: &Task,
    from_status: TaskStatus,
) -> Option<&'a TaskBoardStaffMember> {
    find_task_execution_staff_member(config, task)
        .or_else(|| stage_assignment_member(config, from_status))
}

fn stage_assignment_member(
    config: &TaskBoardStaffConfig,
    status: TaskStatus,
) -> Option<&TaskBoardStaffMember> {
    let assignment =
        config.stage_assignments.iter().find(|assignment| assignment.status == status)?;
    assignment
        .primary_member_id
        .as_deref()
        .into_iter()
        .chain(assignment.backup_member_ids.iter().map(String::as_str))
        .chain(assignment.manager_member_ids.iter().map(String::as_str))
        .find_map(|member_id| enabled_member(config, member_id))
}

fn enabled_member<'a>(
    config: &'a TaskBoardStaffConfig,
    member_id: &str,
) -> Option<&'a TaskBoardStaffMember> {
    let member_id = member_id.trim();
    if member_id.is_empty() {
        return None;
    }
    config.members.iter().find(|member| member.enabled && member.id == member_id)
}

fn add_handoff_skip_log(app: &mut crate::app::App, task_id: &str, message: &str) {
    if let Some(task) = app.task_board_tasks.iter_mut().find(|task| task.id == task_id) {
        task.add_log(message.to_string());
    }
}

fn build_staff_handoff_ipc_delivery(
    app: &mut crate::app::App,
    task: &Task,
    from_status: TaskStatus,
    to_status: TaskStatus,
    from_member: &TaskBoardStaffMember,
    to_member: &TaskBoardStaffMember,
    body: &str,
    timestamp_ms: u64,
) -> Option<iced::Task<crate::app::Message>> {
    if !app.agents_ipc_settings.enabled {
        add_handoff_skip_log(app, &task.id, "agents_ipc 未启用，跳过 IPC 交接消息投递");
        return None;
    }

    let Some(to_agent) =
        to_member.ipc_agent_id.as_deref().map(str::trim).filter(|value| !value.is_empty())
    else {
        add_handoff_skip_log(app, &task.id, "收件员工未配置 ipc_agent_id，跳过 IPC 交接消息投递");
        return None;
    };

    let payload = match build_staff_handoff_ipc_payload(
        task,
        from_status,
        to_status,
        from_member,
        to_member,
        body,
    ) {
        Ok(payload) => payload,
        Err(error) => {
            add_handoff_skip_log(app, &task.id, &format!("IPC 交接消息 payload 无效: {error}"));
            return None;
        }
    };
    let request = vw_gateway_client::vw_api_types::staff::StaffIpcMessageRequest {
        to_agent: to_agent.to_string(),
        payload,
        task_id: Some(task.id.clone()),
        from_member_id: Some(from_member.id.clone()),
        to_member_id: Some(to_member.id.clone()),
    };
    if let Err(error) = request.validate() {
        add_handoff_skip_log(app, &task.id, &format!("IPC 交接消息请求无效: {error}"));
        return None;
    }

    Some(schedule_staff_handoff_ipc_delivery(
        task.id.clone(),
        from_member.id.clone(),
        to_member.id.clone(),
        to_agent.to_string(),
        timestamp_ms,
        request,
    ))
}

fn build_staff_handoff_ipc_payload(
    task: &Task,
    from_status: TaskStatus,
    to_status: TaskStatus,
    from_member: &TaskBoardStaffMember,
    to_member: &TaskBoardStaffMember,
    body: &str,
) -> Result<String, String> {
    let payload = serde_json::json!({
        "version": 1,
        "kind": "staff_handoff",
        "task_id": task.id.as_str(),
        "from_member_id": from_member.id.as_str(),
        "to_member_id": to_member.id.as_str(),
        "from_status": from_status.to_string_key(),
        "to_status": to_status.to_string_key(),
        "body": body,
    });
    let payload = serde_json::to_string(&payload).map_err(|error| error.to_string())?;
    if payload.chars().count()
        > vw_gateway_client::vw_api_types::staff::STAFF_IPC_MESSAGE_PAYLOAD_LIMIT
    {
        return Err(format!(
            "payload must be at most {} characters",
            vw_gateway_client::vw_api_types::staff::STAFF_IPC_MESSAGE_PAYLOAD_LIMIT
        ));
    }
    Ok(payload)
}

fn schedule_staff_handoff_ipc_delivery(
    task_id: String,
    from_member_id: String,
    to_member_id: String,
    to_agent: String,
    timestamp_ms: u64,
    request: vw_gateway_client::vw_api_types::staff::StaffIpcMessageRequest,
) -> iced::Task<crate::app::Message> {
    let message_task_id = task_id.clone();
    let message_from_member_id = from_member_id.clone();
    let message_to_member_id = to_member_id.clone();
    let message_to_agent = to_agent.clone();
    iced::Task::perform(
        async move {
            let client = crate::app::config::gateway_client()?;
            let response = client.staff_ipc_message(&request).await?;
            if response.accepted {
                Ok(())
            } else {
                Err("gateway rejected staff IPC handoff message".to_string())
            }
        },
        move |result| {
            Message::TaskBoard(TaskBoardMessage::StaffHandoffIpcDeliveryCompleted {
                task_id: message_task_id.clone(),
                from_member_id: message_from_member_id.clone(),
                to_member_id: message_to_member_id.clone(),
                to_agent: message_to_agent.clone(),
                timestamp_ms,
                result,
            })
        },
    )
}

fn staff_handoff_message_mut<'a>(
    app: &'a mut crate::app::App,
    task_id: &str,
    from_member_id: &str,
    to_member_id: &str,
    timestamp_ms: u64,
) -> Option<&'a mut crate::app::task::staff::TaskBoardStaffMessageEvent> {
    app.task_board_staff_events.iter_mut().rev().find_map(|event| {
        let message = event.staff_message.as_mut()?;
        (message.task_id == task_id
            && message.from_member_id == from_member_id
            && message.to_member_id == to_member_id
            && message.timestamp_ms == timestamp_ms)
            .then_some(message)
    })
}

fn truncate_handoff_body(body: String) -> String {
    if body.chars().count() <= STAFF_HANDOFF_BODY_MAX_CHARS {
        return body;
    }

    let mut truncated = body.chars().take(STAFF_HANDOFF_BODY_MAX_CHARS).collect::<String>();
    while !truncated.is_char_boundary(truncated.len()) {
        truncated.pop();
    }
    truncated
}

#[cfg(test)]
#[path = "staff_messages_tests.rs"]
mod staff_messages_tests;

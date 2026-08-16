//! 员工任务流本地交接消息。

use super::*;

const STAFF_HANDOFF_BODY_MAX_CHARS: usize = 240;

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
    let event = crate::app::task::staff::TaskBoardStaffEvent::staff_message(
        task.id.clone(),
        from_member.id.clone(),
        to_member.id.clone(),
        body,
        timestamp_ms,
        None,
    );
    record_staff_event(app, event);
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

//! 员工模式成员名册的测试，覆盖空分组、负载警告和成员筛选消息。
//! fixture 固定成员负载与状态数据，验证筛选目标携带稳定命令标识。
use iced::Length;

use crate::app::Message;
use crate::app::message::TaskBoardMessage;
use crate::app::task::staff::{
    TaskBoardStaffCommand, TaskBoardStaffMemberLoad, TaskBoardStaffProjection, TaskBoardStaffRole,
    TaskBoardStaffStatus,
};

use super::*;

fn member_load(
    member_id: &str,
    role: TaskBoardStaffRole,
    enabled: bool,
) -> TaskBoardStaffMemberLoad {
    TaskBoardStaffMemberLoad {
        member_id: member_id.to_string(),
        member_name: format!("Member {member_id}"),
        role,
        status: TaskBoardStaffStatus::Online,
        enabled,
        capacity: 2,
        executor_backend: None,
        delegate_agent: None,
        acp_agent: None,
        task_ids: Vec::new(),
        running_task_ids: Vec::new(),
        blocked_task_ids: Vec::new(),
        load_count: 0,
        over_capacity: false,
    }
}

#[test]
fn roster_panel_builds_empty_and_warning_member_groups() {
    let empty = TaskBoardStaffProjection::default();
    let empty_panel = build_roster_panel(&empty, None, Length::Fixed(286.0), Length::Fill);
    assert_eq!(empty_panel.as_widget().size().width, Length::Fixed(286.0));
    std::hint::black_box(empty_panel);

    let mut overloaded = member_load("executor", TaskBoardStaffRole::Executor, true);
    overloaded.load_count = 3;
    overloaded.over_capacity = true;
    overloaded.task_ids = vec!["T-1".to_string(), "T-2".to_string()];
    overloaded.acp_agent = Some("acp-executor".to_string());

    let mut blocked = member_load("reviewer", TaskBoardStaffRole::Reviewer, false);
    blocked.blocked_task_ids = vec!["T-3".to_string()];
    blocked.delegate_agent = Some("delegate-reviewer".to_string());

    let projection = TaskBoardStaffProjection {
        member_loads: vec![overloaded, blocked],
        blocked_task_count: 1,
        ..TaskBoardStaffProjection::default()
    };
    let panel = build_roster_panel(
        &projection,
        Some("executor"),
        Length::Fixed(320.0),
        Length::Fixed(480.0),
    );

    assert_eq!(panel.as_widget().size().width, Length::Fixed(320.0));
    std::hint::black_box(panel);
}

#[test]
fn filter_message_targets_member_with_stable_command_id() {
    let member = member_load("reviewer", TaskBoardStaffRole::Reviewer, true);

    let Message::TaskBoard(TaskBoardMessage::SubmitStaffCommand(
        TaskBoardStaffCommand::FilterByMember { command_id, member_id },
    )) = filter_message(&member, 42)
    else {
        panic!("expected staff member filter command");
    };

    assert_eq!(command_id, "staff-filter-reviewer-42");
    assert_eq!(member_id, "reviewer");
}

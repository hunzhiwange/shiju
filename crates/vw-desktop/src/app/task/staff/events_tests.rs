use super::{
    TaskBoardStaffCommand, TaskBoardStaffCommandGroup, TaskBoardStaffCommandQueue,
    TaskBoardStaffCommandStatus, TaskBoardStaffEvent, TaskBoardStaffEventKind, TaskBoardStaffLevel,
    TaskBoardStaffMessageDeliveryStatus, TaskBoardStaffOrchestrationContext,
};
use crate::app::task::TaskStatus;
use crate::app::task::staff::models::{TaskBoardStaffMember, TaskBoardStaffRole};

fn start_command(command_id: &str, task_id: &str, member_id: &str) -> TaskBoardStaffCommand {
    TaskBoardStaffCommand::StartTask {
        command_id: command_id.to_string(),
        task_id: task_id.to_string(),
        member_id: member_id.to_string(),
    }
}

fn assign_command(command_id: &str, task_id: &str, member_id: &str) -> TaskBoardStaffCommand {
    TaskBoardStaffCommand::AssignTask {
        command_id: command_id.to_string(),
        task_id: task_id.to_string(),
        member_id: member_id.to_string(),
    }
}

fn pause_command(command_id: &str, task_id: &str, member_id: &str) -> TaskBoardStaffCommand {
    TaskBoardStaffCommand::PauseTask {
        command_id: command_id.to_string(),
        task_id: task_id.to_string(),
        member_id: member_id.to_string(),
    }
}

#[test]
fn command_queue_serializes_by_member_and_deduplicates() {
    let mut queue = TaskBoardStaffCommandQueue::default();
    let first = start_command("cmd-1", "task-1", "staff-executor");
    let second = start_command("cmd-2", "task-2", "staff-executor");
    let other_member = start_command("cmd-3", "task-3", "staff-reviewer");

    assert_eq!(queue.enqueue(first.clone(), 100).status, TaskBoardStaffCommandStatus::Pending);
    assert_eq!(queue.enqueue(second.clone(), 101).status, TaskBoardStaffCommandStatus::Pending);
    assert_eq!(
        queue.enqueue(other_member.clone(), 102).status,
        TaskBoardStaffCommandStatus::Pending
    );
    assert_eq!(queue.enqueue(first.clone(), 103).status, TaskBoardStaffCommandStatus::Skipped);
    assert_eq!(queue.pending_len(), 3);

    let (claimed_first, first_event) = queue.claim_next(110).expect("first command");
    assert_eq!(claimed_first, first);
    assert_eq!(first_event.status, TaskBoardStaffCommandStatus::Inflight);

    let (claimed_other, other_event) = queue.claim_next(111).expect("other member command");
    assert_eq!(claimed_other, other_member);
    assert_eq!(other_event.status, TaskBoardStaffCommandStatus::Inflight);
    assert!(queue.claim_next(112).is_none());

    let applied = queue.mark_applied("cmd-1", 120, "first applied").expect("applied event");
    assert_eq!(applied.status, TaskBoardStaffCommandStatus::Applied);

    let (claimed_second, second_event) = queue.claim_next(121).expect("second command");
    assert_eq!(claimed_second, second);
    assert_eq!(second_event.status, TaskBoardStaffCommandStatus::Inflight);
    assert_eq!(queue.status_for("cmd-1"), Some(TaskBoardStaffCommandStatus::Applied));
}

#[test]
fn command_queue_rejects_blank_command_id_without_enqueuing() {
    let mut queue = TaskBoardStaffCommandQueue::default();
    let event = queue.enqueue(start_command("   ", "task-1", "staff-executor"), 10);

    assert_eq!(event.status, TaskBoardStaffCommandStatus::Skipped);
    assert_eq!(event.command_id.trim(), "");
    assert!(event.message.contains("缺少 command id"));
    assert_eq!(queue.pending_len(), 0);
    assert!(!queue.has_claimable_pending());
}

#[test]
fn command_queue_claims_global_commands_by_command_id_order() {
    let mut queue = TaskBoardStaffCommandQueue::default();
    let later = TaskBoardStaffCommand::OpenTaskLogs {
        command_id: "cmd-b".to_string(),
        task_id: "task-b".to_string(),
    };
    let earlier = TaskBoardStaffCommand::OpenTaskLogs {
        command_id: "cmd-a".to_string(),
        task_id: "task-a".to_string(),
    };

    queue.enqueue(later, 20);
    queue.enqueue(earlier.clone(), 21);

    assert!(queue.has_claimable_pending());
    let (claimed, event) = queue.claim_next(22).expect("global command");

    assert_eq!(claimed, earlier);
    assert_eq!(event.command_id, "cmd-a");
    assert_eq!(event.status, TaskBoardStaffCommandStatus::Inflight);
}

#[test]
fn command_queue_skips_active_duplicate_task_actions() {
    let mut queue = TaskBoardStaffCommandQueue::default();
    let first = start_command("cmd-1", "task-1", "staff-executor");
    let duplicate = start_command("cmd-2", "task-1", "staff-executor");

    assert_eq!(queue.enqueue(first.clone(), 100).status, TaskBoardStaffCommandStatus::Pending);
    assert!(queue.has_active_task_command("task-1", TaskBoardStaffCommandGroup::Start));
    assert_eq!(queue.enqueue(duplicate.clone(), 101).status, TaskBoardStaffCommandStatus::Skipped);
    assert_eq!(queue.pending_len(), 1);

    let (claimed, _) = queue.claim_next(110).expect("claimed command");
    assert_eq!(claimed, first);
    assert_eq!(queue.enqueue(duplicate.clone(), 111).status, TaskBoardStaffCommandStatus::Skipped);

    queue.mark_applied("cmd-1", 120, "first applied").expect("applied event");
    assert!(!queue.has_active_task_command("task-1", TaskBoardStaffCommandGroup::Start));
    assert_eq!(queue.enqueue(duplicate, 121).status, TaskBoardStaffCommandStatus::Pending);
}

#[test]
fn command_queue_uses_assignment_and_pause_resume_duplicate_groups() {
    let mut queue = TaskBoardStaffCommandQueue::default();

    assert_eq!(
        queue.enqueue(assign_command("assign-1", "task-1", "staff-executor"), 100).status,
        TaskBoardStaffCommandStatus::Pending
    );
    assert!(queue.has_active_task_command("task-1", TaskBoardStaffCommandGroup::Assignment));
    assert_eq!(
        queue
            .enqueue(
                TaskBoardStaffCommand::TakeOverTask {
                    command_id: "takeover-1".to_string(),
                    task_id: "task-1".to_string(),
                    member_id: "staff-reviewer".to_string(),
                },
                101,
            )
            .status,
        TaskBoardStaffCommandStatus::Skipped
    );
    assert!(!queue.has_active_task_command("", TaskBoardStaffCommandGroup::Assignment));

    let mut pause_resume_queue = TaskBoardStaffCommandQueue::default();
    assert_eq!(
        pause_resume_queue
            .enqueue(pause_command("pause-1", "task-1", "staff-executor"), 200)
            .status,
        TaskBoardStaffCommandStatus::Pending
    );
    assert_eq!(
        pause_resume_queue
            .enqueue(
                TaskBoardStaffCommand::ResumeTask {
                    command_id: "resume-1".to_string(),
                    task_id: "task-1".to_string(),
                    member_id: "staff-executor".to_string(),
                },
                201,
            )
            .status,
        TaskBoardStaffCommandStatus::Skipped
    );
}

#[test]
fn command_queue_ignores_finish_for_unknown_or_non_inflight_commands() {
    let mut queue = TaskBoardStaffCommandQueue::default();
    queue.enqueue(start_command("cmd-1", "task-1", "staff-executor"), 100);

    assert!(queue.mark_applied("missing", 101, "missing").is_none());
    assert!(queue.mark_failed("cmd-1", 102, "not inflight yet").is_none());
    assert_eq!(queue.status_for("cmd-1"), Some(TaskBoardStaffCommandStatus::Pending));

    let _ = queue.claim_next(103).expect("inflight");
    assert!(queue.mark_failed("cmd-1", 104, "failed").expect("failed event").status.is_failed());
    assert!(queue.mark_applied("cmd-1", 105, "already failed").is_none());
    assert!(!queue.has_active_task_command("task-1", TaskBoardStaffCommandGroup::Start));
}

#[test]
fn staff_events_preserve_orchestration_context() {
    let mut queue = TaskBoardStaffCommandQueue::default();
    let command = start_command("cmd-orchestrated", "task-design", "staff-planner");
    let context = TaskBoardStaffOrchestrationContext {
        stage_status: TaskStatus::Planning,
        stage_label: "规划".to_string(),
        member_level: TaskBoardStaffLevel::Manager,
        member_level_label: "管理智能体".to_string(),
        member_name: "Planner".to_string(),
        action_source: "自主调度".to_string(),
        target: "task-design -> Planner".to_string(),
        manager_member_names: vec!["Lead".to_string()],
    };

    queue.remember_orchestration_context(command.command_id(), context.clone());

    let pending = queue.enqueue(command.clone(), 200);
    assert_eq!(pending.orchestration, Some(context.clone()));

    let (_, inflight) = queue.claim_next(210).expect("inflight command");
    assert_eq!(inflight.orchestration, Some(context.clone()));

    let failed =
        queue.mark_failed("cmd-orchestrated", 220, "任务阻塞：等待设计确认").expect("failed event");
    assert_eq!(failed.status, TaskBoardStaffCommandStatus::Failed);
    assert_eq!(failed.orchestration, Some(context));
    assert!(failed.message.contains("等待设计确认"));
}

#[test]
fn orchestration_context_new_derives_labels_from_member_level_and_stage() {
    let cases = [
        (TaskBoardStaffRole::Orchestrator, TaskBoardStaffLevel::Principal, "主智能体"),
        (TaskBoardStaffRole::Planner, TaskBoardStaffLevel::Manager, "管理智能体"),
        (TaskBoardStaffRole::Executor, TaskBoardStaffLevel::Executor, "执行智能体"),
        (TaskBoardStaffRole::Observer, TaskBoardStaffLevel::Member, "普通智能体"),
    ];

    for (role, level, label) in cases {
        let member = TaskBoardStaffMember::new(role.key(), "Worker", role);
        let context = TaskBoardStaffOrchestrationContext::new(
            TaskStatus::Planning,
            &member,
            "scheduler",
            "task -> worker",
            vec!["Lead".to_string()],
        );

        assert_eq!(context.stage_label, TaskStatus::Planning.label());
        assert_eq!(context.member_level, level);
        assert_eq!(context.member_level_label, label);
        assert_eq!(context.member_name, "Worker");
        assert_eq!(context.action_source, "scheduler");
        assert_eq!(context.target, "task -> worker");
        assert_eq!(context.manager_member_names, vec!["Lead"]);
    }
}

#[test]
fn staff_event_stream_accepts_message_events() {
    let event = TaskBoardStaffEvent::staff_message(
        "T-handoff",
        "staff-planner",
        "staff-executor",
        "规划已完成，请接手执行。",
        300,
        Some(TaskBoardStaffMessageDeliveryStatus::Pending),
    );

    assert_eq!(event.kind, TaskBoardStaffEventKind::Message);
    assert_eq!(event.command_status(), None);
    assert_eq!(event.status.label(), "消息");
    assert_eq!(event.task_id.as_deref(), Some("T-handoff"));
    assert_eq!(event.member_id.as_deref(), Some("staff-planner"));
    assert_eq!(event.message, "规划已完成，请接手执行。");

    let message = event.staff_message.as_ref().expect("message payload");
    assert_eq!(message.task_id, "T-handoff");
    assert_eq!(message.from_member_id, "staff-planner");
    assert_eq!(message.to_member_id, "staff-executor");
    assert_eq!(message.body, "规划已完成，请接手执行。");
    assert_eq!(message.timestamp_ms, 300);
    assert_eq!(message.delivery_status, Some(TaskBoardStaffMessageDeliveryStatus::Pending));
}

#[test]
fn staff_event_statuses_expose_labels_failures_and_command_equivalence() {
    let commands = [
        (TaskBoardStaffCommandStatus::Pending, "待处理"),
        (TaskBoardStaffCommandStatus::Inflight, "处理中"),
        (TaskBoardStaffCommandStatus::Applied, "已应用"),
        (TaskBoardStaffCommandStatus::Failed, "失败"),
        (TaskBoardStaffCommandStatus::Skipped, "已跳过"),
    ];
    for (status, label) in commands {
        let event_status = super::TaskBoardStaffEventStatus::from(status);
        assert_eq!(status.label(), label);
        assert_eq!(event_status.label(), label);
        assert_eq!(event_status.command_status(), Some(status));
        assert_eq!(event_status, status);
        assert_eq!(status, event_status);
        assert_eq!(event_status.is_failed(), status == TaskBoardStaffCommandStatus::Failed);
    }

    assert_eq!(TaskBoardStaffMessageDeliveryStatus::Pending.label(), "待投递");
    assert_eq!(TaskBoardStaffMessageDeliveryStatus::Delivered.label(), "已送达");
    assert_eq!(TaskBoardStaffMessageDeliveryStatus::Failed.label(), "投递失败");
}
